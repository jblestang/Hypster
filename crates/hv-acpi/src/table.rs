//! Shared ACPI table header and byte-slice helpers.

use crate::error::AcpiError;

/// Common ACPI System Description Table header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SdtHeader {
    /// Four-character table signature.
    pub signature: [u8; 4],
    /// Total table length in bytes, including this header.
    pub length: u32,
    /// ACPI specification revision of this table.
    pub revision: u8,
}

/// Size of the ACPI SDT header in bytes.
pub const SDT_HEADER_LEN: usize = 36;

/// Ensures `data` contains at least `needed` bytes.
pub fn ensure_len(data: &[u8], needed: usize) -> Result<(), AcpiError> {
    if data.len() < needed {
        return Err(AcpiError::BufferTooShort { needed, available: data.len() });
    }
    Ok(())
}

/// Reads a little-endian `u8` at `offset`.
pub fn read_u8(data: &[u8], offset: usize) -> Result<u8, AcpiError> {
    ensure_len(data, offset + 1)?;
    Ok(data[offset])
}

/// Reads a little-endian `u16` at `offset`.
pub fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, AcpiError> {
    ensure_len(data, offset + 2)?;
    let bytes = data
        .get(offset..offset + 2)
        .ok_or(AcpiError::BufferTooShort { needed: offset + 2, available: data.len() })?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Reads a little-endian `u32` at `offset`.
pub fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, AcpiError> {
    ensure_len(data, offset + 4)?;
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(AcpiError::BufferTooShort { needed: offset + 4, available: data.len() })?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Reads a little-endian `u64` at `offset`.
pub fn read_u64_le(data: &[u8], offset: usize) -> Result<u64, AcpiError> {
    ensure_len(data, offset + 8)?;
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(AcpiError::BufferTooShort { needed: offset + 8, available: data.len() })?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

/// Reads four signature bytes at `offset`.
pub fn read_signature(data: &[u8], offset: usize) -> Result<[u8; 4], AcpiError> {
    ensure_len(data, offset + 4)?;
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(AcpiError::BufferTooShort { needed: offset + 4, available: data.len() })?;
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Validates that bytes at `offset` match `expected`.
pub fn expect_signature(
    data: &[u8],
    offset: usize,
    expected: &'static str,
) -> Result<(), AcpiError> {
    let expected_bytes = expected.as_bytes();
    if expected_bytes.len() != 4 {
        return Err(AcpiError::InvalidStructure {
            kind: "signature",
            reason: "internal expected signature must be four bytes",
        });
    }
    let found = read_signature(data, offset)?;
    let mut expected_arr = [0u8; 4];
    expected_arr.copy_from_slice(expected_bytes);
    if found != expected_arr {
        return Err(AcpiError::InvalidSignature { expected, found });
    }
    Ok(())
}

/// Parses the common SDT header at the start of `data`.
pub fn parse_sdt_header(data: &[u8]) -> Result<SdtHeader, AcpiError> {
    ensure_len(data, SDT_HEADER_LEN)?;
    let signature = read_signature(data, 0)?;
    let length = read_u32_le(data, 4)?;
    if length as usize > data.len() || (length as usize) < SDT_HEADER_LEN {
        return Err(AcpiError::InvalidLength { context: "SDT header", length });
    }
    let revision = read_u8(data, 8)?;
    validate_checksum(&data[..length as usize])?;
    Ok(SdtHeader { signature, length, revision })
}

/// Validates that the sum of all bytes in `data` is zero modulo 256.
pub fn validate_checksum(data: &[u8]) -> Result<(), AcpiError> {
    let mut sum = 0u8;
    for &byte in data {
        sum = sum.wrapping_add(byte);
    }
    if sum == 0 {
        Ok(())
    } else {
        Err(AcpiError::InvalidChecksum)
    }
}
