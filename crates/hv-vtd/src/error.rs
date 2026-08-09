//! VT-d planner and installation errors.

use core::fmt;

/// Errors produced while building or installing VT-d plans.
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
    /// Install buffer is too small for VT-d tables.
    BufferTooSmall,
    /// Domain list is empty.
    EmptyDomains,
    /// PCI BDF could not be parsed during install.
    InvalidPciBdf,
}

impl fmt::Display for VtdPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPciDevice { bdf } => write!(f, "PCI device `{bdf}` not observed"),
            Self::DeviceOwnershipConflict { bdf } => {
                write!(f, "PCI device `{bdf}` assigned to multiple domains")
            }
            Self::Overflow => f.write_str("VT-d planner overflow"),
            Self::BufferTooSmall => f.write_str("VT-d install buffer too small"),
            Self::EmptyDomains => f.write_str("VT-d install requires at least one domain"),
            Self::InvalidPciBdf => f.write_str("invalid PCI BDF during VT-d install"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VtdPlanError {}
