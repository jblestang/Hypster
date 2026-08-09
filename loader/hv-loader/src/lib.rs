//! UEFI loader library for Hypster (Gate C boot handoff).

#![no_std]
#![allow(unsafe_code)] // UEFI memory map, handoff jump, and firmware allocations.

extern crate alloc;

pub mod boot_info;
pub mod handoff;
