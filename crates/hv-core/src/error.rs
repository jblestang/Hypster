//! Core error type.

use core::fmt;

use crate::boot::BootTransitionError;

/// Errors surfaced by platform validation and resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    /// Boot state transition failed.
    BootTransition(BootTransitionError),
    /// Boot info rejected.
    BootInfo(&'static str),
    /// Platform contract validation failed.
    #[cfg(feature = "std")]
    Validation(crate::validate::PlatformValidationError),
    /// CPU topology planning failed.
    #[cfg(feature = "std")]
    Cpu(hv_cpu_topology::CpuTopologyError),
    /// Memory planning failed.
    #[cfg(feature = "std")]
    Memory(hv_memory::MemoryPlanError),
    /// EPT planning failed.
    #[cfg(feature = "std")]
    Ept(hv_ept::EptPlanError),
    /// VT-d planning failed.
    #[cfg(feature = "std")]
    Vtd(hv_vtd::VtdPlanError),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BootTransition(err) => write!(f, "{err}"),
            Self::BootInfo(msg) => write!(f, "boot info error: {msg}"),
            #[cfg(feature = "std")]
            Self::Validation(err) => write!(f, "{err}"),
            #[cfg(feature = "std")]
            Self::Cpu(err) => write!(f, "{err}"),
            #[cfg(feature = "std")]
            Self::Memory(err) => write!(f, "{err}"),
            #[cfg(feature = "std")]
            Self::Ept(err) => write!(f, "{err}"),
            #[cfg(feature = "std")]
            Self::Vtd(err) => write!(f, "{err}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CoreError {}
