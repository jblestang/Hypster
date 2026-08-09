//! Guest VMLAUNCH under VMX (requires `hardware` feature).

#![allow(static_mut_refs)]

use hv_runtime::{
    capture_host_launch_context, dispatch_in_guest_vmexit, launch_plan_for_vm, resume_guest,
    stage_guest_image, vmlaunch_guest, GateDInitReport, GateDPlans, MmioDispatch,
};
use hv_types::VmId;
use hv_vmx::VmxCapabilities;

use crate::datapath;
use crate::serial;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/guest_in_image.rs"));
}

use embedded::GUEST_IN_IMAGE;

struct LaunchStash {
    plans: *const GateDPlans,
    report: GateDInitReport,
}

static mut LAUNCH_STASH: Option<LaunchStash> = None;
static mut IN_MMIO: Option<MmioDispatch> = None;

/// Stages the IN guest and attempts VMLAUNCH when VMX is available.
///
/// Returns `true` when the guest launched and the VM-exit path entered the datapath loop.
/// Returns `false` when staging or launch preparation fails.
///
/// # Safety
///
/// Caller must have completed Gate D init with VMX enabled. `plans` must remain valid until
/// this function returns or hands off to the datapath loop.
pub unsafe fn try_launch_in_guest(plans: &GateDPlans, report: GateDInitReport) -> bool {
    if GUEST_IN_IMAGE.is_empty() {
        return false;
    }

    let vm_id = VmId::new(0);
    let boot_info = minimal_boot_info_bytes();
    let mut launch = match launch_plan_for_vm(plans, vm_id) {
        Some(plan) => plan,
        None => return false,
    };
    launch.guest_entry = match stage_guest_image(plans, vm_id, GUEST_IN_IMAGE, &boot_info) {
        Ok(entry) => entry,
        Err(_) => return false,
    };

    let partition = match plans.gate_c.ept.partitions.iter().find(|part| part.vm_id == vm_id) {
        Some(partition) => partition,
        None => return false,
    };
    IN_MMIO = Some(MmioDispatch::from_ept_mappings(&partition.mappings));
    LAUNCH_STASH = Some(LaunchStash { plans, report });

    let caps = VmxCapabilities::from_hardware().unwrap_or(VmxCapabilities::from_assumed_qemu());
    let host = capture_host_launch_context(vmexit_entry as usize as u64);
    if vmlaunch_guest(&launch, host, caps).is_err() {
        LAUNCH_STASH = None;
        IN_MMIO = None;
        return false;
    }

    core::hint::unreachable_unchecked();
}

extern "C" {
    fn vmexit_entry() -> !;
}

/// VM-exit dispatch routine reached from `vmexit_entry`.
#[no_mangle]
extern "C" fn vmexit_dispatch() -> ! {
    loop {
        let dispatch = unsafe { IN_MMIO.as_mut() };
        let Some(dispatch) = dispatch else {
            serial::write_str("hypster: vmexit no mmio\n");
            halt_forever();
        };

        match unsafe { dispatch_in_guest_vmexit(dispatch) } {
            Ok(true) => {
                serial::write_str("hypster: vmlaunch ok\n");
                run_datapath_from_stash();
            }
            Ok(false) => match unsafe { resume_guest() } {
                Ok(()) => {}
                Err(_) => {
                    serial::write_str("hypster: vmresume fail\n");
                    halt_forever();
                }
            },
            Err(_) => {
                match unsafe { hv_runtime::read_vmexit_reason() } {
                    Ok(reason) => {
                        serial::write_str("hypster: vmexit reason=");
                        write_hex_u32(reason);
                        serial::write_str("\n");
                    }
                    Err(_) => serial::write_str("hypster: vmexit read fail\n"),
                }
                halt_forever();
            }
        }
    }
}

fn run_datapath_from_stash() -> ! {
    // SAFETY: stash is set before VMLAUNCH and `plans` outlives this init path.
    let stash = unsafe { LAUNCH_STASH.take() };
    let Some(stash) = stash else {
        serial::write_str("hypster: launch stash missing\n");
        halt_forever();
    };
    unsafe {
        IN_MMIO = None;
        datapath::run_steady_state_loop(&*stash.plans, stash.report);
    }
}

fn minimal_boot_info_bytes() -> [u8; core::mem::size_of::<hv_guest_abi::GuestBootInfo>()] {
    let info = hv_guest_abi::GuestBootInfo {
        header: hv_guest_abi::GuestBootInfoHeader {
            magic: hv_guest_abi::GUEST_BOOT_INFO_MAGIC,
            version_major: hv_guest_abi::GUEST_ABI_VERSION_MAJOR,
            version_minor: hv_guest_abi::GUEST_ABI_VERSION_MINOR,
            total_size: core::mem::size_of::<hv_guest_abi::GuestBootInfo>() as u32,
            vm_id: 0,
            vcpu_id: 0,
        },
        memory_region_count: 0,
        ipc_region_count: 0,
        mmio_region_count: 0,
        reserved: 0,
    };
    let mut bytes = [0u8; core::mem::size_of::<hv_guest_abi::GuestBootInfo>()];
    unsafe {
        core::ptr::write(bytes.as_mut_ptr() as *mut hv_guest_abi::GuestBootInfo, info);
    }
    bytes
}

fn write_hex_u32(value: u32) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0xF) as usize;
        serial::write_byte(HEX[nibble]);
    }
}

fn halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
