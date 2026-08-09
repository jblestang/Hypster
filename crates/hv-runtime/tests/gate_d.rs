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
use hv_runtime::{assign_ipc_host_backing, initialize_gate_d, DatapathEngine, GateCPlans, MmioDispatch};
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

    match initialize_gate_d(&info, &mut plans) {
        Ok(report) => {
            assert_eq!(report.partitions.ipc_rings, 2);
            assert_eq!(report.partitions.partitions, 3);
            assert_eq!(report.launch.planned_launches, 3);
            assert_eq!(report.launch.vmcs_field_sets, 3);
            assert!(report.launch.mmio_devices >= 2);
            assert_eq!(report.datapath.ipc_channels, 2);
            assert!(report.datapath.mmio_devices >= 2);
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
    let mut plans = gate_d_plans_from_resolved(&platform);
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
        initialize_gate_d(&info, &mut plans),
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
        vmcs_hpa: HostPhysAddr::new(0x1_1800_0000),
        guest_entry: GuestPhysAddr::new(0x1000),
        guest_stack: GuestPhysAddr::new(0x8000),
        ept_root_hpa: HostPhysAddr::new(0x1_1510_1000),
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
        ept_table_base: HostPhysAddr::new(0x1_1510_0000),
        vtd_table_base: HostPhysAddr::new(0x1_1610_0000),
        vmxon_region_base: HostPhysAddr::new(0x1_1710_0000),
        ept_root_hp_as: Vec::new(),
    };
}

#[test]
fn gate_d_assign_ipc_backing_aligns_ept_ipc_host_phys() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
    let mut plans = gate_d_plans_from_resolved(&platform);

    let mut ipc_backing = vec![0u8; 524_328 * plans.ipc_channels.len()];
    assign_ipc_host_backing(&mut plans, &mut ipc_backing).expect("assign");

    for channel in &plans.ipc_channels {
        let mut mapping_count = 0usize;
        for partition in &plans.gate_c.ept.partitions {
            for mapping in &partition.mappings {
                if mapping.host_phys == channel.host_base {
                    mapping_count += 1;
                }
            }
        }
        assert!(mapping_count >= 2, "channel {} should map producer and consumer", channel.name);
    }
}

#[test]
fn gate_d_vmcs_includes_ept_violation_decode_fields() {
    use hv_vmx::vmcs::{EXIT_QUALIFICATION, GUEST_PHYSICAL_ADDRESS, VM_EXIT_INSTRUCTION_LEN};
    assert_ne!(GUEST_PHYSICAL_ADDRESS, 0);
    assert_ne!(EXIT_QUALIFICATION, 0);
    assert_ne!(VM_EXIT_INSTRUCTION_LEN, 0);
}

#[test]
fn gate_d_e2e_datapath_moves_payload_through_engine() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
    let mut plans = gate_d_plans_from_resolved(&platform);

    use hv_ipc::compute_shared_bytes;

    let total_backing: usize = plans
        .ipc_channels
        .iter()
        .map(|channel| {
            compute_shared_bytes(channel.slot_count, channel.slot_size).expect("ring bytes")
                as usize
        })
        .sum();
    let mut ipc_backing = vec![0u8; total_backing];
    assign_ipc_host_backing(&mut plans, &mut ipc_backing).expect("assign backing");
    for channel in &plans.ipc_channels {
        let ptr = channel.host_base.raw() as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, channel.shared_bytes as usize) };
        hv_ipc::init_ring(slice, &channel.name, channel.slot_count, channel.slot_size)
            .expect("init ring");
    }

    let mut engine = DatapathEngine::from_plans(&plans, &mut ipc_backing).expect("engine");
    let mut out = [0u8; 2048];
    let report = engine.run_e2e_once(b"gate-d-udp-payload", &mut out).expect("e2e transfer");
    assert_eq!(report.step.in_to_mid_frames, 1);
    assert!(report.outbound_bytes >= b"gate-d-udp-payload".len());
    assert_eq!(&out[..b"gate-d-udp-payload".len()], b"gate-d-udp-payload");
}

#[test]
fn gate_d_mmio_dispatch_covers_e1000_mappings() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
    let in_partition =
        platform.ept.partitions.iter().find(|part| part.vm_id.raw() == 0).expect("in ept");
    let dispatch = MmioDispatch::from_ept_mappings(&in_partition.mappings);
    let status = dispatch
        .read32(hv_types::GuestPhysAddr::new(0xFEB0_0000), hv_e1000::REG_STATUS)
        .expect("status");
    assert_ne!(status & 0x80, 0);
}

#[test]
fn gate_d_elf_loader_places_segments_at_guest_phys_zero() {
    use hv_runtime::load_elf_into_guest_ram;

    fn minimal_elf(entry: u64, load_vaddr: u64, payload: &[u8]) -> Vec<u8> {
        use hv_elf::{
            ELF64_EHDR_SIZE, ELF64_PHDR_SIZE, ELFCLASS64, ELFDATA2LSB, ELF_MAGIC, EM_X86_64,
            PT_LOAD,
        };

        let phoff = ELF64_EHDR_SIZE as u64;
        let file_offset = phoff + ELF64_PHDR_SIZE as u64;
        let total = (file_offset as usize) + payload.len();
        let mut image = vec![0u8; total];
        image[0..4].copy_from_slice(&ELF_MAGIC);
        image[4] = ELFCLASS64;
        image[5] = ELFDATA2LSB;
        image[0x10..0x12].copy_from_slice(&2u16.to_le_bytes());
        image[0x12..0x14].copy_from_slice(&EM_X86_64.to_le_bytes());
        image[0x18..0x20].copy_from_slice(&entry.to_le_bytes());
        image[0x20..0x28].copy_from_slice(&phoff.to_le_bytes());
        image[0x36..0x38].copy_from_slice(&(ELF64_PHDR_SIZE as u16).to_le_bytes());
        image[0x38..0x3A].copy_from_slice(&1u16.to_le_bytes());
        let phdr = ELF64_EHDR_SIZE;
        image[phdr..phdr + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
        image[phdr + 0x08..phdr + 0x10].copy_from_slice(&file_offset.to_le_bytes());
        image[phdr + 0x10..phdr + 0x18].copy_from_slice(&load_vaddr.to_le_bytes());
        image[phdr + 0x18..phdr + 0x20].copy_from_slice(&load_vaddr.to_le_bytes());
        image[phdr + 0x20..phdr + 0x28].copy_from_slice(&(payload.len() as u64).to_le_bytes());
        image[phdr + 0x28..phdr + 0x30].copy_from_slice(&(payload.len() as u64).to_le_bytes());
        image[phdr + 0x30..phdr + 0x38].copy_from_slice(&0x1000u64.to_le_bytes());
        image[file_offset as usize..total].copy_from_slice(payload);
        image
    }

    let image = minimal_elf(0x1000, 0, b"guest-image");
    let mut ram = vec![0u8; 4096];
    let loaded = load_elf_into_guest_ram(&image, &mut ram).expect("load elf");
    assert_eq!(loaded.entry_point, 0x1000);
    assert_eq!(&ram[..11], b"guest-image");
}
