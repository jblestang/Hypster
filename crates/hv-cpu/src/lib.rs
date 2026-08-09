//! Host CPU feature probing via CPUID.

#![no_std]
#![warn(missing_docs)]

mod error;
mod probe;

pub use error::CpuProbeError;
pub use probe::{probe_cpu, CpuFeatures};
