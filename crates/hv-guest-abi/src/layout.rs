//! Guest boot info blob layout helpers.

#![allow(unsafe_code)] // Guest handoff requires raw pointer layout over the boot blob.

use core::mem::{align_of, size_of};

use crate::{GuestBootInfo, GuestIpcRegion, GuestMemoryRegion, GuestMmioRegion};

/// Fixed prefix size in bytes (the [`GuestBootInfo`] struct).
pub const GUEST_BOOT_INFO_FIXED_PREFIX_BYTES: usize = size_of::<GuestBootInfo>();

/// Errors validating a guest boot info blob layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuestLayoutError {
    /// Pointer is null.
    NullPointer,
    /// Total byte count is smaller than the fixed prefix.
    BufferTooSmall,
    /// `total_size` does not match the provided total size.
    SizeMismatch {
        /// Declared size in the header.
        declared: u32,
        /// Caller-provided total size.
        provided: usize,
    },
    /// Header `total_size` does not match the prefix field.
    HeaderTotalSizeMismatch {
        /// Header field.
        header_total: u32,
        /// Prefix field.
        prefix_total: u32,
    },
    /// A trailing table extends past the buffer.
    TableOutOfBounds {
        /// Table name for diagnostics.
        table: &'static str,
        /// Required bytes for the full blob.
        required: usize,
        /// Provided buffer size.
        provided: usize,
    },
    /// Magic or major version is incompatible.
    IncompatibleHeader,
    /// A trailing table would be misaligned.
    MisalignedTable {
        /// Table name for diagnostics.
        table: &'static str,
    },
    /// Integer overflow computing layout sizes.
    Overflow,
}

impl GuestLayoutError {
    /// Returns a static label for diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NullPointer => "null guest boot info pointer",
            Self::BufferTooSmall => "guest boot info buffer too small",
            Self::SizeMismatch { .. } => "guest total_size mismatch",
            Self::HeaderTotalSizeMismatch { .. } => "guest header total_size mismatch",
            Self::TableOutOfBounds { .. } => "guest boot info table out of bounds",
            Self::IncompatibleHeader => "incompatible guest boot info header",
            Self::MisalignedTable { .. } => "misaligned guest boot info table",
            Self::Overflow => "guest boot info layout overflow",
        }
    }
}

/// Returns the memory region table immediately following the fixed prefix.
///
/// # Safety
///
/// `info` must point to a valid guest boot info blob whose trailing tables are
/// initialized for `info.memory_region_count` entries.
pub unsafe fn memory_regions_slice(info: &GuestBootInfo) -> &[GuestMemoryRegion] {
    let ptr = (info as *const GuestBootInfo).cast::<u8>().add(GUEST_BOOT_INFO_FIXED_PREFIX_BYTES)
        as *const GuestMemoryRegion;
    core::slice::from_raw_parts(ptr, info.memory_region_count as usize)
}

/// Returns the IPC region table following the memory region table.
///
/// # Safety
///
/// `info` must point to a valid guest boot info blob with initialized tables.
pub unsafe fn ipc_regions_slice(info: &GuestBootInfo) -> &[GuestIpcRegion] {
    let offset = GUEST_BOOT_INFO_FIXED_PREFIX_BYTES
        + info.memory_region_count as usize * size_of::<GuestMemoryRegion>();
    let ptr = (info as *const GuestBootInfo).cast::<u8>().add(offset) as *const GuestIpcRegion;
    core::slice::from_raw_parts(ptr, info.ipc_region_count as usize)
}

/// Returns the MMIO region table following the IPC region table.
///
/// # Safety
///
/// `info` must point to a valid guest boot info blob with initialized tables.
pub unsafe fn mmio_regions_slice(info: &GuestBootInfo) -> &[GuestMmioRegion] {
    let offset = GUEST_BOOT_INFO_FIXED_PREFIX_BYTES
        + info.memory_region_count as usize * size_of::<GuestMemoryRegion>()
        + info.ipc_region_count as usize * size_of::<GuestIpcRegion>();
    let ptr = (info as *const GuestBootInfo).cast::<u8>().add(offset) as *const GuestMmioRegion;
    core::slice::from_raw_parts(ptr, info.mmio_region_count as usize)
}

/// Validates guest boot info layout against the caller-provided total byte count.
///
/// # Safety
///
/// `info` must refer to at least `total_bytes` readable bytes.
pub unsafe fn validate_guest_boot_info_layout(
    info: *const GuestBootInfo,
    total_bytes: usize,
) -> Result<(), GuestLayoutError> {
    if info.is_null() {
        return Err(GuestLayoutError::NullPointer);
    }
    if total_bytes < GUEST_BOOT_INFO_FIXED_PREFIX_BYTES {
        return Err(GuestLayoutError::BufferTooSmall);
    }

    let info = unsafe { &*info };
    if !info.header.is_compatible() {
        return Err(GuestLayoutError::IncompatibleHeader);
    }

    if info.header.total_size as usize != total_bytes {
        return Err(GuestLayoutError::SizeMismatch {
            declared: info.header.total_size,
            provided: total_bytes,
        });
    }

    let memory_bytes = table_bytes(
        info.memory_region_count,
        size_of::<GuestMemoryRegion>(),
        "memory",
        total_bytes,
    )?;
    let ipc_bytes =
        table_bytes(info.ipc_region_count, size_of::<GuestIpcRegion>(), "ipc", total_bytes)?;
    let mmio_bytes =
        table_bytes(info.mmio_region_count, size_of::<GuestMmioRegion>(), "mmio", total_bytes)?;

    let required = GUEST_BOOT_INFO_FIXED_PREFIX_BYTES
        .checked_add(memory_bytes)
        .and_then(|value| value.checked_add(ipc_bytes))
        .and_then(|value| value.checked_add(mmio_bytes))
        .ok_or(GuestLayoutError::Overflow)?;

    if required != total_bytes {
        return Err(GuestLayoutError::TableOutOfBounds {
            table: "guest boot info",
            required,
            provided: total_bytes,
        });
    }

    let memory_offset = GUEST_BOOT_INFO_FIXED_PREFIX_BYTES;
    if memory_offset % align_of::<GuestMemoryRegion>() != 0 {
        return Err(GuestLayoutError::MisalignedTable { table: "memory" });
    }
    let ipc_offset = memory_offset + memory_bytes;
    if ipc_offset % align_of::<GuestIpcRegion>() != 0 {
        return Err(GuestLayoutError::MisalignedTable { table: "ipc" });
    }
    let mmio_offset = ipc_offset + ipc_bytes;
    if mmio_offset % align_of::<GuestMmioRegion>() != 0 {
        return Err(GuestLayoutError::MisalignedTable { table: "mmio" });
    }

    Ok(())
}

fn table_bytes(
    count: u32,
    entry_size: usize,
    table: &'static str,
    total_bytes: usize,
) -> Result<usize, GuestLayoutError> {
    let bytes = (count as usize).checked_mul(entry_size).ok_or(GuestLayoutError::Overflow)?;
    let prefix_end =
        GUEST_BOOT_INFO_FIXED_PREFIX_BYTES.checked_add(bytes).ok_or(GuestLayoutError::Overflow)?;
    if prefix_end > total_bytes {
        return Err(GuestLayoutError::TableOutOfBounds {
            table,
            required: prefix_end,
            provided: total_bytes,
        });
    }
    Ok(bytes)
}

/// Computes total guest boot info bytes for the given table counts.
///
/// # Errors
///
/// Returns [`GuestLayoutError::Overflow`] when sizes overflow.
pub fn compute_guest_boot_info_bytes(
    memory_region_count: u32,
    ipc_region_count: u32,
    mmio_region_count: u32,
) -> Result<usize, GuestLayoutError> {
    let memory = table_size(memory_region_count, size_of::<GuestMemoryRegion>())?;
    let ipc = table_size(ipc_region_count, size_of::<GuestIpcRegion>())?;
    let mmio = table_size(mmio_region_count, size_of::<GuestMmioRegion>())?;
    GUEST_BOOT_INFO_FIXED_PREFIX_BYTES
        .checked_add(memory)
        .and_then(|value| value.checked_add(ipc))
        .and_then(|value| value.checked_add(mmio))
        .ok_or(GuestLayoutError::Overflow)
}

fn table_size(count: u32, entry_size: usize) -> Result<usize, GuestLayoutError> {
    (count as usize).checked_mul(entry_size).ok_or(GuestLayoutError::Overflow)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::GUEST_ABI_VERSION_MINOR;
    use crate::{GuestBootInfoHeader, GUEST_ABI_VERSION_MAJOR, GUEST_BOOT_INFO_MAGIC};

    fn sample_prefix(memory_count: u32, ipc_count: u32, mmio_count: u32) -> (GuestBootInfo, usize) {
        let total =
            compute_guest_boot_info_bytes(memory_count, ipc_count, mmio_count).expect("bytes");
        let info = GuestBootInfo {
            header: GuestBootInfoHeader {
                magic: GUEST_BOOT_INFO_MAGIC,
                version_major: GUEST_ABI_VERSION_MAJOR,
                version_minor: GUEST_ABI_VERSION_MINOR,
                total_size: total as u32,
                vm_id: 1,
                vcpu_id: 0,
            },
            memory_region_count: memory_count,
            ipc_region_count: ipc_count,
            mmio_region_count: mmio_count,
            reserved: 0,
        };
        (info, total)
    }

    #[test]
    fn guest_boot_info_fixed_prefix_size_matches_struct() {
        assert_eq!(GUEST_BOOT_INFO_FIXED_PREFIX_BYTES, size_of::<GuestBootInfo>());
        assert_eq!(size_of::<GuestBootInfo>(), 40);
        assert_eq!(align_of::<GuestBootInfo>(), 8);
    }

    #[test]
    fn validate_accepts_zero_tables() {
        let (info, total) = sample_prefix(0, 0, 0);
        let result = unsafe { validate_guest_boot_info_layout(&info, total) };
        assert!(result.is_ok());
    }

    #[test]
    fn validate_rejects_size_mismatch() {
        let (info, total) = sample_prefix(1, 1, 0);
        let result = unsafe { validate_guest_boot_info_layout(&info, total - 1) };
        assert!(matches!(result, Err(GuestLayoutError::SizeMismatch { .. })));
    }
}
