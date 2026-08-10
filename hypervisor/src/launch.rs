//! Guest VMLAUNCH under VMX (requires `hardware` feature).

#![allow(static_mut_refs)]

use hv_runtime::{
    capture_host_launch_context, dispatch_in_guest_vmexit, inject_inbound_payload,
    launch_plan_for_vm, resume_guest, stage_guest_image, verify_mid_launch_relay, vmlaunch_guest,
    ChannelBacking, GateDInitReport, GateDPlans, MmioDispatch, RuntimeError,
};
use hv_types::VmId;
use hv_vmx::VmxCapabilities;

use crate::datapath;
use crate::serial;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/guest_mid_boot_info.rs"));
    include!(concat!(env!("OUT_DIR"), "/guest_mid_image.rs"));
}

use embedded::{GUEST_MID_BOOT_INFO, GUEST_MID_IMAGE};

const MID_VM_ID: VmId = VmId::new(1);
const MID_LAUNCH_PAYLOAD: &[u8] = b"vmx-mid-relay";

struct LaunchStash {
    plans: *const GateDPlans,
    report: GateDInitReport,
}

static mut LAUNCH_STASH: Option<LaunchStash> = None;
static mut MID_MMIO: Option<MmioDispatch> = None;

/// Stages the MID guest, injects one IPC frame, and attempts VMLAUNCH when VMX is available.
///
/// Returns `true` when the guest launched and the VM-exit path entered the datapath loop.
/// Returns `false` when staging or launch preparation fails.
///
/// # Safety
///
/// Caller must have completed Gate D init with VMX enabled. `plans` must remain valid until
/// this function returns or hands off to the datapath loop.
pub unsafe fn try_launch_mid_guest(plans: &GateDPlans, report: GateDInitReport) -> bool {
    if GUEST_MID_IMAGE.is_empty() || GUEST_MID_BOOT_INFO.is_empty() {
        return false;
    }

    let mut launch = match launch_plan_for_vm(plans, MID_VM_ID) {
        Some(plan) => plan,
        None => return false,
    };
    launch.guest_entry =
        match stage_guest_image(plans, MID_VM_ID, GUEST_MID_IMAGE, GUEST_MID_BOOT_INFO) {
            Ok(entry) => entry,
            Err(_) => return false,
        };

    if inject_mid_launch_frame(plans).is_err() {
        return false;
    }

    let partition = match plans.gate_c.ept.partitions.iter().find(|part| part.vm_id == MID_VM_ID) {
        Some(partition) => partition,
        None => return false,
    };
    MID_MMIO = Some(MmioDispatch::from_ept_mappings(&partition.mappings));
    LAUNCH_STASH = Some(LaunchStash { plans, report });

    let caps = VmxCapabilities::from_hardware().unwrap_or(VmxCapabilities::from_assumed_qemu());
    let host = capture_host_launch_context(vmexit_entry as usize as u64);
    if vmlaunch_guest(&launch, host, caps).is_err() {
        LAUNCH_STASH = None;
        MID_MMIO = None;
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
        let dispatch = unsafe { MID_MMIO.as_mut() };
        let Some(dispatch) = dispatch else {
            serial::write_str("hypster: vmexit no mmio\n");
            halt_forever();
        };

        match unsafe { dispatch_in_guest_vmexit(dispatch) } {
            Ok(true) => match verify_mid_guest_relay() {
                Ok(()) => {
                    serial::write_str("hypster: guest mid ok\n");
                    run_datapath_from_stash();
                }
                Err(()) => {
                    serial::write_str("hypster: guest mid relay fail\n");
                    halt_forever();
                }
            },
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
        MID_MMIO = None;
        datapath::run_steady_state_loop(&*stash.plans, stash.report);
    }
}

fn verify_mid_guest_relay() -> Result<(), ()> {
    // SAFETY: stash is set before VMLAUNCH and `plans` outlives this VM-exit handler.
    let stash = unsafe { LAUNCH_STASH.as_ref() };
    let Some(stash) = stash else {
        return Err(());
    };
    verify_mid_launch_relay(unsafe { &*stash.plans }, MID_LAUNCH_PAYLOAD)
        .map(|_| ())
        .map_err(|_| ())
}

fn inject_mid_launch_frame(plans: &GateDPlans) -> Result<(), RuntimeError> {
    let channel = plans
        .ipc_channels
        .iter()
        .find(|ch| ch.name == "in_to_mid")
        .ok_or(RuntimeError::TableRegionUnavailable)?;
    let size = channel.shared_bytes as usize;
    let ptr = channel.host_base.raw() as *mut u8;
    // SAFETY: Gate D init assigned host backing for IPC rings before launch.
    let backing = unsafe { core::slice::from_raw_parts_mut(ptr, size) };
    let mut channel = ChannelBacking {
        name: "in_to_mid",
        backing,
        slot_count: channel.slot_count,
        slot_size: channel.slot_size,
    };
    inject_inbound_payload(&mut channel, MID_LAUNCH_PAYLOAD)?;
    Ok(())
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
