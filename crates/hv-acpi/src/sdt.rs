//! Root and Extended System Description Table parsers.

use alloc::vec::Vec;

use hv_types::HostPhysAddr;

use crate::error::AcpiError;
use crate::table::{parse_sdt_header, read_u32_le, read_u64_le, SDT_HEADER_LEN, SdtHeader};

/// RSDT signature.
pub const RSDT_SIGNATURE: [u8; 4] = *b"RSDT";
/// XSDT signature.
pub const XSDT_SIGNATURE: [u8; 4] = *b"XSDT";

/// Parsed Root System Description Table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rsdt {
    /// SDT header for the RSDT.
    pub header: SdtHeader,
    /// Physical addresses of tables referenced by the RSDT.
    pub entries: Vec<HostPhysAddr>,
}

/// Parsed Extended System Description Table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Xsdt {
    /// SDT header for the XSDT.
    pub header: SdtHeader,
    /// Physical addresses of tables referenced by the XSDT.
    pub entries: Vec<HostPhysAddr>,
}

/// Parses an RSDT from `data`.
///
/// # Errors
///
/// Returns [`AcpiError`] when the header, length, or checksum is invalid.
pub fn parse_rsdt(data: &[u8]) -> Result<Rsdt, AcpiError> {
    let header = parse_sdt_header(data)?;
    if header.signature != RSDT_SIGNATURE {
        return Err(AcpiError::InvalidSignature {
            expected: "RSDT",
            found: header.signature,
        });
    }
    let table_len = header.length as usize;
    let body = table_len.saturating_sub(SDT_HEADER_LEN);
    if body % 4 != 0 {
        return Err(AcpiError::InvalidStructure {
            kind: "RSDT",
            reason: "entry array length is not a multiple of four",
        });
    }
    let mut entries = Vec::new();
    let mut offset = SDT_HEADER_LEN;
    while offset + 4 <= table_len {
        let pointer = read_u32_le(data, offset)?;
        if pointer != 0 {
            entries.push(HostPhysAddr::new(u64::from(pointer)));
        }
        offset += 4;
    }
    Ok(Rsdt { header, entries })
}

/// Parses an XSDT from `data`.
///
/// # Errors
///
/// Returns [`AcpiError`] when the header, length, or checksum is invalid.
pub fn parse_xsdt(data: &[u8]) -> Result<Xsdt, AcpiError> {
    let header = parse_sdt_header(data)?;
    if header.signature != XSDT_SIGNATURE {
        return Err(AcpiError::InvalidSignature {
            expected: "XSDT",
            found: header.signature,
        });
    }
    let table_len = header.length as usize;
    let body = table_len.saturating_sub(SDT_HEADER_LEN);
    if body % 8 != 0 {
        return Err(AcpiError::InvalidStructure {
            kind: "XSDT",
            reason: "entry array length is not a multiple of eight",
        });
    }
    let mut entries = Vec::new();
    let mut offset = SDT_HEADER_LEN;
    while offset + 8 <= table_len {
        let pointer = read_u64_le(data, offset)?;
        if pointer != 0 {
            entries.push(HostPhysAddr::new(pointer));
        }
        offset += 8;
    }
    Ok(Xsdt { header, entries })
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::table::validate_checksum;

    fn sdt_header(signature: [u8; 4], total_len: u32) -> [u8; SDT_HEADER_LEN] {
        let mut header = [0u8; SDT_HEADER_LEN];
        header[..4].copy_from_slice(&signature);
        header[4..8].copy_from_slice(&total_len.to_le_bytes());
        header[8] = 1;
        let sum: u8 = header.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        header[9] = header[9].wrapping_sub(sum);
        header
    }

    fn fix_table_checksum(table: &mut [u8]) {
        table[9] = 0;
        let sum: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[9] = table[9].wrapping_sub(sum);
    }

    #[test]
    fn parse_rsdt_fixture() {
        let total_len = SDT_HEADER_LEN + 8;
        let mut table = vec![0u8; total_len];
        table[..SDT_HEADER_LEN].copy_from_slice(&sdt_header(RSDT_SIGNATURE, total_len as u32));
        table[SDT_HEADER_LEN..SDT_HEADER_LEN + 4].copy_from_slice(&0x3000u32.to_le_bytes());
        table[SDT_HEADER_LEN + 4..SDT_HEADER_LEN + 8].copy_from_slice(&0x4000u32.to_le_bytes());
        fix_table_checksum(&mut table);
        validate_checksum(&table).expect("valid checksum");

        let rsdt = parse_rsdt(&table).expect("valid RSDT");
        assert_eq!(rsdt.entries.len(), 2);
        assert_eq!(rsdt.entries[0], HostPhysAddr::new(0x3000));
        assert_eq!(rsdt.entries[1], HostPhysAddr::new(0x4000));
    }

    #[test]
    fn parse_xsdt_fixture() {
        let total_len = SDT_HEADER_LEN + 16;
        let mut table = vec![0u8; total_len];
        table[..SDT_HEADER_LEN].copy_from_slice(&sdt_header(XSDT_SIGNATURE, total_len as u32));
        table[SDT_HEADER_LEN..SDT_HEADER_LEN + 8].copy_from_slice(&0x5000u64.to_le_bytes());
        table[SDT_HEADER_LEN + 8..SDT_HEADER_LEN + 16]
            .copy_from_slice(&0x6000u64.to_le_bytes());
        fix_table_checksum(&mut table);
        validate_checksum(&table).expect("valid checksum");

        let xsdt = parse_xsdt(&table).expect("valid XSDT");
        assert_eq!(xsdt.entries.len(), 2);
        assert_eq!(xsdt.entries[0], HostPhysAddr::new(0x5000));
        assert_eq!(xsdt.entries[1], HostPhysAddr::new(0x6000));
    }
}
