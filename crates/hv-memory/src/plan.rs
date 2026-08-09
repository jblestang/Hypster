//! Memory allocation planner.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use hv_config_model::intent::StaticIntentIR;
use hv_types::arithmetic::{align_up_u64, checked_add_u64, is_aligned_u64, ranges_overlap_u64};
use hv_types::HostPhysAddr;

use crate::error::MemoryPlanError;
use crate::map::{ConventionalRegion, total_conventional_bytes};
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

/// Builds a deterministic memory plan from conventional regions and intent.
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
            purpose: MemoryPurpose::PartitionGuestRam {
                vm_id: partition.vm_id.raw(),
            },
            size: partition.memory_bytes,
        });
    }
    for channel in intent.ipc.iter() {
        pending.push(PendingAllocation {
            purpose: MemoryPurpose::IpcChannel {
                name: channel.name.clone(),
            },
            size: channel.shared_bytes,
        });
    }

    let required: u64 = pending
        .iter()
        .try_fold(0u64, |acc, item| checked_add_u64(acc, item.size))
        .map_err(|_| MemoryPlanError::Overflow)?;
    let available = total_conventional_bytes(conventional).ok_or(MemoryPlanError::Overflow)?;
    if required > available {
        return Err(MemoryPlanError::OutOfMemory { required, available });
    }

    let mut cursor = select_allocation_base(conventional, required)?;
    let mut regions = Vec::with_capacity(pending.len());
    for item in pending {
        cursor = HostPhysAddr::new(
            align_up_u64(cursor.raw(), PLANNER_PAGE_SIZE).map_err(|_| MemoryPlanError::Overflow)?,
        );
        let end = checked_add_u64(cursor.raw(), item.size).map_err(|_| MemoryPlanError::Overflow)?;
        regions.push(MemoryRegionPlan {
            purpose: item.purpose,
            base: cursor,
            size: item.size,
        });
        cursor = HostPhysAddr::new(end);
    }

    validate_plan(&regions)?;
    Ok(MemoryPlan {
        allocated_bytes: required,
        regions,
    })
}

fn select_allocation_base(
    conventional: &[ConventionalRegion],
    required: u64,
) -> Result<HostPhysAddr, MemoryPlanError> {
    let mut best: Option<ConventionalRegion> = None;
    for region in conventional {
        if region.size < required {
            continue;
        }
        best = Some(match best {
            Some(current) if current.base.raw() > region.base.raw() => current,
            _ => *region,
        });
    }
    let region = best.ok_or(MemoryPlanError::NoConventionalMemory)?;
    let aligned = align_up_u64(region.base.raw(), PLANNER_PAGE_SIZE)
        .map_err(|_| MemoryPlanError::Overflow)?;
    let end = checked_add_u64(aligned, required).map_err(|_| MemoryPlanError::Overflow)?;
    if end > checked_add_u64(region.base.raw(), region.size).map_err(|_| MemoryPlanError::Overflow)? {
        return Err(MemoryPlanError::OutOfMemory {
            required,
            available: region.size,
        });
    }
    Ok(HostPhysAddr::new(aligned))
}

fn validate_plan(regions: &[MemoryRegionPlan]) -> Result<(), MemoryPlanError> {
    for region in regions {
        if !is_aligned_u64(region.base.raw(), PLANNER_PAGE_SIZE).map_err(|_| MemoryPlanError::Overflow)? {
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
    }
}
