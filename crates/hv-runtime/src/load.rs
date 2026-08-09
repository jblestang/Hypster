//! Guest ELF image loading into partition RAM backing.

use hv_elf::{parse_elf64, ElfError};
use hv_types::arithmetic::checked_add_u64;

/// Result of loading a guest ELF image into guest RAM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadedGuestImage {
    /// Program entry point from the ELF header.
    pub entry_point: u64,
    /// Highest guest physical byte used by loadable segments.
    pub loaded_bytes: u64,
}

/// Loads PT_LOAD segments from `image` into guest RAM starting at GPA 0.
///
/// # Errors
///
/// Returns [`ElfError`] when parsing fails or the image does not fit in `guest_ram`.
pub fn load_elf_into_guest_ram(
    image: &[u8],
    guest_ram: &mut [u8],
) -> Result<LoadedGuestImage, ElfError> {
    let elf = parse_elf64(image)?;
    let mut high_water = 0u64;
    for segment in elf.load_segments() {
        let vaddr = segment.virt_addr;
        let mem_size = segment.mem_size;
        let end = checked_add_u64(vaddr, mem_size).map_err(|_| ElfError::Overflow)?;
        if end as usize > guest_ram.len() {
            return Err(ElfError::BufferTooShort {
                needed: end as usize,
                available: guest_ram.len(),
            });
        }
        let start = vaddr as usize;
        let file_size = segment.file_size as usize;
        let file_offset = segment.file_offset as usize;
        if file_offset.checked_add(file_size).is_none() || file_offset + file_size > image.len() {
            return Err(ElfError::SegmentFileDataOutOfBounds { index: 0 });
        }
        let seg_end = start.checked_add(mem_size as usize).ok_or(ElfError::Overflow)?;
        guest_ram[start..seg_end].fill(0);
        guest_ram[start..start + file_size].copy_from_slice(&image[file_offset..file_offset + file_size]);
        if end > high_water {
            high_water = end;
        }
    }
    Ok(LoadedGuestImage { entry_point: elf.entry_point(), loaded_bytes: high_water })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    fn minimal_elf(entry: u64, load_vaddr: u64, payload: &[u8]) -> Vec<u8> {
        use hv_elf::{ELF64_EHDR_SIZE, ELF64_PHDR_SIZE, EM_X86_64, ELFCLASS64, ELFDATA2LSB, ELF_MAGIC};
        use hv_elf::PT_LOAD;

        let phoff = ELF64_EHDR_SIZE as u64;
        let file_offset = phoff + ELF64_PHDR_SIZE as u64;
        let total = (file_offset as usize) + payload.len();
        let mut image = vec![0u8; total];
        image[0..4].copy_from_slice(&ELF_MAGIC);
        image[4] = ELFCLASS64;
        image[5] = ELFDATA2LSB;
        image[0x10..0x12].copy_from_slice(&2u16.to_le_bytes());
        image[0x12..0x14].copy_from_slice(&EM_X86_64.to_le_bytes());
        image[0x18..0x20].copy_from_slice(&entry.to_le_bytes());
        image[0x20..0x28].copy_from_slice(&phoff.to_le_bytes());
        image[0x36..0x38].copy_from_slice(&(ELF64_PHDR_SIZE as u16).to_le_bytes());
        image[0x38..0x3A].copy_from_slice(&1u16.to_le_bytes());

        let phdr = ELF64_EHDR_SIZE;
        image[phdr..phdr + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
        image[phdr + 0x08..phdr + 0x10].copy_from_slice(&file_offset.to_le_bytes());
        image[phdr + 0x10..phdr + 0x18].copy_from_slice(&load_vaddr.to_le_bytes());
        image[phdr + 0x18..phdr + 0x20].copy_from_slice(&load_vaddr.to_le_bytes());
        image[phdr + 0x20..phdr + 0x28].copy_from_slice(&(payload.len() as u64).to_le_bytes());
        image[phdr + 0x28..phdr + 0x30].copy_from_slice(&(payload.len() as u64).to_le_bytes());
        image[phdr + 0x30..phdr + 0x38].copy_from_slice(&0x1000u64.to_le_bytes());
        image[file_offset as usize..total].copy_from_slice(payload);
        image
    }

    #[test]
    fn load_elf_places_segments_at_guest_phys_zero() {
        let image = minimal_elf(0x1000, 0, b"hello-guest");
        let mut ram = vec![0u8; 4096];
        let loaded = load_elf_into_guest_ram(&image, &mut ram).expect("load");
        assert_eq!(loaded.entry_point, 0x1000);
        assert_eq!(&ram[..11], b"hello-guest");
    }
}
