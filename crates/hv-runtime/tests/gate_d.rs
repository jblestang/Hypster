#![allow(missing_docs)]
#![allow(clippy::expect_used)]
#![allow(unsafe_code)]

use hv_config_model::pipeline::compile_config;
use hv_config_model::yaml::read_yaml_file;
use hv_core::fixture::qemu_validation_observed;
use hv_core::resolve_platform;
use hv_guest_abi::layout;
use hv_guest_abi::GuestBootInfo;
use hv_ipc::{init_ring, try_pop, try_push, validate_ring, IpcError};
use hv_partition::{build_guest_boot_info, gate_d_plans_from_resolved};
use hv_runtime::{initialize_gate_d, GateCPlans};
use hv_types::{HostPhysAddr, VcpuId, VmId};

const IPC_RING_BYTES: usize = 524_328;

#[test]
fn gate_d_ipc_ring_push_pop_across_channels() {
    let mut in_to_mid = vec![0u8; IPC_RING_BYTES];
    init_ring(&mut in_to_mid, "in_to_mid", 256, 2048).expect("init in_to_mid");
    try_push(&mut in_to_mid, "in_to_mid", 256, 2048, b"frame-a").expect("push");
    let mut out = [0u8; 2048];
    let len = try_pop(&mut in_to_mid, "in_to_mid", 256, 2048, &mut out).expect("pop");
    assert_eq!(len, 2048);
    assert_eq!(&out[..7], b"frame-a");
}

#[test]
fn gate_d_malicious_ipc_corruption_is_detected() {
    let mut backing = vec![0u8; IPC_RING_BYTES];
    init_ring(&mut backing, "mid_to_out", 256, 2048).expect("init");
    backing[8] = 0xFF;
    assert!(matches!(
        validate_ring(&backing, "mid_to_out", 256, 2048),
        Err(IpcError::CorruptionDetected | IpcError::InvalidHeader | IpcError::ParameterMismatch)
    ));
}

#[test]
fn gate_d_guest_boot_info_for_all_partitions() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
    for vm_id in 0..3 {
        let blob =
            build_guest_boot_info(&platform, VmId::new(vm_id), VcpuId::new(0)).expect("build");
        let info = unsafe { &*(blob.bytes.as_ptr() as *const GuestBootInfo) };
        let result = unsafe { layout::validate_guest_boot_info_layout(info, blob.bytes.len()) };
        assert!(result.is_ok(), "vm{vm_id} layout invalid");
    }
}

#[test]
fn gate_d_resolved_ept_includes_ipc_mappings() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
    let mid = platform.ept.partitions.iter().find(|part| part.vm_id.raw() == 1).expect("mid ept");
    assert!(mid.mappings.len() >= 3, "mid should map RAM + 2 IPC channels");
}

#[test]
fn gate_d_runtime_initialize_with_allocated_ipc_backing() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
    let mut plans = gate_d_plans_from_resolved(&platform);
    plans.require_config_hash = false;

    let mut ipc_backing = vec![0u8; IPC_RING_BYTES * plans.ipc_channels.len()];
    let mut offset = 0usize;
    for channel in &mut plans.ipc_channels {
        let size = channel.shared_bytes as usize;
        channel.host_base = HostPhysAddr::new(ipc_backing.as_mut_ptr() as u64 + offset as u64);
        offset += size;
    }

    let total = core::mem::size_of::<hv_boot_abi::BootInfo>();
    let info = hv_boot_abi::BootInfo {
        header: hv_boot_abi::BootInfoHeader {
            magic: hv_boot_abi::BOOT_INFO_MAGIC,
            version_major: hv_boot_abi::BOOT_ABI_VERSION_MAJOR,
            version_minor: hv_boot_abi::BOOT_ABI_VERSION_MINOR,
            total_size: total as u32,
            config_hash: platform.config_hash.bytes(),
        },
        acpi: hv_boot_abi::BootAcpiInfo { rsdp_address: 0 },
        memory_map_entry_count: 0,
        flags: 0,
        hypervisor_load_address: 0x100000,
        boot_info_bytes: total as u32,
    };

    match initialize_gate_d(&info, &plans) {
        Ok(report) => {
            assert_eq!(report.partitions.ipc_rings, 2);
            assert_eq!(report.partitions.partitions, 3);
            assert_eq!(report.launch.planned_launches, 3);
            assert_eq!(report.launch.vmcs_field_sets, 3);
            assert!(report.launch.mmio_devices >= 2);
        }
        Err(err) => {
            assert!(
                matches!(
                    err,
                    hv_runtime::RuntimeError::Cpu(_)
                        | hv_runtime::RuntimeError::TableRegionUnavailable
                        | hv_runtime::RuntimeError::Ept(_)
                        | hv_runtime::RuntimeError::Vtd(_)
                ),
                "unexpected error: {}",
                err.as_str()
            );
        }
    }
}

#[test]
fn gate_d_config_hash_mismatch_is_fail_closed() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
    let plans = gate_d_plans_from_resolved(&platform);
    let total = core::mem::size_of::<hv_boot_abi::BootInfo>();
    let info = hv_boot_abi::BootInfo {
        header: hv_boot_abi::BootInfoHeader {
            magic: hv_boot_abi::BOOT_INFO_MAGIC,
            version_major: hv_boot_abi::BOOT_ABI_VERSION_MAJOR,
            version_minor: hv_boot_abi::BOOT_ABI_VERSION_MINOR,
            total_size: total as u32,
            config_hash: [0; 32],
        },
        acpi: hv_boot_abi::BootAcpiInfo { rsdp_address: 0 },
        memory_map_entry_count: 0,
        flags: 0,
        hypervisor_load_address: 0x100000,
        boot_info_bytes: total as u32,
    };
    assert!(matches!(
        initialize_gate_d(&info, &plans),
        Err(hv_runtime::RuntimeError::ConfigHashMismatch)
    ));
}

#[test]
fn gate_d_e1000_mmio_status_is_link_up() {
    let state = hv_e1000::E1000DeviceState::new_link_up();
    let status = hv_e1000::mmio_read(&state, hv_e1000::REG_STATUS).expect("status");
    assert_ne!(status & 0x80, 0);
}

#[test]
fn gate_d_guest_vmcs_launch_plan_includes_ept_pointer() {
    use hv_types::{GuestPhysAddr, HostPhysAddr, VmId};
    use hv_vmx::{build_guest_vmcs_fields, vmcs::EPT_POINTER, GuestLaunchPlan};
    let plan = GuestLaunchPlan {
        vm_id: VmId::new(0),
        vmcs_hpa: HostPhysAddr::new(0x2400_0000),
        guest_entry: GuestPhysAddr::new(0x1000),
        guest_stack: GuestPhysAddr::new(0x8000),
        ept_root_hpa: HostPhysAddr::new(0x2000_1000),
        guest_boot_info_gpa: GuestPhysAddr::new(0x9000),
    };
    let fields = build_guest_vmcs_fields(&plan).expect("fields");
    assert!(fields.iter().any(|field| field.field == EPT_POINTER));
}

#[test]
fn gate_d_gate_c_plans_remain_compatible() {
    let _ = GateCPlans {
        ept: hv_ept::EptPlan { partitions: Vec::new() },
        vtd: hv_vtd::VtdPlan { domains: Vec::new() },
        ept_table_base: HostPhysAddr::new(0x2000_0000),
        vtd_table_base: HostPhysAddr::new(0x2100_0000),
        vmxon_region_base: HostPhysAddr::new(0x2200_0000),
    };
}
