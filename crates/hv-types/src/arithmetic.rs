//! Overflow-safe arithmetic helpers for address and size calculations.

use core::fmt;

/// Errors produced by checked arithmetic and alignment helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArithmeticError {
    /// Addition overflowed.
    Overflow,
    /// Subtraction underflowed.
    Underflow,
    /// Multiplication overflowed.
    MultiplyOverflow,
    /// Alignment is zero or not a power of two.
    InvalidAlignment,
    /// Address is not aligned to the required boundary.
    Misaligned,
    /// PCI BDF string could not be parsed.
    InvalidPciBdf,
    /// PCI device number is out of range.
    InvalidPciDevice,
    /// PCI function number is out of range.
    InvalidPciFunction,
}

impl fmt::Display for ArithmeticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => f.write_str("overflow"),
            Self::Underflow => f.write_str("underflow"),
            Self::MultiplyOverflow => f.write_str("multiply overflow"),
            Self::InvalidAlignment => f.write_str("invalid alignment"),
            Self::Misaligned => f.write_str("misaligned address"),
            Self::InvalidPciBdf => f.write_str("invalid PCI BDF"),
            Self::InvalidPciDevice => f.write_str("invalid PCI device number"),
            Self::InvalidPciFunction => f.write_str("invalid PCI function number"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ArithmeticError {}

/// Adds two `u64` values with overflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::Overflow`] when the sum exceeds `u64::MAX`.
pub const fn checked_add_u64(a: u64, b: u64) -> Result<u64, ArithmeticError> {
    match a.checked_add(b) {
        Some(v) => Ok(v),
        None => Err(ArithmeticError::Overflow),
    }
}

/// Subtracts two `u64` values with underflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::Underflow`] when `b` is greater than `a`.
pub const fn checked_sub_u64(a: u64, b: u64) -> Result<u64, ArithmeticError> {
    match a.checked_sub(b) {
        Some(v) => Ok(v),
        None => Err(ArithmeticError::Underflow),
    }
}

/// Multiplies two `u64` values with overflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::MultiplyOverflow`] when the product exceeds `u64::MAX`.
pub const fn checked_mul_u64(a: u64, b: u64) -> Result<u64, ArithmeticError> {
    match a.checked_mul(b) {
        Some(v) => Ok(v),
        None => Err(ArithmeticError::MultiplyOverflow),
    }
}

/// Adds two `usize` values with overflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::Overflow`] when the sum exceeds `usize::MAX`.
pub const fn checked_add_usize(a: usize, b: usize) -> Result<usize, ArithmeticError> {
    match a.checked_add(b) {
        Some(v) => Ok(v),
        None => Err(ArithmeticError::Overflow),
    }
}

/// Subtracts two `usize` values with underflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::Underflow`] when `b` is greater than `a`.
pub const fn checked_sub_usize(a: usize, b: usize) -> Result<usize, ArithmeticError> {
    match a.checked_sub(b) {
        Some(v) => Ok(v),
        None => Err(ArithmeticError::Underflow),
    }
}

/// Multiplies two `usize` values with overflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::MultiplyOverflow`] when the product exceeds `usize::MAX`.
pub const fn checked_mul_usize(a: usize, b: usize) -> Result<usize, ArithmeticError> {
    match a.checked_mul(b) {
        Some(v) => Ok(v),
        None => Err(ArithmeticError::MultiplyOverflow),
    }
}

/// Returns `value` rounded up to `alignment` if `alignment` is a power of two.
///
/// # Errors
///
/// Returns [`ArithmeticError::InvalidAlignment`] when `alignment` is zero or not a
/// power of two, or [`ArithmeticError::Overflow`] when rounding overflows.
pub const fn align_up_u64(value: u64, alignment: u64) -> Result<u64, ArithmeticError> {
    if alignment == 0 || alignment & (alignment - 1) != 0 {
        return Err(ArithmeticError::InvalidAlignment);
    }
    let mask = alignment - 1;
    match value.checked_add(mask) {
        Some(v) => Ok(v & !mask),
        None => Err(ArithmeticError::Overflow),
    }
}

/// Returns true when `value` is aligned to `alignment`.
///
/// # Errors
///
/// Returns [`ArithmeticError::InvalidAlignment`] when `alignment` is zero or not a
/// power of two.
pub const fn is_aligned_u64(value: u64, alignment: u64) -> Result<bool, ArithmeticError> {
    if alignment == 0 || alignment & (alignment - 1) != 0 {
        return Err(ArithmeticError::InvalidAlignment);
    }
    Ok(value & (alignment - 1) == 0)
}

/// Converts gibibytes to bytes with overflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::MultiplyOverflow`] when the conversion overflows.
pub const fn gib_to_bytes(gib: u64) -> Result<u64, ArithmeticError> {
    checked_mul_u64(gib, 1024 * 1024 * 1024)
}

/// Converts mebibytes to bytes with overflow checking.
///
/// # Errors
///
/// Returns [`ArithmeticError::MultiplyOverflow`] when the conversion overflows.
pub const fn mib_to_bytes(mib: u64) -> Result<u64, ArithmeticError> {
    checked_mul_u64(mib, 1024 * 1024)
}

/// Computes whether `[start, start + size)` overlaps another range.
#[must_use]
pub const fn ranges_overlap_u64(a_start: u64, a_size: u64, b_start: u64, b_size: u64) -> bool {
    let Some(a_end) = a_start.checked_add(a_size) else {
        return true;
    };
    let Some(b_end) = b_start.checked_add(b_size) else {
        return true;
    };
    a_start < b_end && b_start < a_end
}
