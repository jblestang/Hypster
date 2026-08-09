//! Host CPU feature probing via CPUID.

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // CPUID intrinsics.

#[cfg(feature = "std")]
extern crate std;

mod error;
mod probe;

pub use error::CpuProbeError;
pub use probe::{probe_cpu, probe_cpu_optional, CpuFeatures};
