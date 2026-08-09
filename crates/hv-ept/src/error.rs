//! EPT planner and installation errors.

use core::fmt;

/// Errors produced while building or installing EPT plans.
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
    /// Install buffer is too small for the requested page tables.
    BufferTooSmall,
    /// Mapping violates alignment requirements for the chosen page size.
    MisalignedMapping,
    /// Empty mapping list.
    EmptyMappings,
}

impl fmt::Display for EptPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingGuestRam { vm_id } => {
                write!(f, "missing guest RAM backing for vm{vm_id}")
            }
            Self::MappingOverlap { vm_id } => write!(f, "EPT mappings overlap for vm{vm_id}"),
            Self::Overflow => f.write_str("EPT planner overflow"),
            Self::BufferTooSmall => f.write_str("EPT install buffer too small"),
            Self::MisalignedMapping => f.write_str("EPT mapping misaligned"),
            Self::EmptyMappings => f.write_str("EPT install requires at least one mapping"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EptPlanError {}
