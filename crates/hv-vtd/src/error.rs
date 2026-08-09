//! VT-d planner errors.

use core::fmt;

/// Errors produced while building VT-d plans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VtdPlanError {
    /// Expected PCI device not discovered on platform.
    MissingPciDevice {
        /// BDF string.
        bdf: alloc::string::String,
    },
    /// Device assigned to multiple domains.
    DeviceOwnershipConflict {
        /// BDF string.
        bdf: alloc::string::String,
    },
    /// Address overflow.
    Overflow,
}

impl fmt::Display for VtdPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPciDevice { bdf } => write!(f, "PCI device `{bdf}` not observed"),
            Self::DeviceOwnershipConflict { bdf } => {
                write!(f, "PCI device `{bdf}` assigned to multiple domains")
            }
            Self::Overflow => f.write_str("VT-d planner overflow"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VtdPlanError {}
