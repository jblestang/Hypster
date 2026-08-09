//! Hypster hypervisor library (Gate C entrypoint).

#![no_std]
#![allow(unsafe_code)] // Bare-metal entry, table installs, and inline asm.

extern crate alloc;

pub mod entry;
mod datapath;
mod plans;

pub use entry::hypster_entry;
