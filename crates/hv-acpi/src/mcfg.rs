//! PCI Express MCFG table parser.

use alloc::vec::Vec;

use hv_types::{HostPhysAddr, PciBus, PciSegment};

use crate::error::AcpiError;
use crate::table::{
    parse_sdt_header, read_u16_le, read_u64_le, read_u8, SDT_HEADER_LEN, SdtHeader,
};

/// MCFG signature.
pub const MCFG_SIGNATURE: [u8; 4] = *b"MCFG";

/// Reserved bytes following the SDT header.
pub const MCFG_RESERVED_LEN: usize = 8;

/// Length of each MCFG allocation structure.
pub const MCFG_ENTRY_LEN: usize = 44;

/// Summary of one MCFG allocation structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McfgEntry {
    /// ECAM base address for this segment and bus range.
    pub base_address: HostPhysAddr,
    /// PCI segment (domain) number.
    pub segment: PciSegment,
    /// First bus number covered by this ECAM window.
    pub start_bus: PciBus,
    /// Last bus number covered by this ECAM window.
    pub end_bus: PciBus,
}

impl McfgEntry {
    /// Returns the inclusive number of buses covered by this entry.
    #[must_use]
    pub fn bus_count(&self) -> u16 {
        u16::from(self.end_bus.raw())
            .wrapping_sub(u16::from(self.start_bus.raw()))
            .wrapping_add(1)
    }
}

/// Parsed MCFG summary for platform observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McfgSummary {
    /// SDT header for the MCFG table.
    pub header: SdtHeader,
    /// ECAM allocation entries.
    pub entries: Vec<McfgEntry>,
}

impl McfgSummary {
    /// Returns true when at least one ECAM allocation is present.
    #[must_use]
    pub fn has_ecam(&self) -> bool {
        !self.entries.is_empty()
    }
}

/// Parses an MCFG table from `data`.
///
/// # Errors
///
/// Returns [`AcpiError`] when the header, length, checksum, or any allocation
/// structure is invalid.
pub fn parse_mcfg(data: &[u8]) -> Result<McfgSummary, AcpiError> {
    let header = parse_sdt_header(data)?;
    if header.signature != MCFG_SIGNATURE {
        return Err(AcpiError::InvalidSignature {
            expected: "MCFG",
            found: header.signature,
        });
    }
    let table_len = header.length as usize;
    let min_len = SDT_HEADER_LEN + MCFG_RESERVED_LEN;
    if table_len < min_len {
        return Err(AcpiError::InvalidLength {
            context: "MCFG",
            length: header.length,
        });
    }

    let body_len = table_len - min_len;
    if body_len % MCFG_ENTRY_LEN != 0 {
        return Err(AcpiError::InvalidStructure {
            kind: "MCFG",
            reason: "allocation array length is not a multiple of 44",
        });
    }

    let mut entries = Vec::new();
    let mut offset = min_len;
    while offset + MCFG_ENTRY_LEN <= table_len {
        let base_address = HostPhysAddr::new(read_u64_le(data, offset)?);
        let segment = PciSegment::new(read_u16_le(data, offset + 8)?);
        let start_bus = PciBus::new(read_u8(data, offset + 10)?);
        let end_bus = PciBus::new(read_u8(data, offset + 11)?);
        if end_bus.raw() < start_bus.raw() {
            return Err(AcpiError::InvalidStructure {
                kind: "MCFG entry",
                reason: "end bus is less than start bus",
            });
        }
        entries.push(McfgEntry {
            base_address,
            segment,
            start_bus,
            end_bus,
        });
        offset += MCFG_ENTRY_LEN;
    }

    Ok(McfgSummary { header, entries })
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::table::validate_checksum;

    fn mcfg_header(total_len: u32) -> [u8; SDT_HEADER_LEN] {
        let mut header = [0u8; SDT_HEADER_LEN];
        header[..4].copy_from_slice(&MCFG_SIGNATURE);
        header[4..8].copy_from_slice(&total_len.to_le_bytes());
        header[8] = 1;
        let sum: u8 = header.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        header[9] = header[9].wrapping_sub(sum);
        header
    }

    #[test]
    fn parse_mcfg_fixture() {
        let total_len = SDT_HEADER_LEN + MCFG_RESERVED_LEN + MCFG_ENTRY_LEN;
        let mut table = vec![0u8; total_len];
        table[..SDT_HEADER_LEN].copy_from_slice(&mcfg_header(total_len as u32));

        let entry_off = SDT_HEADER_LEN + MCFG_RESERVED_LEN;
        table[entry_off..entry_off + 8].copy_from_slice(&0xE000_0000u64.to_le_bytes());
        table[entry_off + 8..entry_off + 10].copy_from_slice(&0u16.to_le_bytes());
        table[entry_off + 10] = 0;
        table[entry_off + 11] = 255;

        let sum: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[9] = table[9].wrapping_sub(sum);
        validate_checksum(&table).expect("valid checksum");

        let mcfg = parse_mcfg(&table).expect("valid MCFG");
        assert_eq!(mcfg.entries.len(), 1);
        assert_eq!(mcfg.entries[0].base_address, HostPhysAddr::new(0xE000_0000));
        assert_eq!(mcfg.entries[0].segment, PciSegment::new(0));
        assert_eq!(mcfg.entries[0].start_bus, PciBus::new(0));
        assert_eq!(mcfg.entries[0].end_bus, PciBus::new(255));
        assert_eq!(mcfg.entries[0].bus_count(), 256);
        assert!(mcfg.has_ecam());
    }
}
