//! Shared guest partition helpers.

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)]

mod ipc;
mod topology;

pub use hv_guest_abi::{GuestBootInfo, GUEST_BOOT_INFO_MAGIC};
pub use ipc::{relay_one_frame, run_mid_relay_loop};
pub use topology::{
    names, IPC_SLOT_COUNT, IPC_SLOT_SIZE, MID_IN_TO_MID_GPA, MID_MID_TO_OUT_GPA,
};

/// Returns true when `info` has a compatible guest boot info header.
#[must_use]
pub const fn is_boot_info_compatible(info: &GuestBootInfo) -> bool {
    info.header.magic == GUEST_BOOT_INFO_MAGIC && info.header.is_compatible()
}
