//! Low-level x86_64 intrinsics wrappers for Hypster.
//!
//! All register and MSR access helpers are `unsafe` at the call site because
//! they manipulate privileged CPU state.

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // privileged register and MSR access wrappers

pub mod control;
pub mod cpuid;
pub mod cr;
pub mod error;
pub mod msr;

pub use control::{
    FEATURE_CONTROL_LOCKED, FEATURE_CONTROL_VMX_IN_SMX, FEATURE_CONTROL_VMX_OUTSIDE_SMX,
    CR4_VMX_ENABLE,
};
pub use cpuid::{cpuid, CpuidLeaf};
pub use cr::{read_cr0, read_cr4, write_cr4};
pub use error::X86Error;
pub use msr::{
    rdmsr, wrmsr, IA32_EFER, IA32_FEATURE_CONTROL, IA32_VMX_BASIC, IA32_VMX_CR0_FIXED0,
    IA32_VMX_CR0_FIXED1, IA32_VMX_CR4_FIXED0, IA32_VMX_CR4_FIXED1,
};
