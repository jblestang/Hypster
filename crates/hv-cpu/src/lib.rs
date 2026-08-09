//! CPU capability probing for Hypster.

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod probe;

pub use error::CpuProbeError;
pub use probe::{probe_cpu_capabilities, validate_vmx_prerequisites, CpuCapabilities};
