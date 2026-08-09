//! EPT paging entry format constants.

/// EPT page table entry size in bytes.
pub const EPT_ENTRY_SIZE: usize = 8;
/// EPT page table entries per table.
pub const EPT_ENTRIES_PER_TABLE: usize = 512;
/// EPT page table size in bytes.
pub const EPT_TABLE_SIZE: usize = EPT_ENTRY_SIZE * EPT_ENTRIES_PER_TABLE;

/// EPT permission: read.
pub const EPT_READ: u64 = 1 << 0;
/// EPT permission: write.
pub const EPT_WRITE: u64 = 1 << 1;
/// EPT permission: execute.
pub const EPT_EXECUTE: u64 = 1 << 2;
/// EPT memory type field shift.
pub const EPT_MEMORY_TYPE_SHIFT: u32 = 3;
/// EPT ignore PAT bit.
pub const EPT_IGNORE_PAT: u64 = 1 << 6;
/// EPT large page bit (2 MiB or 1 GiB).
pub const EPT_LARGE_PAGE: u64 = 1 << 7;

/// EPT memory type: uncacheable.
pub const EPT_MEMTYPE_UC: u64 = 0;
/// EPT memory type: write-back.
pub const EPT_MEMTYPE_WB: u64 = 6;

/// EPT page shift for 4 KiB pages.
pub const EPT_PAGE_SHIFT: u32 = 12;
/// EPT page size for 4 KiB pages.
pub const EPT_PAGE_SIZE: u64 = 1 << EPT_PAGE_SHIFT;
/// EPT huge page shift for 2 MiB pages.
pub const EPT_HUGE_2M_SHIFT: u32 = 21;
/// EPT huge page size for 2 MiB pages.
pub const EPT_HUGE_2M_SIZE: u64 = 1 << EPT_HUGE_2M_SHIFT;

/// Builds permission bits from read/write/execute flags.
#[must_use]
pub const fn ept_permissions(read: bool, write: bool, execute: bool) -> u64 {
    (read as u64 * EPT_READ) | (write as u64 * EPT_WRITE) | (execute as u64 * EPT_EXECUTE)
}

/// Builds a non-leaf EPT entry pointing at a child table.
#[must_use]
pub fn ept_pointer_entry(child_hpa: u64) -> u64 {
    (child_hpa & 0x000F_FFFF_FFFF_F000) | EPT_READ | EPT_WRITE | EPT_EXECUTE
}

/// Builds a 4 KiB leaf EPT entry.
#[must_use]
pub fn ept_4k_entry(host_hpa: u64, perms: u64, memtype: u64) -> u64 {
    (host_hpa & 0x000F_FFFF_FFFF_F000) | perms | (memtype << EPT_MEMORY_TYPE_SHIFT)
}

/// Builds a 2 MiB leaf EPT entry.
#[must_use]
pub fn ept_2m_entry(host_hpa: u64, perms: u64, memtype: u64) -> u64 {
    (host_hpa & 0x000F_FFFF_FFE0_0000) | perms | (memtype << EPT_MEMORY_TYPE_SHIFT) | EPT_LARGE_PAGE
}
