//! Multiple APIC Description Table (MADT) parser.

use alloc::vec::Vec;

use hv_types::{ApicId, HostPhysAddr};

use crate::error::AcpiError;
use crate::table::{parse_sdt_header, read_u32_le, read_u8, SdtHeader, SDT_HEADER_LEN};

/// MADT signature.
pub const MADT_SIGNATURE: [u8; 4] = *b"APIC";

/// Fixed-length body prefix following the SDT header.
pub const MADT_BODY_LEN: usize = 8;

/// Processor Local APIC structure type.
pub const MADT_TYPE_LOCAL_APIC: u8 = 0;
/// Processor Local x2APIC structure type.
pub const MADT_TYPE_LOCAL_X2APIC: u8 = 9;

/// Minimum length of a Processor Local APIC structure.
pub const LOCAL_APIC_ENTRY_LEN: u8 = 8;
/// Minimum length of a Processor Local x2APIC structure.
pub const LOCAL_X2APIC_ENTRY_LEN: u8 = 16;

/// Local APIC flags: processor is enabled.
pub const LOCAL_APIC_ENABLED: u32 = 1;

/// Summary of a Processor Local APIC entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalApicEntry {
    /// ACPI processor UID.
    pub processor_uid: u8,
    /// Local APIC ID.
    pub apic_id: ApicId,
    /// Whether the processor is enabled in ACPI.
    pub enabled: bool,
}

/// Summary of a Processor Local x2APIC entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X2ApicEntry {
    /// ACPI processor UID.
    pub processor_uid: u32,
    /// x2APIC ID.
    pub apic_id: ApicId,
    /// Whether the processor is enabled in ACPI.
    pub enabled: bool,
}

/// Parsed MADT summary for platform observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MadtSummary {
    /// SDT header for the MADT.
    pub header: SdtHeader,
    /// Local APIC MMIO base address.
    pub local_apic_address: HostPhysAddr,
    /// When set, the platform also has dual 8259 PICs.
    pub pc_at_compatible: bool,
    /// Enabled and disabled local APIC processor entries.
    pub local_apics: Vec<LocalApicEntry>,
    /// Enabled and disabled x2APIC processor entries.
    pub x2_apics: Vec<X2ApicEntry>,
}

impl MadtSummary {
    /// Returns the count of enabled local and x2APIC processors.
    #[must_use]
    pub fn enabled_processor_count(&self) -> usize {
        let local = self.local_apics.iter().filter(|e| e.enabled).count();
        let x2 = self.x2_apics.iter().filter(|e| e.enabled).count();
        local + x2
    }

    /// Returns true when any x2APIC entries are present.
    #[must_use]
    pub fn has_x2apic(&self) -> bool {
        !self.x2_apics.is_empty()
    }
}

/// Parses a MADT from `data`.
///
/// Local APIC and x2APIC entries are collected; other interrupt controller
/// structures are skipped using their declared lengths.
///
/// # Errors
///
/// Returns [`AcpiError`] when the header, length, checksum, or any structure
/// length is invalid.
pub fn parse_madt(data: &[u8]) -> Result<MadtSummary, AcpiError> {
    let header = parse_sdt_header(data)?;
    if header.signature != MADT_SIGNATURE {
        return Err(AcpiError::InvalidSignature { expected: "APIC", found: header.signature });
    }
    let table_len = header.length as usize;
    if table_len < SDT_HEADER_LEN + MADT_BODY_LEN {
        return Err(AcpiError::InvalidLength { context: "MADT", length: header.length });
    }

    let local_apic_phys = read_u32_le(data, SDT_HEADER_LEN)?;
    let flags = read_u32_le(data, SDT_HEADER_LEN + 4)?;
    let local_apic_address = HostPhysAddr::new(u64::from(local_apic_phys));
    let pc_at_compatible = flags & 1 != 0;

    let mut local_apics = Vec::new();
    let mut x2_apics = Vec::new();
    let mut offset = SDT_HEADER_LEN + MADT_BODY_LEN;

    while offset < table_len {
        let entry_type = read_u8(data, offset)?;
        let entry_len = read_u8(data, offset + 1)?;
        if entry_len == 0 {
            return Err(AcpiError::InvalidStructure {
                kind: "MADT entry",
                reason: "zero-length structure",
            });
        }
        let end =
            offset.checked_add(usize::from(entry_len)).ok_or(AcpiError::InvalidStructure {
                kind: "MADT entry",
                reason: "structure length overflow",
            })?;
        if end > table_len {
            return Err(AcpiError::InvalidStructure {
                kind: "MADT entry",
                reason: "structure extends past table end",
            });
        }

        match entry_type {
            MADT_TYPE_LOCAL_APIC => {
                if entry_len < LOCAL_APIC_ENTRY_LEN {
                    return Err(AcpiError::InvalidStructure {
                        kind: "Local APIC",
                        reason: "structure shorter than minimum",
                    });
                }
                let processor_uid = read_u8(data, offset + 2)?;
                let apic_id = ApicId::new(u32::from(read_u8(data, offset + 3)?));
                let entry_flags = read_u32_le(data, offset + 4)?;
                local_apics.push(LocalApicEntry {
                    processor_uid,
                    apic_id,
                    enabled: entry_flags & LOCAL_APIC_ENABLED != 0,
                });
            }
            MADT_TYPE_LOCAL_X2APIC => {
                if entry_len < LOCAL_X2APIC_ENTRY_LEN {
                    return Err(AcpiError::InvalidStructure {
                        kind: "Local x2APIC",
                        reason: "structure shorter than minimum",
                    });
                }
                let apic_id = ApicId::new(read_u32_le(data, offset + 4)?);
                let entry_flags = read_u32_le(data, offset + 8)?;
                let processor_uid = read_u32_le(data, offset + 12)?;
                x2_apics.push(X2ApicEntry {
                    processor_uid,
                    apic_id,
                    enabled: entry_flags & LOCAL_APIC_ENABLED != 0,
                });
            }
            _ => {}
        }

        offset = end;
    }

    Ok(MadtSummary { header, local_apic_address, pc_at_compatible, local_apics, x2_apics })
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::table::validate_checksum;

    fn madt_header(total_len: u32) -> [u8; SDT_HEADER_LEN] {
        let mut header = [0u8; SDT_HEADER_LEN];
        header[..4].copy_from_slice(&MADT_SIGNATURE);
        header[4..8].copy_from_slice(&total_len.to_le_bytes());
        header[8] = 1;
        let sum: u8 = header.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        header[9] = header[9].wrapping_sub(sum);
        header
    }

    #[test]
    fn parse_madt_local_apic_fixture() {
        let total_len = SDT_HEADER_LEN + MADT_BODY_LEN + usize::from(LOCAL_APIC_ENTRY_LEN);
        let mut table = vec![0u8; total_len];
        table[..SDT_HEADER_LEN].copy_from_slice(&madt_header(total_len as u32));
        table[SDT_HEADER_LEN..SDT_HEADER_LEN + 4].copy_from_slice(&0xFEE0_0000u32.to_le_bytes());

        let entry_off = SDT_HEADER_LEN + MADT_BODY_LEN;
        table[entry_off] = MADT_TYPE_LOCAL_APIC;
        table[entry_off + 1] = LOCAL_APIC_ENTRY_LEN;
        table[entry_off + 2] = 1;
        table[entry_off + 3] = 4;
        table[entry_off + 4..entry_off + 8].copy_from_slice(&LOCAL_APIC_ENABLED.to_le_bytes());

        let sum: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[9] = table[9].wrapping_sub(sum);
        validate_checksum(&table).expect("valid checksum");

        let madt = parse_madt(&table).expect("valid MADT");
        assert_eq!(madt.local_apic_address, HostPhysAddr::new(0xFEE0_0000));
        assert!(!madt.pc_at_compatible);
        assert_eq!(madt.local_apics.len(), 1);
        assert_eq!(madt.local_apics[0].processor_uid, 1);
        assert_eq!(madt.local_apics[0].apic_id, ApicId::new(4));
        assert!(madt.local_apics[0].enabled);
        assert!(madt.x2_apics.is_empty());
        assert_eq!(madt.enabled_processor_count(), 1);
    }

    #[test]
    fn parse_madt_x2apic_fixture() {
        let total_len = SDT_HEADER_LEN + MADT_BODY_LEN + usize::from(LOCAL_X2APIC_ENTRY_LEN);
        let mut table = vec![0u8; total_len];
        table[..SDT_HEADER_LEN].copy_from_slice(&madt_header(total_len as u32));
        table[SDT_HEADER_LEN..SDT_HEADER_LEN + 4].copy_from_slice(&0xFEE0_0000u32.to_le_bytes());

        let entry_off = SDT_HEADER_LEN + MADT_BODY_LEN;
        table[entry_off] = MADT_TYPE_LOCAL_X2APIC;
        table[entry_off + 1] = LOCAL_X2APIC_ENTRY_LEN;
        table[entry_off + 4..entry_off + 8].copy_from_slice(&256u32.to_le_bytes());
        table[entry_off + 8..entry_off + 12].copy_from_slice(&LOCAL_APIC_ENABLED.to_le_bytes());
        table[entry_off + 12..entry_off + 16].copy_from_slice(&2u32.to_le_bytes());

        let sum: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[9] = table[9].wrapping_sub(sum);
        validate_checksum(&table).expect("valid checksum");

        let madt = parse_madt(&table).expect("valid MADT");
        assert!(madt.has_x2apic());
        assert_eq!(madt.x2_apics.len(), 1);
        assert_eq!(madt.x2_apics[0].processor_uid, 2);
        assert_eq!(madt.x2_apics[0].apic_id, ApicId::new(256));
        assert!(madt.x2_apics[0].enabled);
    }
}
