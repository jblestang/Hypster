//! CPU topology planner errors.

use core::fmt;

/// Errors produced by CPU topology planning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpuTopologyError {
    /// Not enough physical cores for configured partitions.
    InsufficientCores {
        /// Required exclusive cores.
        required: u32,
        /// Available exclusive cores.
        available: u32,
    },
    /// Partition requests more vCPUs than exclusive-core policy allows on one core.
    TooManyVcpusPerCore {
        /// Partition name.
        partition: alloc::string::String,
        /// Requested vCPUs.
        requested: u32,
    },
    /// SMT policy violation.
    SmtPolicyViolation {
        /// Explanation.
        reason: alloc::string::String,
    },
}

impl fmt::Display for CpuTopologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientCores { required, available } => {
                write!(f, "insufficient cores: need {required}, have {available}")
            }
            Self::TooManyVcpusPerCore {
                partition,
                requested,
            } => write!(
                f,
                "partition `{partition}` requests {requested} vCPUs under exclusive_core"
            ),
            Self::SmtPolicyViolation { reason } => write!(f, "SMT policy violation: {reason}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CpuTopologyError {}
