//! Guest-side IPC relay helpers.

use core::arch::asm;

use hv_ipc::{try_pop, try_push, IpcError};

use crate::boot::{boot_info_ptr, ipc_region_base};
use crate::topology::{
    names, IPC_SHARED_BYTES, IPC_SLOT_COUNT, IPC_SLOT_SIZE, MID_IN_TO_MID_GPA, MID_MID_TO_OUT_GPA,
};

/// Relays at most one frame from `in_to_mid` into `mid_to_out`.
///
/// # Errors
///
/// Returns [`IpcError::QueueEmpty`] when the source ring has no frame ready.
pub fn relay_one_frame(
    in_to_mid: &mut [u8],
    mid_to_out: &mut [u8],
    slot: &mut [u8],
) -> Result<usize, IpcError> {
    let len = try_pop(in_to_mid, names::IN_TO_MID, IPC_SLOT_COUNT, IPC_SLOT_SIZE, slot)?;
    let payload_len = len.min(IPC_SLOT_SIZE as usize);
    try_push(mid_to_out, names::MID_TO_OUT, IPC_SLOT_COUNT, IPC_SLOT_SIZE, &slot[..payload_len])?;
    Ok(payload_len)
}

/// Relays one frame using boot-info IPC regions, then halts the vCPU.
///
/// # Safety
///
/// Hypervisor must have mapped boot info and initialized IPC rings before guest entry.
pub unsafe fn relay_once_then_halt() -> ! {
    let info = &*boot_info_ptr();
    let in_to_mid_gpa = ipc_region_base(info, names::IN_TO_MID).unwrap_or(MID_IN_TO_MID_GPA);
    let mid_to_out_gpa = ipc_region_base(info, names::MID_TO_OUT).unwrap_or(MID_MID_TO_OUT_GPA);
    let in_slice = core::slice::from_raw_parts_mut(in_to_mid_gpa as *mut u8, shared_bytes());
    let out_slice = core::slice::from_raw_parts_mut(mid_to_out_gpa as *mut u8, shared_bytes());
    let mut slot = [0u8; IPC_SLOT_SIZE as usize];
    if relay_one_frame(in_slice, out_slice, &mut slot).is_ok() {
        halt_forever();
    }
    spin_forever();
}

/// Drains one frame from boot-info `mid_to_out`, then halts the vCPU.
///
/// # Safety
///
/// Hypervisor must have mapped boot info and initialized IPC rings before guest entry.
pub unsafe fn drain_once_then_halt() -> ! {
    let info = &*boot_info_ptr();
    let mid_to_out_gpa = ipc_region_base(info, names::MID_TO_OUT).unwrap_or(MID_MID_TO_OUT_GPA);
    let slice = core::slice::from_raw_parts_mut(mid_to_out_gpa as *mut u8, shared_bytes());
    let mut slot = [0u8; IPC_SLOT_SIZE as usize];
    if try_pop(slice, names::MID_TO_OUT, IPC_SLOT_COUNT, IPC_SLOT_SIZE, &mut slot).is_ok() {
        halt_forever();
    }
    spin_forever();
}

/// Runs the MID partition relay loop against identity-mapped IPC backing.
///
/// # Safety
///
/// `in_to_mid` and `mid_to_out` must point to initialized IPC rings mapped at the
/// guest-physical addresses described in the boot info blob.
pub unsafe fn run_mid_relay_loop(in_to_mid: *mut u8, mid_to_out: *mut u8) -> ! {
    let in_slice = core::slice::from_raw_parts_mut(in_to_mid, shared_bytes());
    let out_slice = core::slice::from_raw_parts_mut(mid_to_out, shared_bytes());
    let mut slot = [0u8; IPC_SLOT_SIZE as usize];
    loop {
        if relay_one_frame(in_slice, out_slice, &mut slot).is_ok() {
            continue;
        }
        core::hint::spin_loop();
    }
}

fn shared_bytes() -> usize {
    IPC_SHARED_BYTES
}

fn halt_forever() -> ! {
    loop {
        // SAFETY: guest executes HLT to trap to the hypervisor after relay.
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

fn spin_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    extern crate alloc;

    use alloc::vec;
    use hv_ipc::init_ring;

    use super::*;
    use crate::topology::names;

    #[test]
    fn guest_relay_moves_payload_between_rings() {
        let mut in_to_mid = vec![0u8; shared_bytes()];
        let mut mid_to_out = vec![0u8; shared_bytes()];
        init_ring(&mut in_to_mid, names::IN_TO_MID, IPC_SLOT_COUNT, IPC_SLOT_SIZE)
            .expect("init in");
        init_ring(&mut mid_to_out, names::MID_TO_OUT, IPC_SLOT_COUNT, IPC_SLOT_SIZE)
            .expect("init out");
        hv_ipc::try_push(
            &mut in_to_mid,
            names::IN_TO_MID,
            IPC_SLOT_COUNT,
            IPC_SLOT_SIZE,
            b"guest-relay",
        )
        .expect("push");
        let mut slot = [0u8; IPC_SLOT_SIZE as usize];
        let len = relay_one_frame(&mut in_to_mid, &mut mid_to_out, &mut slot).expect("relay");
        assert!(len >= b"guest-relay".len());
        assert_eq!(&slot[..b"guest-relay".len()], b"guest-relay");
    }
}
