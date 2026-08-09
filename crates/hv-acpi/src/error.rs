//! ACPI parsing error taxonomy.

use core::fmt;

/// Errors produced while parsing ACPI tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcpiError {
    /// Input buffer is shorter than required.
    BufferTooShort {
        /// Minimum bytes needed for the operation.
        needed: usize,
        /// Bytes available in the input.
        available: usize,
    },
    /// Table or structure signature did not match.
    InvalidSignature {
        /// Expected ASCII signature.
        expected: &'static str,
        /// Signature bytes found in the input.
        found: [u8; 4],
    },
    /// ACPI checksum validation failed.
    InvalidChecksum,
    /// Declared table length is invalid or inconsistent.
    InvalidLength {
        /// Human-readable context, e.g. table name.
        context: &'static str,
        /// Declared length value.
        length: u32,
    },
    /// A nested structure within a table is malformed.
    InvalidStructure {
        /// Table or entry kind.
        kind: &'static str,
        /// Human-readable reason.
        reason: &'static str,
    },
    /// Table revision is not supported by this parser.
    UnsupportedRevision {
        /// Table name.
        table: &'static str,
        /// Revision byte from the header.
        revision: u8,
    },
}

impl fmt::Display for AcpiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooShort { needed, available } => {
                write!(f, "buffer too short: need {needed} bytes, have {available}")
            }
            Self::InvalidSignature { expected, found } => {
                write!(f, "invalid signature: expected {expected}, found {found:02x?}")
            }
            Self::InvalidChecksum => f.write_str("invalid ACPI checksum"),
            Self::InvalidLength { context, length } => {
                write!(f, "invalid {context} length: {length}")
            }
            Self::InvalidStructure { kind, reason } => {
                write!(f, "invalid {kind}: {reason}")
            }
            Self::UnsupportedRevision { table, revision } => {
                write!(f, "unsupported {table} revision: {revision}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AcpiError {}
