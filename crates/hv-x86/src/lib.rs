//! Low-level x86 helpers used during early hypervisor init.

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Raw MSR/CR access and identity-mapped physical reads.

#[cfg(feature = "std")]
extern crate std;

mod error;
pub mod cr;
pub mod msr;
mod phys;

pub use cr::{read_cr4, write_cr4, Cr4Flags};
pub use error::X86Error;
pub use msr::{
    read_msr, write_msr, IA32_FEATURE_CONTROL, IA32_VMX_BASIC, IA32_VMX_CR0_FIXED0,
    IA32_VMX_CR0_FIXED1, IA32_VMX_CR4_FIXED0, IA32_VMX_CR4_FIXED1, Msr,
};
pub use phys::{read_phys_bytes, read_phys_u8};
