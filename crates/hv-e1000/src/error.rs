//! e1000 MMIO errors.

use core::fmt;

/// Errors decoding e1000 MMIO accesses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum E1000Error {
    /// Offset is not within the mapped BAR size.
    OffsetOutOfRange,
    /// Write to a read-only register.
    ReadOnlyRegister,
}

impl E1000Error {
    /// Returns a static diagnostic label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OffsetOutOfRange => "e1000 mmio offset out of range",
            Self::ReadOnlyRegister => "e1000 read-only register write",
        }
    }
}

impl fmt::Display for E1000Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
