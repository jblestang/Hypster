//! Low-level x86 helpers used during early hypervisor init.

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Raw MSR/CR access and identity-mapped physical reads.

#[cfg(feature = "std")]
extern crate std;

pub mod cr;
mod error;
pub mod msr;
mod phys;

pub use cr::{read_cr4, write_cr4, Cr4Flags};
pub use error::X86Error;
pub use msr::{
    read_msr, write_msr, Msr, IA32_FEATURE_CONTROL, IA32_VMX_BASIC, IA32_VMX_CR0_FIXED0,
    IA32_VMX_CR0_FIXED1, IA32_VMX_CR4_FIXED0, IA32_VMX_CR4_FIXED1,
};
pub use phys::{read_phys_bytes, read_phys_u8};
