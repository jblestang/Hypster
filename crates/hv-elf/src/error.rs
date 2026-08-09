//! ELF parsing error taxonomy.

use core::fmt;

/// Errors produced while parsing or validating an ELF64 image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElfError {
    /// Input buffer is shorter than the required minimum.
    BufferTooShort {
        /// Minimum bytes required for the operation.
        needed: usize,
        /// Bytes available in the input buffer.
        available: usize,
    },
    /// ELF magic bytes are missing or incorrect.
    InvalidMagic,
    /// ELF class is not 64-bit.
    UnsupportedClass {
        /// Value of `e_ident[EI_CLASS]`.
        found: u8,
    },
    /// Endianness is not little-endian.
    UnsupportedEndianness {
        /// Value of `e_ident[EI_DATA]`.
        found: u8,
    },
    /// Machine type is not x86-64.
    UnsupportedMachine {
        /// Value of `e_machine`.
        found: u16,
    },
    /// Program header entry size is not the ELF64 size.
    InvalidProgramHeaderEntrySize {
        /// Value of `e_phentsize`.
        found: u16,
    },
    /// Program header table lies outside the input buffer.
    ProgramHeaderTableOutOfBounds,
    /// A program header entry lies outside the input buffer.
    ProgramHeaderOutOfBounds {
        /// Zero-based program header index.
        index: usize,
    },
    /// A segment's file-backed range extends past the end of the input buffer.
    SegmentFileDataOutOfBounds {
        /// Zero-based program header index.
        index: usize,
    },
    /// Two loadable segments overlap in virtual address space.
    SegmentOverlap {
        /// Index of the first overlapping segment.
        first: usize,
        /// Index of the second overlapping segment.
        second: usize,
    },
    /// An address or size calculation overflowed.
    Overflow,
}

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooShort { needed, available } => {
                write!(f, "buffer too short: need {needed} bytes, have {available}")
            }
            Self::InvalidMagic => f.write_str("invalid ELF magic"),
            Self::UnsupportedClass { found } => write!(f, "unsupported ELF class: {found}"),
            Self::UnsupportedEndianness { found } => {
                write!(f, "unsupported ELF endianness: {found}")
            }
            Self::UnsupportedMachine { found } => {
                write!(f, "unsupported machine type: {found}")
            }
            Self::InvalidProgramHeaderEntrySize { found } => {
                write!(f, "invalid ELF64 program header entry size: {found}")
            }
            Self::ProgramHeaderTableOutOfBounds => {
                f.write_str("program header table out of bounds")
            }
            Self::ProgramHeaderOutOfBounds { index } => {
                write!(f, "program header {index} out of bounds")
            }
            Self::SegmentFileDataOutOfBounds { index } => {
                write!(f, "segment {index} file data out of bounds")
            }
            Self::SegmentOverlap { first, second } => {
                write!(f, "loadable segments {first} and {second} overlap")
            }
            Self::Overflow => f.write_str("arithmetic overflow"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ElfError {}
