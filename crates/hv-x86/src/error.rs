//! x86 helper errors.

use core::fmt;

/// Physical memory access failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X86Error {
    /// Address is not identity-mapped in the current environment.
    NotMapped,
    /// Read would overflow the address space.
    Overflow,
}

impl fmt::Display for X86Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMapped => write!(f, "physical address not mapped"),
            Self::Overflow => write!(f, "physical read overflow"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for X86Error {}
