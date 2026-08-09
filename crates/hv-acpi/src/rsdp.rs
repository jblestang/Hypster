//! Root System Description Pointer (RSDP) parser.

use hv_types::HostPhysAddr;

use crate::error::AcpiError;
use crate::table::{ensure_len, read_u32_le, read_u64_le, read_u8, validate_checksum};

/// ACPI RSDP signature (`"RSD PTR "`).
pub const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";

/// Minimum RSDP length for ACPI 1.0.
pub const RSDP_V1_LEN: usize = 20;
/// Minimum RSDP length for ACPI 2.0 and later.
pub const RSDP_V2_LEN: usize = 36;

/// Parsed Root System Description Pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rsdp {
    /// ACPI revision byte from the RSDP (0 for ACPI 1.0, 2+ for ACPI 2.0).
    pub revision: u8,
    /// Physical address of the RSDT when present (ACPI 1.0 field).
    pub rsdt_address: Option<HostPhysAddr>,
    /// Physical address of the XSDT when present (ACPI 2.0+ field).
    pub xsdt_address: Option<HostPhysAddr>,
}

impl Rsdp {
    /// Returns the preferred root table address (XSDT when available, else RSDT).
    #[must_use]
    pub const fn root_table_address(&self) -> Option<HostPhysAddr> {
        if let Some(xsdt) = self.xsdt_address {
            Some(xsdt)
        } else {
            self.rsdt_address
        }
    }
}

/// Parses an RSDP structure from `data`.
///
/// # Errors
///
/// Returns [`AcpiError`] when the buffer is too short, the signature is invalid,
/// or either required checksum fails.
pub fn parse_rsdp(data: &[u8]) -> Result<Rsdp, AcpiError> {
    ensure_len(data, RSDP_V1_LEN)?;
    for (idx, &expected) in RSDP_SIGNATURE.iter().enumerate() {
        let found = read_u8(data, idx)?;
        if found != expected {
            return Err(AcpiError::InvalidSignature {
                expected: "RSD PTR ",
                found: [
                    read_u8(data, 0)?,
                    read_u8(data, 1)?,
                    read_u8(data, 2)?,
                    read_u8(data, 3)?,
                ],
            });
        }
    }

    validate_checksum(&data[..RSDP_V1_LEN])?;

    let revision = read_u8(data, 15)?;
    let rsdt_phys = read_u32_le(data, 16)?;
    let rsdt_address = if rsdt_phys == 0 {
        None
    } else {
        Some(HostPhysAddr::new(u64::from(rsdt_phys)))
    };

    if revision >= 2 {
        ensure_len(data, RSDP_V2_LEN)?;
        let length = read_u32_le(data, 20)?;
        if length as usize != RSDP_V2_LEN {
            return Err(AcpiError::InvalidLength {
                context: "RSDP",
                length,
            });
        }
        validate_checksum(&data[..RSDP_V2_LEN])?;
        let xsdt_phys = read_u64_le(data, 24)?;
        let xsdt_address = if xsdt_phys == 0 {
            None
        } else {
            Some(HostPhysAddr::new(xsdt_phys))
        };
        Ok(Rsdp {
            revision,
            rsdt_address,
            xsdt_address,
        })
    } else {
        Ok(Rsdp {
            revision,
            rsdt_address,
            xsdt_address: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_checksum(mut table: [u8; RSDP_V2_LEN]) -> [u8; RSDP_V2_LEN] {
        let sum_v1: u8 = table[..RSDP_V1_LEN].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[8] = table[8].wrapping_sub(sum_v1);
        let sum_v2: u8 = table.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        table[32] = table[32].wrapping_sub(sum_v2);
        table
    }

    #[test]
    fn parse_rsdp_v2_fixture() {
        let mut raw = [0u8; RSDP_V2_LEN];
        raw[..8].copy_from_slice(RSDP_SIGNATURE);
        raw[15] = 2;
        raw[16..20].copy_from_slice(&0x1000u32.to_le_bytes());
        raw[20..24].copy_from_slice(&(RSDP_V2_LEN as u32).to_le_bytes());
        raw[24..32].copy_from_slice(&0x2000u64.to_le_bytes());
        let table = with_checksum(raw);

        let rsdp = parse_rsdp(&table).expect("valid RSDP fixture");
        assert_eq!(rsdp.revision, 2);
        assert_eq!(
            rsdp.rsdt_address,
            Some(HostPhysAddr::new(0x1000))
        );
        assert_eq!(
            rsdp.xsdt_address,
            Some(HostPhysAddr::new(0x2000))
        );
        assert_eq!(
            rsdp.root_table_address(),
            Some(HostPhysAddr::new(0x2000))
        );
    }

    #[test]
    fn parse_rsdp_rejects_bad_signature() {
        let mut raw = [0u8; RSDP_V2_LEN];
        raw[15] = 2;
        raw[20..24].copy_from_slice(&(RSDP_V2_LEN as u32).to_le_bytes());
        let table = with_checksum(raw);
        let err = parse_rsdp(&table).expect_err("bad signature");
        assert!(matches!(err, AcpiError::InvalidSignature { .. }));
    }
}
