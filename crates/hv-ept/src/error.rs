//! EPT planner errors.

use core::fmt;

/// Errors produced while building EPT plans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EptPlanError {
    /// Guest RAM backing region missing from memory plan.
    MissingGuestRam {
        /// VM identifier.
        vm_id: u32,
    },
    /// EPT mapping overlap within a partition.
    MappingOverlap {
        /// VM identifier.
        vm_id: u32,
    },
    /// Address overflow while building mappings.
    Overflow,
}

impl fmt::Display for EptPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingGuestRam { vm_id } => {
                write!(f, "missing guest RAM backing for vm{vm_id}")
            }
            Self::MappingOverlap { vm_id } => write!(f, "EPT mappings overlap for vm{vm_id}"),
            Self::Overflow => f.write_str("EPT planner overflow"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EptPlanError {}
