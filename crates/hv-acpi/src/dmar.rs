//! DMA Remapping (DMAR) table parser.

use alloc::vec::Vec;

use hv_types::{HostPhysAddr, PciSegment};

use crate::error::AcpiError;
use crate::table::{
    parse_sdt_header, read_u16_le, read_u64_le, read_u8, SDT_HEADER_LEN, SdtHeader,
};

/// DMAR signature.
pub const DMAR_SIGNATURE: [u8; 4] = *b"DMAR";

/// Fixed-length body prefix following the SDT header.
pub const DMAR_BODY_LEN: usize = 12;

/// DMA-remapping hardware definition structure type.
pub const DMAR_TYPE_DRHD: u16 = 0;

/// Minimum length of a DRHD structure.
pub const DRHD_ENTRY_LEN: u16 = 16;

/// DMAR flags: interrupt remapping is supported.
pub const DMAR_FLAG_INTR_REMAP: u8 = 1;

/// DRHD flags: all PCI devices behind this unit are included.
pub const DRHD_FLAG_INCLUDE_PCI_ALL: u8 = 1;

/// Summary of a DRHD remapping structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrhdEntry {
    /// PCI segment number for this remapping hardware unit.
    pub segment: PciSegment,
    /// MMIO base of the remapping hardware registers.
    pub register_base: HostPhysAddr,
    /// When set, all PCI devices in the segment are under this unit.
    pub include_all_pci: bool,
}

/// Parsed DMAR summary for platform observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmarSummary {
    /// SDT header for the DMAR table.
    pub header: SdtHeader,
    /// Host address width minus one (e.g. 39 means 40-bit addresses).
    pub host_address_width: u8,
    /// When set, the platform advertises interrupt remapping support.
    pub interrupt_remapping: bool,
    /// DRHD remapping hardware definitions.
    pub drhd_entries: Vec<DrhdEntry>,
}

impl DmarSummary {
    /// Returns true when at least one DRHD entry is present.
    #[must_use]
    pub fn has_drhd(&self) -> bool {
        !self.drhd_entries.is_empty()
    }
}

/// Parses a DMAR table from `data`.
///
/// DRHD structures are collected; other remapping structure types are skipped
/// using their declared lengths.
///
/// # Errors
///
/// Returns [`AcpiError`] when the header, length, checksum, or any structure
/// length is invalid.
pub fn parse_dmar(data: &[u8]) -> Result<DmarSummary, AcpiError> {
    let header = parse_sdt_header(data)?;
    if header.signature != DMAR_SIGNATURE {
        return Err(AcpiError::InvalidSignature {
            expected: "DMAR",
            found: header.signature,
        });
    }
    let table_len = header.length as usize;
    if table_len < SDT_HEADER_LEN + DMAR_BODY_LEN {
        return Err(AcpiError::InvalidLength {
            context: "DMAR",
            length: header.length,
        });
    }

    let host_address_width = read_u8(data, SDT_HEADER_LEN)?;
    let flags = read_u8(data, SDT_HEADER_LEN + 1)?;
    let interrupt_remapping = flags & DMAR_FLAG_INTR_REMAP != 0;

    let mut drhd_entries = Vec::new();
    let mut offset = SDT_HEADER_LEN + DMAR_BODY_LEN;

    while offset + 4 <= table_len {
        let structure_type = read_u16_le(data, offset)?;
        let structure_len = read_u16_le(data, offset + 2)?;
        if structure_len == 0 {
            return Err(AcpiError::InvalidStructure {
                kind: "DMAR structure",
                reason: "zero-length structure",
            });
        }
        let end = offset
            .checked_add(usize::from(structure_len))
            .ok_or(AcpiError::InvalidStructure {
                kind: "DMAR structure",
                reason: "structure length overflow",
            })?;
        if end > table_len {
            return Err(AcpiError::InvalidStructure {
                kind: "DMAR structure",
                reason: "structure extends past table end",
            });
        }

        if structure_type == DMAR_TYPE_DRHD {
            if structure_len < DRHD_ENTRY_LEN {
                return Err(AcpiError::InvalidStructure {
                    kind: "DRHD",
                    reason: "structure shorter than minimum",
                });
            }
            let drhd_flags = read_u8(data, offset + 4)?;
            let segment = PciSegment::new(read_u16_le(data, offset + 6)?);
            let register_base = HostPhysAddr::new(read_u64_le(data, offset + 8)?);
            drhd_entries.push(DrhdEntry {
                segment,
                register_base,
                include_all_pci: drhd_flags & DRHD_FLAG_INCLUDE_PCI_ALL != 0,
            });
        }

        offset = end;
    }

    Ok(DmarSummary {
        header,
        host_address_width,
        interrupt_remapping,
        drhd_entries,
    })
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::table::validate_checksum;

    fn dmar_header(total_len: u32) -> [u8; SDT_HEADER_LEN] {
        let mut header = [0u8; SDT_HEADER_LEN];
        header[..4].copy_from_slice(&DMAR_SIGNATURE);
        header[4..8].copy_from_slice(&total_len.to_le_bytes());
        header[8] = 1;
        let sum: u8 = header.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        header[9] = header[9].wrapping_sub(sum);
        header
    }

    #[test]
    fn parse_dmar_drhd_with_intr_remap_fixture() {
        let total_len = SDT_HEADER_LEN + DMAR_BODY_LEN + usize::from(DRHD_ENTRY_LEN);
        let mut table = vec![0u8; total_len];
        table[..SDT_HEADER_LEN].copy_from_slice(&dmar_header(total_len as u32));
        table[SDT_HEADER_LEN] = 39;
        table[SDT_HEADER_LEN + 1] = DMAR_FLAG_INTR_REMAP;

        let entry_off = SDT_HEADER_LEN + DMAR_BODY_LEN;
        table[entry_off..entry_off + 2].copy_from_slice(&DMAR_TYPE_DRHD.to_le_bytes());
        table[entry_off + 2..entry_off + 4].copy_from_slice(&DRHD_ENTRY_LEN.to_le_bytes());
        table[entry_off + 4] = DRHD_FLAG_INCLUDE_PCI_ALL;
        table[entry_off + 6..entry_off + 8].copy_from_slice(&1u16.to_le_bytes());
        table[entry_off + 8..entry_off + 16].copy_from_slice(&0xFED9_0000u64.to_le_bytes());

        let sum: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[9] = table[9].wrapping_sub(sum);
        validate_checksum(&table).expect("valid checksum");

        let dmar = parse_dmar(&table).expect("valid DMAR");
        assert_eq!(dmar.host_address_width, 39);
        assert!(dmar.interrupt_remapping);
        assert_eq!(dmar.drhd_entries.len(), 1);
        assert_eq!(dmar.drhd_entries[0].segment, PciSegment::new(1));
        assert_eq!(
            dmar.drhd_entries[0].register_base,
            HostPhysAddr::new(0xFED9_0000)
        );
        assert!(dmar.drhd_entries[0].include_all_pci);
    }

    #[test]
    fn parse_dmar_without_intr_remap_fixture() {
        let total_len = SDT_HEADER_LEN + DMAR_BODY_LEN;
        let mut table = vec![0u8; total_len];
        table[..SDT_HEADER_LEN].copy_from_slice(&dmar_header(total_len as u32));
        table[SDT_HEADER_LEN] = 39;
        table[SDT_HEADER_LEN + 1] = 0;

        let sum: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[9] = table[9].wrapping_sub(sum);
        validate_checksum(&table).expect("valid checksum");

        let dmar = parse_dmar(&table).expect("valid DMAR");
        assert!(!dmar.interrupt_remapping);
        assert!(dmar.drhd_entries.is_empty());
    }
}
