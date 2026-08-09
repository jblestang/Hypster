//! Validated platform contract outcome.

use crate::observed::ObservedPlatform;

/// Platform that satisfies the configured requirements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedPlatform {
    /// Configuration name.
    pub config_name: alloc::string::String,
    /// Observed platform that satisfied the contract.
    pub observed: ObservedPlatform,
}
