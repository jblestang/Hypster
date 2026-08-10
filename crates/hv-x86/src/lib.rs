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

pub use cr::{read_cr0, read_cr4, write_cr0, write_cr4, Cr4Flags};
pub use error::X86Error;
pub use msr::{
    read_msr, write_msr, Msr, IA32_EFER, IA32_FEATURE_CONTROL, IA32_FS_BASE, IA32_GS_BASE,
    IA32_SYSENTER_CS, IA32_SYSENTER_EIP, IA32_SYSENTER_ESP, IA32_VMX_BASIC,
    IA32_VMX_CR0_FIXED0, IA32_VMX_CR0_FIXED1, IA32_VMX_CR4_FIXED0, IA32_VMX_CR4_FIXED1,
    IA32_VMX_ENTRY_CTLS, IA32_VMX_EXIT_CTLS, IA32_VMX_PINBASED_CTLS, IA32_VMX_PROCBASED_CTLS,
    IA32_VMX_PROCBASED_CTLS2, IA32_VMX_TRUE_ENTRY_CTLS, IA32_VMX_TRUE_EXIT_CTLS,
    IA32_VMX_TRUE_PINBASED_CTLS, IA32_VMX_TRUE_PROCBASED_CTLS,
};
pub use phys::{read_phys_bytes, read_phys_u8};
