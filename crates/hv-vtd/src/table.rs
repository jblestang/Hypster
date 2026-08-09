//! Intel VT-d root and context table entry format constants.

/// Root-table entry size in bytes (128-bit).
pub const ROOT_ENTRY_SIZE: usize = 16;
/// Root-table entries (one per PCI bus).
pub const ROOT_ENTRY_COUNT: usize = 256;
/// Root-table size in bytes.
pub const ROOT_TABLE_SIZE: usize = ROOT_ENTRY_SIZE * ROOT_ENTRY_COUNT;

/// Context-table entry size in bytes (256-bit).
pub const CONTEXT_ENTRY_SIZE: usize = 32;
/// Context-table entries (one per device/function on a bus).
pub const CONTEXT_ENTRY_COUNT: usize = 256;
/// Context-table size in bytes.
pub const CONTEXT_TABLE_SIZE: usize = CONTEXT_ENTRY_SIZE * CONTEXT_ENTRY_COUNT;

/// Root-table present bit.
pub const ROOT_PRESENT: u64 = 1;
/// Context-entry present bit.
pub const CONTEXT_PRESENT: u64 = 1;
/// Context-entry translation type: translate (second-level).
pub const CONTEXT_TT_TRANSLATE: u64 = 1 << 2;

/// Builds a 128-bit root-table entry pointing at `context_table_hpa`.
#[must_use]
pub fn root_entry(context_table_hpa: u64) -> [u8; ROOT_ENTRY_SIZE] {
    let lo = (context_table_hpa & 0x000F_FFFF_FFFF_F000) | ROOT_PRESENT;
    let mut bytes = [0u8; ROOT_ENTRY_SIZE];
    bytes[..8].copy_from_slice(&lo.to_le_bytes());
    bytes
}

/// Builds a 256-bit context entry for the given domain identifier.
#[must_use]
pub fn context_entry(domain_id: u16) -> [u8; CONTEXT_ENTRY_SIZE] {
    let lo = CONTEXT_PRESENT | CONTEXT_TT_TRANSLATE | ((domain_id as u64) << 8);
    let mut bytes = [0u8; CONTEXT_ENTRY_SIZE];
    bytes[..8].copy_from_slice(&lo.to_le_bytes());
    bytes
}

/// Context-table index for a PCI device/function on a bus.
#[must_use]
pub const fn context_index(device: u8, function: u8) -> usize {
    ((device as usize) << 3) | function as usize
}
