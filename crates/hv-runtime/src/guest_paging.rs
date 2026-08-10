//! Guest long-mode identity page tables installed before VMLAUNCH.

/// Guest PML4 GPA used as CR3.
pub const GUEST_CR3_GPA: u64 = 0xA000;
/// Guest GDT GPA (null / CS64 / DS / TSS).
pub const GUEST_GDT_GPA: u64 = 0x7000;

const PTE_PRESENT: u64 = 1 << 0;
const PTE_WRITE: u64 = 1 << 1;
const PTE_PS: u64 = 1 << 7;

const PDPT_GPA: u64 = 0xB000;
/// PD page base for PDPT index `i` at `0xC000 + i * 0x1000` (i = 0..4).
const PD_BASE_GPA: u64 = 0xC000;

/// Minimum guest RAM needed for the identity-map tables and GDT.
pub const MIN_GUEST_RAM_FOR_PAGING: usize = 0x10_000;

/// Installs a minimal long-mode identity map covering:
/// - the first 2 MiB (code, stack, boot info, page tables, GDT)
/// - each requested high GPA as a 2 MiB leaf (IPC / MMIO)
///
/// # Errors
///
/// Returns `false` when `guest_mem` is too small.
#[must_use]
pub fn install_identity_map(guest_mem: &mut [u8], high_gpas: &[u64]) -> bool {
    if guest_mem.len() < MIN_GUEST_RAM_FOR_PAGING {
        return false;
    }

    // Clear only paging/GDT pages — do not wipe ELF or boot-info payloads.
    clear_page(guest_mem, GUEST_GDT_GPA as usize);
    clear_page(guest_mem, GUEST_CR3_GPA as usize);
    clear_page(guest_mem, PDPT_GPA as usize);
    for idx in 0..4 {
        clear_page(guest_mem, pd_gpa(idx) as usize);
    }

    write_u64(guest_mem, GUEST_CR3_GPA as usize, PDPT_GPA | PTE_PRESENT | PTE_WRITE);
    ensure_pd(guest_mem, 0);
    write_u64(guest_mem, pd_gpa(0) as usize, PTE_PRESENT | PTE_WRITE | PTE_PS);

    for &gpa in high_gpas {
        map_high_2mib(guest_mem, gpa);
    }

    // GDT: idx0 null, idx1 CS64 (0x08), idx2 DS (0x10), idx3 TSS (0x18)
    write_u64(guest_mem, GUEST_GDT_GPA as usize + 0x08, 0x00AF_9B00_0000_FFFF);
    write_u64(guest_mem, GUEST_GDT_GPA as usize + 0x10, 0x00CF_9300_0000_FFFF);
    write_u64(guest_mem, GUEST_GDT_GPA as usize + 0x18, 0x0000_8900_0000_0067);
    write_u64(guest_mem, GUEST_GDT_GPA as usize + 0x20, 0);
    true
}

fn pd_gpa(pdpt_idx: usize) -> u64 {
    PD_BASE_GPA + (pdpt_idx as u64) * 0x1000
}

fn ensure_pd(guest_mem: &mut [u8], pdpt_idx: usize) {
    let pdpt_off = PDPT_GPA as usize + pdpt_idx * 8;
    let existing = read_u64(guest_mem, pdpt_off);
    if existing & PTE_PRESENT == 0 {
        write_u64(guest_mem, pdpt_off, pd_gpa(pdpt_idx) | PTE_PRESENT | PTE_WRITE);
    }
}

fn map_high_2mib(guest_mem: &mut [u8], gpa: u64) {
    let pdpt_idx = ((gpa >> 30) & 0x1FF) as usize;
    if pdpt_idx > 3 {
        return;
    }
    let pd_idx = ((gpa >> 21) & 0x1FF) as usize;
    ensure_pd(guest_mem, pdpt_idx);
    let pd_off = pd_gpa(pdpt_idx) as usize + pd_idx * 8;
    write_u64(
        guest_mem,
        pd_off,
        (gpa & !0x1F_FFFF) | PTE_PRESENT | PTE_WRITE | PTE_PS,
    );
}

fn clear_page(mem: &mut [u8], offset: usize) {
    mem[offset..offset + 0x1000].fill(0);
}

fn write_u64(mem: &mut [u8], offset: usize, value: u64) {
    mem[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u64(mem: &[u8], offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&mem[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn identity_map_covers_low_ram_and_high_gpas() {
        let mut mem = [0u8; MIN_GUEST_RAM_FOR_PAGING];
        assert!(install_identity_map(&mut mem, &[0x4000_0000, 0x8000_0000, 0xFEB0_0000]));
        let pml4e = read_u64(&mem, GUEST_CR3_GPA as usize);
        assert_ne!(pml4e & PTE_PRESENT, 0);
        let low = read_u64(&mem, pd_gpa(0) as usize);
        assert_ne!(low & PTE_PS, 0);
        let ipc_pdpt = read_u64(&mem, PDPT_GPA as usize + 8);
        assert_ne!(ipc_pdpt & PTE_PRESENT, 0);
        let mid_pdpt = read_u64(&mem, PDPT_GPA as usize + 16);
        assert_ne!(mid_pdpt & PTE_PRESENT, 0);
    }
}
