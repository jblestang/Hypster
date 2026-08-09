//! Versioned boot ABI between the UEFI loader and hypervisor.
//!
//! This crate defines a stable, `no_std` layout for information passed from
//! `hv-loader.efi` to the hypervisor after `ExitBootServices`.
//!
//! # Versioning
//!
//! Any incompatible layout change must bump [`BOOT_ABI_VERSION_MAJOR`].
//! Compatible additions bump [`BOOT_ABI_VERSION_MINOR`].
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | Layout size/alignment | UNIT |
//! | Version mismatch handling | UNIT + REVIEW |

#![no_std]
#![warn(missing_docs)]

/// Major boot ABI version. Incompatible changes require a bump.
pub const BOOT_ABI_VERSION_MAJOR: u16 = 1;
/// Minor boot ABI version for backward-compatible additions.
pub const BOOT_ABI_VERSION_MINOR: u16 = 0;

/// Magic signature for [`BootInfoHeader`].
pub const BOOT_INFO_MAGIC: u64 = 0x1484_1484_1484_1484;

/// Header prefixed to every boot info blob.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootInfoHeader {
    /// Magic signature [`BOOT_INFO_MAGIC`].
    pub magic: u64,
    /// Major ABI version.
    pub version_major: u16,
    /// Minor ABI version.
    pub version_minor: u16,
    /// Total boot info size in bytes including this header.
    pub total_size: u32,
    /// SHA-256 hash of the validated configuration.
    pub config_hash: [u8; 32],
}

impl BootInfoHeader {
    /// Returns true when the header matches the compiled ABI version.
    #[must_use]
    pub const fn is_compatible(&self) -> bool {
        self.magic == BOOT_INFO_MAGIC && self.version_major == BOOT_ABI_VERSION_MAJOR
    }
}

/// Descriptor for a UEFI memory map entry copied into boot info.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootMemoryDescriptor {
    /// Descriptor type copied from UEFI.
    pub typ: u32,
    /// Physical start address.
    pub physical_start: u64,
    /// Number of bytes.
    pub number_of_bytes: u64,
    /// UEFI attribute bits.
    pub attribute: u64,
}

/// ACPI RSDP pointer handed to the hypervisor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootAcpiInfo {
    /// Physical address of the RSDP.
    pub rsdp_address: u64,
}

/// Fixed-layout boot info prefix.
///
/// Variable-length tables (memory map, image list) follow this struct in the
/// boot info blob. Offsets are resolved by the loader in later phases.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootInfo {
    /// Header with versioning and config hash.
    pub header: BootInfoHeader,
    /// ACPI discovery information.
    pub acpi: BootAcpiInfo,
    /// Number of memory map entries following the fixed prefix.
    pub memory_map_entry_count: u32,
    /// Reserved for future flags.
    pub flags: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_info_header_size_is_stable() {
        assert_eq!(core::mem::size_of::<BootInfoHeader>(), 48);
        assert_eq!(core::mem::align_of::<BootInfoHeader>(), 8);
    }

    #[test]
    fn compatible_header_matches_current_major() {
        let header = BootInfoHeader {
            magic: BOOT_INFO_MAGIC,
            version_major: BOOT_ABI_VERSION_MAJOR,
            version_minor: BOOT_ABI_VERSION_MINOR,
            total_size: core::mem::size_of::<BootInfoHeader>() as u32,
            config_hash: [0; 32],
        };
        assert!(header.is_compatible());
    }
}
