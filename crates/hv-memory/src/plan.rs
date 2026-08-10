//! Memory allocation planner.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use hv_config_model::intent::StaticIntentIR;
use hv_types::arithmetic::{align_up_u64, checked_add_u64, is_aligned_u64, ranges_overlap_u64};
use hv_types::HostPhysAddr;

use crate::error::MemoryPlanError;
use crate::map::{total_conventional_bytes, ConventionalRegion};
use crate::{HYPERVISOR_RESERVE_BYTES, PLANNER_PAGE_SIZE};

/// Purpose label for a planned host memory region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryPurpose {
    /// Hypervisor image and private data.
    Hypervisor,
    /// Hypervisor page tables.
    HypervisorPageTables,
    /// Partition private guest RAM backing.
    PartitionGuestRam {
        /// VM identifier.
        vm_id: u32,
    },
    /// IPC shared ring memory.
    IpcChannel {
        /// Channel name.
        name: String,
    },
    /// EPT page-table buffer.
    EptTables,
    /// VT-d page-table buffer.
    VtdTables,
    /// VMXON region.
    VmxonRegion,
    /// VMCS regions (one page per partition).
    VmcsRegions,
    /// Emulated MMIO backing pages.
    MmioEmulation,
}

/// One planned host memory region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryRegionPlan {
    /// Human-readable purpose.
    pub purpose: MemoryPurpose,
    /// Host physical base.
    pub base: HostPhysAddr,
    /// Region size in bytes.
    pub size: u64,
}

/// Full memory plan for the platform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryPlan {
    /// Planned regions sorted by base address.
    pub regions: Vec<MemoryRegionPlan>,
    /// Total bytes allocated by the plan.
    pub allocated_bytes: u64,
}

struct PendingAllocation {
    purpose: MemoryPurpose,
    size: u64,
}

/// EPT table buffer size reserved in the host memory plan.
pub const EPT_TABLE_RESERVE_BYTES: u64 = 16 * 1024 * 1024;
/// VT-d table buffer size reserved in the host memory plan.
pub const VTD_TABLE_RESERVE_BYTES: u64 = 16 * 1024 * 1024;
/// VMXON region size.
pub const VMXON_RESERVE_BYTES: u64 = 4096;
/// VMCS region pool size (enough for several partitions).
pub const VMCS_RESERVE_BYTES: u64 = 64 * 1024;
/// Emulated MMIO backing pool size.
pub const MMIO_RESERVE_BYTES: u64 = 16 * 1024 * 1024;

/// Builds a deterministic memory plan from conventional regions and intent.
///
/// Allocations are packed in purpose order across conventional regions without
/// crossing region boundaries, so PCI holes between low and high RAM are skipped.
pub fn plan_memory(
    intent: &StaticIntentIR,
    conventional: &[ConventionalRegion],
) -> Result<MemoryPlan, MemoryPlanError> {
    if conventional.is_empty() {
        return Err(MemoryPlanError::NoConventionalMemory);
    }

    let mut pending = Vec::new();
    pending.push(PendingAllocation {
        purpose: MemoryPurpose::Hypervisor,
        size: HYPERVISOR_RESERVE_BYTES,
    });
    pending.push(PendingAllocation {
        purpose: MemoryPurpose::HypervisorPageTables,
        size: 16 * 1024 * 1024,
    });
    for partition in intent.partitions.iter() {
        pending.push(PendingAllocation {
            purpose: MemoryPurpose::PartitionGuestRam { vm_id: partition.vm_id.raw() },
            size: partition.memory_bytes,
        });
    }
    for channel in intent.ipc.iter() {
        let size = align_up_u64(channel.shared_bytes, PLANNER_PAGE_SIZE)
            .map_err(|_| MemoryPlanError::Overflow)?;
        pending.push(PendingAllocation {
            purpose: MemoryPurpose::IpcChannel { name: channel.name.clone() },
            size,
        });
    }
    pending.push(PendingAllocation {
        purpose: MemoryPurpose::EptTables,
        size: EPT_TABLE_RESERVE_BYTES,
    });
    pending.push(PendingAllocation {
        purpose: MemoryPurpose::VtdTables,
        size: VTD_TABLE_RESERVE_BYTES,
    });
    pending.push(PendingAllocation {
        purpose: MemoryPurpose::VmxonRegion,
        size: VMXON_RESERVE_BYTES,
    });
    pending.push(PendingAllocation {
        purpose: MemoryPurpose::VmcsRegions,
        size: VMCS_RESERVE_BYTES,
    });
    pending.push(PendingAllocation {
        purpose: MemoryPurpose::MmioEmulation,
        size: MMIO_RESERVE_BYTES,
    });

    let required: u64 = pending
        .iter()
        .try_fold(0u64, |acc, item| checked_add_u64(acc, item.size))
        .map_err(|_| MemoryPlanError::Overflow)?;
    let available = total_conventional_bytes(conventional).ok_or(MemoryPlanError::Overflow)?;
    if required > available {
        return Err(MemoryPlanError::OutOfMemory { required, available });
    }

    let mut regions = Vec::with_capacity(pending.len());
    let mut region_idx = 0usize;
    let mut cursor = aligned_region_start(&conventional[0])?;
    let mut region_end = region_exclusive_end(&conventional[0])?;

    for item in pending {
        let size = align_up_u64(item.size, PLANNER_PAGE_SIZE).map_err(|_| MemoryPlanError::Overflow)?;
        loop {
            cursor =
                HostPhysAddr::new(align_up_u64(cursor.raw(), PLANNER_PAGE_SIZE).map_err(|_| {
                    MemoryPlanError::Overflow
                })?);
            let end = checked_add_u64(cursor.raw(), size).map_err(|_| MemoryPlanError::Overflow)?;
            if end <= region_end.raw() {
                regions.push(MemoryRegionPlan {
                    purpose: item.purpose,
                    base: cursor,
                    size,
                });
                cursor = HostPhysAddr::new(end);
                break;
            }
            region_idx = region_idx.checked_add(1).ok_or(MemoryPlanError::Overflow)?;
            if region_idx >= conventional.len() {
                return Err(MemoryPlanError::OutOfMemory { required, available });
            }
            cursor = aligned_region_start(&conventional[region_idx])?;
            region_end = region_exclusive_end(&conventional[region_idx])?;
        }
    }

    validate_plan(&regions)?;
    Ok(MemoryPlan { allocated_bytes: required, regions })
}

fn aligned_region_start(region: &ConventionalRegion) -> Result<HostPhysAddr, MemoryPlanError> {
    let aligned =
        align_up_u64(region.base.raw(), PLANNER_PAGE_SIZE).map_err(|_| MemoryPlanError::Overflow)?;
    Ok(HostPhysAddr::new(aligned))
}

fn region_exclusive_end(region: &ConventionalRegion) -> Result<HostPhysAddr, MemoryPlanError> {
    let end =
        checked_add_u64(region.base.raw(), region.size).map_err(|_| MemoryPlanError::Overflow)?;
    Ok(HostPhysAddr::new(end))
}

fn validate_plan(regions: &[MemoryRegionPlan]) -> Result<(), MemoryPlanError> {
    for region in regions {
        if !is_aligned_u64(region.base.raw(), PLANNER_PAGE_SIZE)
            .map_err(|_| MemoryPlanError::Overflow)?
        {
            return Err(MemoryPlanError::Misaligned {
                region: purpose_label(&region.purpose),
                base: region.base,
            });
        }
    }
    for (i, left) in regions.iter().enumerate() {
        for right in regions.iter().skip(i + 1) {
            if ranges_overlap_u64(left.base.raw(), left.size, right.base.raw(), right.size) {
                return Err(MemoryPlanError::Overlap {
                    first: purpose_label(&left.purpose),
                    second: purpose_label(&right.purpose),
                });
            }
        }
    }
    Ok(())
}

fn purpose_label(purpose: &MemoryPurpose) -> String {
    match purpose {
        MemoryPurpose::Hypervisor => String::from("hypervisor"),
        MemoryPurpose::HypervisorPageTables => String::from("hypervisor_page_tables"),
        MemoryPurpose::PartitionGuestRam { vm_id } => format!("partition_guest_ram[{vm_id}]"),
        MemoryPurpose::IpcChannel { name } => format!("ipc[{name}]"),
        MemoryPurpose::EptTables => String::from("ept_tables"),
        MemoryPurpose::VtdTables => String::from("vtd_tables"),
        MemoryPurpose::VmxonRegion => String::from("vmxon_region"),
        MemoryPurpose::VmcsRegions => String::from("vmcs_regions"),
        MemoryPurpose::MmioEmulation => String::from("mmio_emulation"),
    }
}

