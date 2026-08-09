//! x86 helper errors.

use core::fmt;

/// Errors returned by x86 intrinsics wrappers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X86Error {
    /// The current target architecture is not x86_64.
    UnsupportedArchitecture,
}

impl fmt::Display for X86Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArchitecture => f.write_str("x86_64 intrinsics are unavailable"),
        }
    }
}
