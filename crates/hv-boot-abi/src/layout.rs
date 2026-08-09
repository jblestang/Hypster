//! Boot info blob layout helpers.

use core::mem::{align_of, size_of};

use crate::{BootInfo, BootMemoryDescriptor};

/// Fixed prefix size in bytes (the [`BootInfo`] struct).
pub const BOOT_INFO_FIXED_PREFIX_BYTES: usize = size_of::<BootInfo>();

/// Errors validating a boot info blob layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootLayoutError {
    /// Pointer is null.
    NullPointer,
    /// Total byte count is smaller than the fixed prefix.
    BufferTooSmall,
    /// `boot_info_bytes` does not match the provided total size.
    SizeMismatch {
        /// Declared size in the struct.
        declared: u32,
        /// Caller-provided total size.
        provided: usize,
    },
    /// Header `total_size` does not match `boot_info_bytes`.
    HeaderTotalSizeMismatch {
        /// Header field.
        header_total: u32,
        /// Struct field.
        boot_info_bytes: u32,
    },
    /// Memory map entry count overflows or extends past the buffer.
    MemoryMapOutOfBounds {
        /// Entry count from boot info.
        entry_count: u32,
        /// Required bytes for the full blob.
        required: usize,
        /// Provided buffer size.
        provided: usize,
    },
    /// Magic or major version is incompatible.
    IncompatibleHeader,
    /// Descriptor slice would be misaligned.
    MisalignedMemoryMap,
}

impl BootLayoutError {
    /// Returns a static label for diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NullPointer => "null boot info pointer",
            Self::BufferTooSmall => "boot info buffer too small",
            Self::SizeMismatch { .. } => "boot_info_bytes mismatch",
            Self::HeaderTotalSizeMismatch { .. } => "header total_size mismatch",
            Self::MemoryMapOutOfBounds { .. } => "memory map out of bounds",
            Self::IncompatibleHeader => "incompatible boot info header",
            Self::MisalignedMemoryMap => "misaligned memory map slice",
        }
    }
}

/// Returns a slice of memory map descriptors immediately following the fixed prefix.
///
/// # Safety
///
/// `info` must point to a valid boot info blob whose trailing descriptors are
/// initialized for `info.memory_map_entry_count` entries.
pub unsafe fn memory_map_slice(info: &BootInfo) -> &[BootMemoryDescriptor] {
    let desc_ptr = (info as *const BootInfo)
        .cast::<u8>()
        .add(BOOT_INFO_FIXED_PREFIX_BYTES) as *const BootMemoryDescriptor;
    core::slice::from_raw_parts(desc_ptr, info.memory_map_entry_count as usize)
}

/// Validates boot info layout against the caller-provided total byte count.
///
/// # Safety
///
/// `info` must refer to at least `total_bytes` readable bytes.
pub unsafe fn validate_boot_info_layout(
    info: *const BootInfo,
    total_bytes: usize,
) -> Result<(), BootLayoutError> {
    if info.is_null() {
        return Err(BootLayoutError::NullPointer);
    }
    if total_bytes < BOOT_INFO_FIXED_PREFIX_BYTES {
        return Err(BootLayoutError::BufferTooSmall);
    }

    let info = unsafe { &*info };

    if !info.header.is_compatible() {
        return Err(BootLayoutError::IncompatibleHeader);
    }

    let declared = info.boot_info_bytes as usize;
    if declared != total_bytes {
        return Err(BootLayoutError::SizeMismatch {
            declared: info.boot_info_bytes,
            provided: total_bytes,
        });
    }

    if info.header.total_size != info.boot_info_bytes {
        return Err(BootLayoutError::HeaderTotalSizeMismatch {
            header_total: info.header.total_size,
            boot_info_bytes: info.boot_info_bytes,
        });
    }

    let desc_bytes = info
        .memory_map_entry_count
        .checked_mul(size_of::<BootMemoryDescriptor>() as u32)
        .ok_or(BootLayoutError::MemoryMapOutOfBounds {
            entry_count: info.memory_map_entry_count,
            required: usize::MAX,
            provided: total_bytes,
        })? as usize;

    let required = BOOT_INFO_FIXED_PREFIX_BYTES
        .checked_add(desc_bytes)
        .ok_or(BootLayoutError::MemoryMapOutOfBounds {
            entry_count: info.memory_map_entry_count,
            required: usize::MAX,
            provided: total_bytes,
        })?;

    if required != total_bytes {
        return Err(BootLayoutError::MemoryMapOutOfBounds {
            entry_count: info.memory_map_entry_count,
            required,
            provided: total_bytes,
        });
    }

    let map_offset = BOOT_INFO_FIXED_PREFIX_BYTES;
    if map_offset % align_of::<BootMemoryDescriptor>() != 0 {
        return Err(BootLayoutError::MisalignedMemoryMap);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BootAcpiInfo, BootInfoHeader, BOOT_ABI_VERSION_MAJOR, BOOT_ABI_VERSION_MINOR, BOOT_INFO_MAGIC};

    fn sample_info(entry_count: u32) -> (BootInfo, usize) {
        let desc_bytes = entry_count as usize * size_of::<BootMemoryDescriptor>();
        let total = BOOT_INFO_FIXED_PREFIX_BYTES + desc_bytes;
        let info = BootInfo {
            header: BootInfoHeader {
                magic: BOOT_INFO_MAGIC,
                version_major: BOOT_ABI_VERSION_MAJOR,
                version_minor: BOOT_ABI_VERSION_MINOR,
                total_size: total as u32,
                config_hash: [0; 32],
            },
            acpi: BootAcpiInfo { rsdp_address: 0 },
            memory_map_entry_count: entry_count,
            flags: 0,
            hypervisor_load_address: 0x100000,
            boot_info_bytes: total as u32,
        };
        (info, total)
    }

    #[test]
    fn boot_info_fixed_prefix_size_matches_struct() {
        assert_eq!(BOOT_INFO_FIXED_PREFIX_BYTES, size_of::<BootInfo>());
        assert_eq!(size_of::<BootInfo>(), 80);
        assert_eq!(size_of::<BootMemoryDescriptor>(), 32);
        assert_eq!(align_of::<BootInfo>(), 8);
    }

    #[test]
    fn validate_accepts_zero_entry_blob() {
        let (info, total) = sample_info(0);
        let result = unsafe { validate_boot_info_layout(&info, total) };
        assert!(result.is_ok());
    }

    #[test]
    fn validate_rejects_size_mismatch() {
        let (info, total) = sample_info(2);
        let result = unsafe { validate_boot_info_layout(&info, total - 1) };
        assert!(matches!(
            result,
            Err(BootLayoutError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn memory_map_slice_empty() {
        let (info, _total) = sample_info(0);
        let slice = unsafe { memory_map_slice(&info) };
        assert!(slice.is_empty());
    }
}
