//! Guest VMLAUNCH under VMX (requires `hardware` feature).

#![allow(static_mut_refs)]

use hv_runtime::{
    capture_host_launch_context, dispatch_in_guest_vmexit, launch_plan_for_vm,
    peek_in_launch_payload, peek_mid_launch_relay, resume_guest, stage_guest_image,
    verify_mid_to_out_drained, vmlaunch_guest, GateDInitReport, GateDPlans, MmioDispatch,
    RuntimeError,
};
use hv_types::VmId;
use hv_vmx::VmxCapabilities;

use crate::datapath;
use crate::serial;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/guest_in_boot_info.rs"));
    include!(concat!(env!("OUT_DIR"), "/guest_in_image.rs"));
    include!(concat!(env!("OUT_DIR"), "/guest_mid_boot_info.rs"));
    include!(concat!(env!("OUT_DIR"), "/guest_mid_image.rs"));
    include!(concat!(env!("OUT_DIR"), "/guest_out_boot_info.rs"));
    include!(concat!(env!("OUT_DIR"), "/guest_out_image.rs"));
}

use embedded::{
    GUEST_IN_BOOT_INFO, GUEST_IN_IMAGE, GUEST_MID_BOOT_INFO, GUEST_MID_IMAGE, GUEST_OUT_BOOT_INFO,
    GUEST_OUT_IMAGE,
};

const IN_VM_ID: VmId = VmId::new(0);
const MID_VM_ID: VmId = VmId::new(1);
const OUT_VM_ID: VmId = VmId::new(2);
const LAUNCH_PAYLOAD: &[u8] = b"vmx-mid-relay";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveGuest {
    In,
    Mid,
    Out,
}

struct LaunchStash {
    plans: *const GateDPlans,
    report: GateDInitReport,
}

static mut LAUNCH_STASH: Option<LaunchStash> = None;
static mut GUEST_MMIO: Option<MmioDispatch> = None;
static mut ACTIVE_GUEST: ActiveGuest = ActiveGuest::In;

/// Runs the IN → MID → OUT guest launch chain under VMX when available.
///
/// Returns `true` when the chain launched and the VM-exit path entered the datapath loop.
/// Returns `false` when staging or launch preparation fails.
///
/// # Safety
///
/// Caller must have completed Gate D init with VMX enabled. `plans` must remain valid until
/// this function returns or hands off to the datapath loop.
pub unsafe fn try_launch_guest_chain(plans: &GateDPlans, report: GateDInitReport) -> bool {
    if guest_images_missing() {
        return false;
    }

    ACTIVE_GUEST = ActiveGuest::In;
    launch_guest(plans, report, IN_VM_ID, GUEST_IN_IMAGE, GUEST_IN_BOOT_INFO, || Ok(()))
}

extern "C" {
    fn vmexit_entry() -> !;
}

/// VM-exit dispatch routine reached from `vmexit_entry`.
#[no_mangle]
extern "C" fn vmexit_dispatch() -> ! {
    loop {
        let dispatch = unsafe { GUEST_MMIO.as_mut() };
        let Some(dispatch) = dispatch else {
            serial::write_str("hypster: vmexit no mmio\n");
            halt_forever();
        };

        match unsafe { dispatch_in_guest_vmexit(dispatch) } {
            Ok(true) => match unsafe { ACTIVE_GUEST } {
                ActiveGuest::In => handle_in_guest_halt(),
                ActiveGuest::Mid => handle_mid_guest_halt(),
                ActiveGuest::Out => handle_out_guest_halt(),
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
                        serial::write_str(" rip=");
                        match unsafe { hv_vmx::vmread(hv_vmx::vmcs::GUEST_RIP) } {
                            Ok(rip) => write_hex_u64(rip),
                            Err(_) => serial::write_str("????"),
                        }
                        serial::write_str(" qual=");
                        match unsafe { hv_vmx::vmread(hv_vmx::vmcs::EXIT_QUALIFICATION) } {
                            Ok(qual) => write_hex_u64(qual),
                            Err(_) => serial::write_str("????"),
                        }
                        serial::write_str(" eptp=");
                        match unsafe { hv_vmx::vmread(hv_vmx::vmcs::EPT_POINTER) } {
                            Ok(eptp) => write_hex_u64(eptp),
                            Err(_) => serial::write_str("????"),
                        }
                        serial::write_str(" cr3=");
                        match unsafe { hv_vmx::vmread(hv_vmx::vmcs::GUEST_CR3) } {
                            Ok(cr3) => write_hex_u64(cr3),
                            Err(_) => serial::write_str("????"),
                        }
                        serial::write_str("\n");
                    }
                    Err(_) => serial::write_str("hypster: vmexit read fail\n"),
                }
                halt_forever();
            }
        }
    }
}

fn handle_in_guest_halt() -> ! {
    match peek_in_guest_payload() {
        Ok(()) => {
            serial::write_str("hypster: guest in ok\n");
            launch_mid_guest_from_stash();
        }
        Err(()) => {
            serial::write_str("hypster: guest in push fail\n");
            halt_forever();
        }
    }
}

fn handle_mid_guest_halt() -> ! {
    match peek_mid_guest_relay() {
        Ok(()) => {
            serial::write_str("hypster: guest mid ok\n");
            launch_out_guest_from_stash();
        }
        Err(()) => {
            serial::write_str("hypster: guest mid relay fail\n");
            halt_forever();
        }
    }
}

fn handle_out_guest_halt() -> ! {
    match verify_out_guest_drain() {
        Ok(()) => {
            serial::write_str("hypster: guest out ok\n");
            run_datapath_from_stash();
        }
        Err(()) => {
            serial::write_str("hypster: guest out drain fail\n");
            halt_forever();
        }
    }
}

fn launch_mid_guest_from_stash() -> ! {
    let stash = stash_ref_or_halt();
    let launched = unsafe {
        launch_follow_on_guest(
            &*stash.plans,
            stash.report,
            ActiveGuest::Mid,
            MID_VM_ID,
            GUEST_MID_IMAGE,
            GUEST_MID_BOOT_INFO,
        )
    };
    if launched {
        // SAFETY: successful VMLAUNCH does not return to this path.
        unsafe {
            core::hint::unreachable_unchecked();
        }
    }
    serial::write_str("hypster: guest mid launch fail\n");
    halt_forever();
}

fn launch_out_guest_from_stash() -> ! {
    let stash = stash_ref_or_halt();
    let launched = unsafe {
        launch_follow_on_guest(
            &*stash.plans,
            stash.report,
            ActiveGuest::Out,
            OUT_VM_ID,
            GUEST_OUT_IMAGE,
            GUEST_OUT_BOOT_INFO,
        )
    };
    if launched {
        // SAFETY: successful VMLAUNCH does not return to this path.
        unsafe {
            core::hint::unreachable_unchecked();
        }
    }
    serial::write_str("hypster: guest out launch fail\n");
    halt_forever();
}

unsafe fn launch_follow_on_guest(
    plans: &GateDPlans,
    report: GateDInitReport,
    guest: ActiveGuest,
    vm_id: VmId,
    image: &[u8],
    boot_info: &[u8],
) -> bool {
    ACTIVE_GUEST = guest;
    launch_guest(plans, report, vm_id, image, boot_info, || Ok(()))
}

unsafe fn launch_guest(
    plans: &GateDPlans,
    report: GateDInitReport,
    vm_id: VmId,
    image: &[u8],
    boot_info: &[u8],
    prepare: impl FnOnce() -> Result<(), RuntimeError>,
) -> bool {
    let mut launch = match launch_plan_for_vm(plans, vm_id) {
        Some(plan) => plan,
        None => return false,
    };
    launch.guest_entry = match stage_guest_image(plans, vm_id, image, boot_info) {
        Ok(entry) => entry,
        Err(_) => return false,
    };

    if prepare().is_err() {
        return false;
    }

    let partition = match plans.gate_c.ept.partitions.iter().find(|part| part.vm_id == vm_id) {
        Some(partition) => partition,
        None => return false,
    };
    GUEST_MMIO = Some(MmioDispatch::from_ept_mappings(&partition.mappings));
    LAUNCH_STASH = Some(LaunchStash { plans, report });

    let caps = VmxCapabilities::from_hardware().unwrap_or(VmxCapabilities::from_assumed_qemu());
    let host = capture_host_launch_context(vmexit_entry as usize as u64);
    match vmlaunch_guest(&launch, host, caps) {
        Ok(()) => {}
        Err(err) => {
            serial::write_str("hypster: vmlaunch fail: ");
            serial::write_str(err.as_str());
            serial::write_str(" err=");
            match unsafe { hv_runtime::read_vm_instruction_error() } {
                Ok(code) => write_hex_u32(code),
                Err(_) => serial::write_str("????"),
            }
            serial::write_str("\n");
            LAUNCH_STASH = None;
            GUEST_MMIO = None;
            return false;
        }
    }

    core::hint::unreachable_unchecked();
}

fn run_datapath_from_stash() -> ! {
    let stash = unsafe { LAUNCH_STASH.take() };
    let Some(stash) = stash else {
        serial::write_str("hypster: launch stash missing\n");
        halt_forever();
    };
    unsafe {
        GUEST_MMIO = None;
        datapath::run_steady_state_loop(&*stash.plans, stash.report);
    }
}

fn stash_ref_or_halt() -> &'static LaunchStash {
    // SAFETY: stash is set before VMLAUNCH and `plans` outlives this VM-exit handler.
    let stash = unsafe { LAUNCH_STASH.as_ref() };
    stash.unwrap_or_else(|| {
        serial::write_str("hypster: launch stash missing\n");
        halt_forever();
    })
}

fn peek_in_guest_payload() -> Result<(), ()> {
    let stash = stash_ref_or_halt();
    peek_in_launch_payload(unsafe { &*stash.plans }, LAUNCH_PAYLOAD).map(|_| ()).map_err(|_| ())
}

fn peek_mid_guest_relay() -> Result<(), ()> {
    let stash = stash_ref_or_halt();
    peek_mid_launch_relay(unsafe { &*stash.plans }, LAUNCH_PAYLOAD).map(|_| ()).map_err(|_| ())
}

fn verify_out_guest_drain() -> Result<(), ()> {
    let stash = stash_ref_or_halt();
    verify_mid_to_out_drained(unsafe { &*stash.plans }).map_err(|_| ())
}

fn guest_images_missing() -> bool {
    GUEST_IN_IMAGE.is_empty()
        || GUEST_IN_BOOT_INFO.is_empty()
        || GUEST_MID_IMAGE.is_empty()
        || GUEST_MID_BOOT_INFO.is_empty()
        || GUEST_OUT_IMAGE.is_empty()
        || GUEST_OUT_BOOT_INFO.is_empty()
}

fn write_hex_u32(value: u32) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0xF) as usize;
        serial::write_byte(HEX[nibble]);
    }
}

fn write_hex_u64(value: u64) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in (0..16).rev() {
        let nibble = ((value >> (shift * 4)) & 0xF) as usize;
        serial::write_byte(HEX[nibble]);
    }
}

fn halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
