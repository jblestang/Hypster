//! Hypster hypervisor library (Gate C entrypoint).

#![no_std]

extern crate alloc;

mod bump_alloc;
pub mod entry;
mod plans;

pub use entry::hypster_entry;
