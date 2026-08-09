//! CPU probe errors.

use core::fmt;

/// CPU probing failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuProbeError {
    /// CPUID is unavailable on this host.
    CpuidUnavailable,
    /// Required VMX support is missing.
    MissingVmx,
    /// Required EPT support is missing.
    MissingEpt,
    /// Required NX support is missing.
    MissingNx,
}

impl fmt::Display for CpuProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CpuidUnavailable => write!(f, "CPUID unavailable"),
            Self::MissingVmx => write!(f, "VMX not supported"),
            Self::MissingEpt => write!(f, "EPT not supported"),
            Self::MissingNx => write!(f, "NX not supported"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CpuProbeError {}
