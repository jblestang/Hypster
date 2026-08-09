//! CPU probe errors.

use core::fmt;

/// Errors produced while probing or validating CPU capabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpuProbeError {
    /// The host architecture is not supported.
    UnsupportedArchitecture,
    /// A required feature for VMX bring-up is missing.
    MissingRequiredFeature {
        /// Human-readable feature name.
        feature: &'static str,
    },
}

impl fmt::Display for CpuProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArchitecture => f.write_str("cpu probing requires x86_64"),
            Self::MissingRequiredFeature { feature } => {
                write!(f, "missing required cpu feature: {feature}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CpuProbeError {}
