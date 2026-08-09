//! Partition builder errors.

use core::fmt;

/// Errors building guest partition boot info.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartitionError {
    /// Partition VM ID was not found in resolved platform plans.
    UnknownPartition {
        /// Requested VM identifier.
        vm_id: u32,
    },
    /// Guest boot info layout computation failed.
    Layout(hv_guest_abi::layout::GuestLayoutError),
    /// Buffer provided for serialization is too small.
    BufferTooSmall {
        /// Required bytes.
        required: usize,
        /// Provided bytes.
        provided: usize,
    },
    /// Integer overflow computing guest addresses.
    Overflow,
}

impl PartitionError {
    /// Returns a static diagnostic label.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::UnknownPartition { .. } => "unknown partition",
            Self::Layout(_) => "guest layout error",
            Self::BufferTooSmall { .. } => "guest boot info buffer too small",
            Self::Overflow => "partition overflow",
        }
    }
}

impl fmt::Display for PartitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<hv_guest_abi::layout::GuestLayoutError> for PartitionError {
    fn from(value: hv_guest_abi::layout::GuestLayoutError) -> Self {
        Self::Layout(value)
    }
}
