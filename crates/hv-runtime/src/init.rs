//! Gate C runtime initialization sequence.

use hv_acpi::rsdp::RSDP_V2_LEN;
use hv_acpi::parse_rsdp;
use hv_boot_abi::{layout, BootInfo};
use hv_core::boot::BootPhase;
use hv_cpu::probe_cpu;
use hv_ept::install_ept_mappings;
use hv_types::HostPhysAddr;
use hv_vtd::install_vtd_domains;
#[cfg(all(feature = "hardware", target_arch = "x86_64"))]
use hv_vmx::{VmxHostInit, VmxonRegion, VMXON_REGION_SIZE};

use crate::error::RuntimeError;

/// Gate C MVP DRHD register base (QEMU q35).
pub const DEFAULT_DRHD_BASE: u64 = 0xFED9_0000;
/// Gate C MVP reserved size for EPT table buffers (16 MiB).
pub const EPT_TABLE_REGION_BYTES: usize = 16 * 1024 * 1024;
/// Gate C MVP reserved size for VT-d table buffers (16 MiB).
pub const VT_D_TABLE_REGION_BYTES: usize = 16 * 1024 * 1024;

/// Static plans and table bases passed from hypervisor image data.
#[derive(Clone, Debug)]
pub struct GateCPlans {
    /// Resolved EPT plan.
    pub ept: hv_ept::EptPlan,
    /// Resolved VT-d plan.
    pub vtd: hv_vtd::VtdPlan,
    /// Host physical base for EPT page tables.
    pub ept_table_base: HostPhysAddr,
    /// Host physical base for VT-d structures.
    pub vtd_table_base: HostPhysAddr,
    /// Host physical base for the VMXON region.
    pub vmxon_region_base: HostPhysAddr,
}

/// Summary returned after successful runtime initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeInitReport {
    /// Final boot phase reached.
    pub phase: BootPhase,
    /// True when VMXON succeeded.
    pub vmx_enabled: bool,
    /// Number of EPT roots installed.
    pub ept_roots: usize,
    /// Number of VT-d domain tables installed.
    pub vtd_domains: usize,
}

/// Initializes the hypervisor runtime from loader-provided boot info.
///
/// # Errors
///
/// Returns [`RuntimeError`] when validation or table installation fails.
pub fn initialize(boot_info: &BootInfo, plans: &GateCPlans) -> Result<RuntimeInitReport, RuntimeError> {
    let total_bytes = boot_info.boot_info_bytes as usize;
    // SAFETY: loader guarantees `boot_info` points at `total_bytes` valid bytes.
    unsafe {
        layout::validate_boot_info_layout(boot_info, total_bytes)?;
    }

    let mut phase = BootPhase::BootServicesExited;
    phase = advance_phase(phase, BootPhase::MemoryReady)?;

    let _memory_map = memory_descriptors(boot_info);

    probe_cpu()?;
    phase = advance_phase(phase, BootPhase::CpuReady)?;

    verify_rsdp(boot_info)?;

    let ept_roots = install_ept_tables(plans)?;
    let vtd_domains = install_vtd_tables(plans)?;

    phase = advance_phase(phase, BootPhase::VmxReady)?;
    let vmx_enabled = enable_vmx(plans)?;

    phase = advance_phase(phase, BootPhase::IommuReady)?;
    phase = advance_phase(phase, BootPhase::InterruptsReady)?;
    phase = advance_phase(phase, BootPhase::PartitionsPrepared)?;
    phase = advance_phase(phase, BootPhase::Running)?;

    Ok(RuntimeInitReport {
        phase,
        vmx_enabled,
        ept_roots,
        vtd_domains,
    })
}

fn advance_phase(current: BootPhase, next: BootPhase) -> Result<BootPhase, RuntimeError> {
    current
        .transition(next)
        .map_err(|_| RuntimeError::TableRegionUnavailable)
}

fn memory_descriptors(boot_info: &BootInfo) -> alloc::vec::Vec<(u32, HostPhysAddr, u64)> {
    let map = unsafe { layout::memory_map_slice(boot_info) };
    map.iter()
        .map(|desc| {
            (
                desc.typ,
                HostPhysAddr::new(desc.physical_start),
                desc.number_of_bytes,
            )
        })
        .collect()
}

fn verify_rsdp(boot_info: &BootInfo) -> Result<(), RuntimeError> {
    let rsdp_addr = boot_info.acpi.rsdp_address;
    if rsdp_addr == 0 {
        return Ok(());
    }
    let mut buf = [0u8; RSDP_V2_LEN];
    // SAFETY: firmware mapped the RSDP for the loader; Gate C uses identity map.
    unsafe {
        hv_x86::read_phys_bytes(HostPhysAddr::new(rsdp_addr), &mut buf)?;
    }
    parse_rsdp(&buf)?;
    Ok(())
}

fn install_ept_tables(plans: &GateCPlans) -> Result<usize, RuntimeError> {
    let base = plans.ept_table_base.raw();
    if base == 0 {
        return Err(RuntimeError::TableRegionUnavailable);
    }
    let table_ptr = base as *mut u8;
    let table_bytes = unsafe { core::slice::from_raw_parts_mut(table_ptr, EPT_TABLE_REGION_BYTES) };

    let mut offset = 0usize;
    for partition in &plans.ept.partitions {
        if partition.mappings.is_empty() {
            continue;
        }
        if offset >= table_bytes.len() {
            return Err(RuntimeError::Ept(hv_ept::EptPlanError::BufferTooSmall));
        }
        let sub = &mut table_bytes[offset..];
        let sub_base = HostPhysAddr::new(base + offset as u64);
        let result = install_ept_mappings(sub, sub_base, &partition.mappings)?;
        offset += result.bytes_used;
    }
    Ok(plans.ept.partitions.len())
}

fn install_vtd_tables(plans: &GateCPlans) -> Result<usize, RuntimeError> {
    if plans.vtd.domains.is_empty() {
        return Ok(0);
    }
    let base = plans.vtd_table_base.raw();
    if base == 0 {
        return Err(RuntimeError::TableRegionUnavailable);
    }
    let table_ptr = base as *mut u8;
    let table_bytes = unsafe { core::slice::from_raw_parts_mut(table_ptr, VT_D_TABLE_REGION_BYTES) };
    let drhd = HostPhysAddr::new(DEFAULT_DRHD_BASE);
    install_vtd_domains(
        table_bytes,
        plans.vtd_table_base,
        &plans.vtd.domains,
        drhd,
    )?;
    Ok(plans.vtd.domains.len())
}

fn enable_vmx(plans: &GateCPlans) -> Result<bool, RuntimeError> {
    #[cfg(all(feature = "hardware", target_arch = "x86_64"))]
    {
        let mut init = VmxHostInit::from_hardware()?;
        let revision = init.capabilities().revision_id;
        let region = VmxonRegion::new(revision);
        let region_ptr = plans.vmxon_region_base.raw() as *mut u8;
        if region_ptr.is_null() {
            return Err(RuntimeError::TableRegionUnavailable);
        }
        let region_bytes = unsafe { core::slice::from_raw_parts_mut(region_ptr, VMXON_REGION_SIZE) };
        region_bytes.copy_from_slice(region.as_bytes());
        // SAFETY: VMXON region is initialized and loader-reserved.
        unsafe {
            init.try_enable_vmx(plans.vmxon_region_base)?;
        }
        return Ok(true);
    }
    #[cfg(not(all(feature = "hardware", target_arch = "x86_64")))]
    {
        let _ = plans;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use hv_boot_abi::{
        BootAcpiInfo, BootInfoHeader, BOOT_ABI_VERSION_MAJOR, BOOT_ABI_VERSION_MINOR,
        BOOT_INFO_MAGIC,
    };
    use hv_ept::EptPlan;

    #[test]
    fn initialize_requires_valid_layout() {
        let info = BootInfo {
            header: BootInfoHeader {
                magic: BOOT_INFO_MAGIC,
                version_major: BOOT_ABI_VERSION_MAJOR,
                version_minor: BOOT_ABI_VERSION_MINOR,
                total_size: 0,
                config_hash: [0; 32],
            },
            acpi: BootAcpiInfo { rsdp_address: 0 },
            memory_map_entry_count: 0,
            flags: 0,
            hypervisor_load_address: 0,
            boot_info_bytes: 0,
        };
        let plans = GateCPlans {
            ept: EptPlan {
                partitions: alloc::vec::Vec::new(),
            },
            vtd: hv_vtd::VtdPlan {
                domains: alloc::vec::Vec::new(),
            },
            ept_table_base: HostPhysAddr::new(0x2000_0000),
            vtd_table_base: HostPhysAddr::new(0x2100_0000),
            vmxon_region_base: HostPhysAddr::new(0x2200_0000),
        };
        assert!(initialize(&info, &plans).is_err());
    }
}
