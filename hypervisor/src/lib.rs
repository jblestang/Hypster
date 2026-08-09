//! Hypster hypervisor library (Gate C entrypoint).

#![no_std]
#![allow(unsafe_code)] // Bare-metal entry, table installs, and inline asm.

extern crate alloc;

mod datapath;
pub mod entry;
#[cfg(feature = "hardware")]
mod launch;
mod plans;
mod serial;
#[cfg(feature = "hardware")]
mod vmexit_host;

pub use entry::hypster_entry;
