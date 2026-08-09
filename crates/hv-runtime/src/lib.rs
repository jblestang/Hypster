//! Hypervisor runtime initialization after UEFI handoff (Gate C).

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Identity-mapped firmware tables and loader-reserved regions.

extern crate alloc;

mod error;
mod init;

pub use error::RuntimeError;
pub use init::{initialize, GateCPlans, RuntimeInitReport};
