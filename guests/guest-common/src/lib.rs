//! Shared guest partition helpers.

#![no_std]
#![warn(missing_docs)]

use hv_guest_abi::{GuestBootInfo, GUEST_BOOT_INFO_MAGIC};

/// Returns true when `info` has a compatible guest boot info header.
#[must_use]
pub const fn is_boot_info_compatible(info: &GuestBootInfo) -> bool {
    info.header.magic == GUEST_BOOT_INFO_MAGIC && info.header.is_compatible()
}
