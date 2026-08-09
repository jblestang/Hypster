#![allow(missing_docs)]
#![allow(clippy::expect_used)]
#![allow(unsafe_code)]

use hv_boot_abi::{
    layout, BootAcpiInfo, BootInfo, BootInfoHeader, BOOT_ABI_VERSION_MAJOR, BOOT_ABI_VERSION_MINOR,
    BOOT_INFO_MAGIC,
};
use hv_ept::{install_ept_mappings, EptMapping, EptMemoryType, EptPermissions};
use hv_runtime::{initialize, GateCPlans};
use hv_types::{GuestPhysAddr, HostPhysAddr, VmId};
use hv_vtd::{install_vtd_domains, VtdDomainPlan};

#[test]
fn gate_c_boot_layout_validates_empty_map() {
    let total = core::mem::size_of::<BootInfo>();
    let info = BootInfo {
        header: BootInfoHeader {
            magic: BOOT_INFO_MAGIC,
            version_major: BOOT_ABI_VERSION_MAJOR,
            version_minor: BOOT_ABI_VERSION_MINOR,
            total_size: total as u32,
            config_hash: [0; 32],
        },
        acpi: BootAcpiInfo { rsdp_address: 0 },
        memory_map_entry_count: 0,
        flags: 0,
        hypervisor_load_address: 0x100000,
        boot_info_bytes: total as u32,
    };
    let result = unsafe { layout::validate_boot_info_layout(&info, total) };
    assert!(result.is_ok());
}

#[test]
fn gate_c_ept_install_identity_gib() {
    let mut buffer = vec![0u8; 64 * 1024];
    let mapping = EptMapping {
        guest_phys: GuestPhysAddr::new(0),
        host_phys: HostPhysAddr::new(0x4000_0000),
        size: 1024 * 1024 * 1024,
        permissions: EptPermissions::GUEST_RAM,
        memory_type: EptMemoryType::WriteBack,
    };
    let result = install_ept_mappings(&mut buffer, HostPhysAddr::new(0x1000), &[mapping])
        .expect("install ept");
    assert!(result.bytes_used <= buffer.len());
    assert_ne!(result.root_hpa.raw(), 0);
}

#[test]
fn gate_c_vtd_install_three_domains() {
    let mut buffer = vec![0u8; 64 * 1024];
    let domains = [
        VtdDomainPlan {
            domain_id: hv_types::IommuDomainId::new(1),
            vm_id: VmId::new(1),
            devices: vec!["0000:00:03.0".into()],
            mappings: Vec::new(),
        },
        VtdDomainPlan {
            domain_id: hv_types::IommuDomainId::new(2),
            vm_id: VmId::new(2),
            devices: Vec::new(),
            mappings: Vec::new(),
        },
        VtdDomainPlan {
            domain_id: hv_types::IommuDomainId::new(3),
            vm_id: VmId::new(3),
            devices: vec!["0000:00:04.0".into()],
            mappings: Vec::new(),
        },
    ];
    let result = install_vtd_domains(
        &mut buffer,
        HostPhysAddr::new(0x2000),
        &domains,
        HostPhysAddr::new(0xFED9_0000),
    )
    .expect("install vtd");
    assert!(result.bytes_used <= buffer.len());
    assert_ne!(result.root_table_hpa.raw(), 0);
}

#[test]
fn gate_c_runtime_initialize_without_hardware_vmx() {
    let total = core::mem::size_of::<BootInfo>();
    let info = BootInfo {
        header: BootInfoHeader {
            magic: BOOT_INFO_MAGIC,
            version_major: BOOT_ABI_VERSION_MAJOR,
            version_minor: BOOT_ABI_VERSION_MINOR,
            total_size: total as u32,
            config_hash: [0; 32],
        },
        acpi: BootAcpiInfo { rsdp_address: 0 },
        memory_map_entry_count: 0,
        flags: 0,
        hypervisor_load_address: 0x100000,
        boot_info_bytes: total as u32,
    };
    let plans = GateCPlans {
        ept: hv_ept::EptPlan { partitions: Vec::new() },
        vtd: hv_vtd::VtdPlan { domains: Vec::new() },
        ept_table_base: HostPhysAddr::new(0),
        vtd_table_base: HostPhysAddr::new(0),
        vmxon_region_base: HostPhysAddr::new(0),
    };
    // CPU probe may fail on hosts without VMX; that is acceptable for CI.
    match initialize(&info, &plans) {
        Ok(report) => {
            assert!(!report.vmx_enabled);
        }
        Err(err) => {
            assert!(
                matches!(
                    err,
                    hv_runtime::RuntimeError::Cpu(_)
                        | hv_runtime::RuntimeError::TableRegionUnavailable
                ),
                "unexpected error: {}",
                err.as_str()
            );
        }
    }
}
