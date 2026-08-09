//! Memory planner errors.

use core::fmt;

use hv_types::HostPhysAddr;

/// Errors produced while planning host memory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryPlanError {
    /// No suitable conventional memory region exists.
    NoConventionalMemory,
    /// Allocation exceeded available conventional memory.
    OutOfMemory {
        /// Bytes required.
        required: u64,
        /// Bytes available.
        available: u64,
    },
    /// Address arithmetic overflow.
    Overflow,
    /// Planned regions overlap.
    Overlap {
        /// First region purpose label.
        first: alloc::string::String,
        /// Second region purpose label.
        second: alloc::string::String,
    },
    /// Planned region is misaligned.
    Misaligned {
        /// Region purpose label.
        region: alloc::string::String,
        /// Observed base address.
        base: HostPhysAddr,
    },
}

impl fmt::Display for MemoryPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoConventionalMemory => f.write_str("no conventional memory available"),
            Self::OutOfMemory { required, available } => {
                write!(f, "out of memory: need {required} bytes, have {available}")
            }
            Self::Overflow => f.write_str("address overflow"),
            Self::Overlap { first, second } => {
                write!(f, "memory regions `{first}` and `{second}` overlap")
            }
            Self::Misaligned { region, base } => {
                write!(f, "region `{region}` misaligned at {base}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MemoryPlanError {}
