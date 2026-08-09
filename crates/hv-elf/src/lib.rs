//! Minimal ELF64 parser for Hypster guest images.
//!
//! This crate validates ELF64 little-endian x86-64 executables sufficiently
//! for hypervisor loading: magic, architecture, program headers, segment file
//! bounds, and virtual-address overlap checks are performed with overflow-safe
//! arithmetic throughout.
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | Header and segment bounds | UNIT |
//! | Loadable segment overlap | UNIT |
//! | Overflow-safe arithmetic | UNIT + PROPERTY |

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(test, allow(clippy::expect_used, clippy::slow_vector_initialization))]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod header;
pub mod segment;

pub use error::ElfError;
pub use header::{
    EI_CLASS, EI_DATA, ELF64_EHDR_SIZE, ELF64_PHDR_SIZE, ELFCLASS64, ELFDATA2LSB, ELF_MAGIC,
    EM_X86_64,
};
pub use segment::{Elf64Segment, PF_R, PF_W, PF_X, PT_LOAD};

use alloc::vec::Vec;

use hv_types::arithmetic::{
    checked_add_u64, checked_add_usize, checked_mul_usize, ranges_overlap_u64, ArithmeticError,
};

use crate::header::{
    E_ENTRY_OFFSET, E_MACHINE_OFFSET, E_PHENTSIZE_OFFSET, E_PHNUM_OFFSET, E_PHOFF_OFFSET,
};

/// A validated ELF64 x86-64 image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Elf64<'a> {
    data: &'a [u8],
    entry_point: u64,
    segments: Vec<Elf64Segment>,
}

impl<'a> Elf64<'a> {
    /// Returns the raw image bytes.
    #[must_use]
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Returns the program entry point (`e_entry`).
    #[must_use]
    pub fn entry_point(&self) -> u64 {
        self.entry_point
    }

    /// Returns all parsed program headers.
    #[must_use]
    pub fn segments(&self) -> &[Elf64Segment] {
        &self.segments
    }

    /// Returns only loadable (`PT_LOAD`) segments.
    pub fn load_segments(&self) -> impl Iterator<Item = &Elf64Segment> {
        self.segments.iter().filter(|segment| segment.is_load())
    }
}

/// Parses and validates an ELF64 little-endian x86-64 image.
///
/// # Errors
///
/// Returns [`ElfError`] when the image is malformed, unsupported, or fails
/// segment validation.
pub fn parse_elf64(data: &[u8]) -> Result<Elf64<'_>, ElfError> {
    validate_ident(data)?;
    let entry_point = read_u64_le(data, E_ENTRY_OFFSET)?;
    let phoff = read_u64_le(data, E_PHOFF_OFFSET)?;
    let phentsize = read_u16_le(data, E_PHENTSIZE_OFFSET)?;
    let phnum = read_u16_le(data, E_PHNUM_OFFSET)?;

    if phentsize != ELF64_PHDR_SIZE as u16 {
        return Err(ElfError::InvalidProgramHeaderEntrySize { found: phentsize });
    }

    if phnum == 0 {
        return Err(ElfError::ProgramHeaderTableOutOfBounds);
    }

    let phnum_usize = usize::from(phnum);
    let table_bytes = checked_mul_usize(phnum_usize, ELF64_PHDR_SIZE).map_err(map_arithmetic)?;
    let phoff_usize = program_header_offset(phoff, data.len())?;
    let table_end = checked_add_usize(phoff_usize, table_bytes).map_err(map_arithmetic)?;

    if table_end > data.len() {
        return Err(ElfError::ProgramHeaderTableOutOfBounds);
    }

    let mut segments = Vec::with_capacity(phnum_usize);
    for index in 0..phnum_usize {
        let header_offset =
            checked_add_usize(phoff_usize, index * ELF64_PHDR_SIZE).map_err(map_arithmetic)?;
        let header_end =
            checked_add_usize(header_offset, ELF64_PHDR_SIZE).map_err(map_arithmetic)?;
        if header_end > data.len() {
            return Err(ElfError::ProgramHeaderOutOfBounds { index });
        }

        let segment = parse_program_header(data, header_offset)?;
        validate_segment_file_bounds(data, index, &segment)?;
        segments.push(segment);
    }

    validate_load_segment_overlaps(&segments)?;

    Ok(Elf64 { data, entry_point, segments })
}

fn validate_ident(data: &[u8]) -> Result<(), ElfError> {
    if data.len() < ELF64_EHDR_SIZE {
        return Err(ElfError::BufferTooShort { needed: ELF64_EHDR_SIZE, available: data.len() });
    }

    if data[0..4] != ELF_MAGIC {
        return Err(ElfError::InvalidMagic);
    }

    let class = data[EI_CLASS];
    if class != ELFCLASS64 {
        return Err(ElfError::UnsupportedClass { found: class });
    }

    let data_encoding = data[EI_DATA];
    if data_encoding != ELFDATA2LSB {
        return Err(ElfError::UnsupportedEndianness { found: data_encoding });
    }

    let machine = read_u16_le(data, E_MACHINE_OFFSET)?;
    if machine != EM_X86_64 {
        return Err(ElfError::UnsupportedMachine { found: machine });
    }

    Ok(())
}

fn parse_program_header(data: &[u8], offset: usize) -> Result<Elf64Segment, ElfError> {
    Ok(Elf64Segment {
        typ: read_u32_le(data, offset)?,
        flags: read_u32_le(data, offset + 4)?,
        file_offset: read_u64_le(data, offset + 8)?,
        virt_addr: read_u64_le(data, offset + 16)?,
        phys_addr: read_u64_le(data, offset + 24)?,
        file_size: read_u64_le(data, offset + 32)?,
        mem_size: read_u64_le(data, offset + 40)?,
        align: read_u64_le(data, offset + 48)?,
    })
}

fn validate_segment_file_bounds(
    data: &[u8],
    index: usize,
    segment: &Elf64Segment,
) -> Result<(), ElfError> {
    if segment.file_size == 0 {
        return Ok(());
    }

    let end = match checked_add_u64(segment.file_offset, segment.file_size) {
        Ok(value) => value,
        Err(_) => return Err(ElfError::SegmentFileDataOutOfBounds { index }),
    };
    if end > data.len() as u64 {
        return Err(ElfError::SegmentFileDataOutOfBounds { index });
    }

    Ok(())
}

fn validate_load_segment_overlaps(segments: &[Elf64Segment]) -> Result<(), ElfError> {
    for (first, left) in segments.iter().enumerate() {
        if !left.is_load() || left.mem_size == 0 {
            continue;
        }
        for (second, right) in segments.iter().enumerate().skip(first + 1) {
            if !right.is_load() || right.mem_size == 0 {
                continue;
            }
            if ranges_overlap_u64(left.virt_addr, left.mem_size, right.virt_addr, right.mem_size) {
                return Err(ElfError::SegmentOverlap { first, second });
            }
        }
    }
    Ok(())
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, ElfError> {
    ensure_bytes(data, offset, 2)?;
    let bytes = &data[offset..offset + 2];
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, ElfError> {
    ensure_bytes(data, offset, 4)?;
    let bytes = &data[offset..offset + 4];
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64_le(data: &[u8], offset: usize) -> Result<u64, ElfError> {
    ensure_bytes(data, offset, 8)?;
    let bytes = &data[offset..offset + 8];
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn ensure_bytes(data: &[u8], offset: usize, len: usize) -> Result<(), ElfError> {
    let end = checked_add_usize(offset, len).map_err(map_arithmetic)?;
    if end > data.len() {
        return Err(ElfError::BufferTooShort { needed: end, available: data.len() });
    }
    Ok(())
}

fn program_header_offset(phoff: u64, data_len: usize) -> Result<usize, ElfError> {
    if phoff > data_len as u64 {
        return Err(ElfError::ProgramHeaderTableOutOfBounds);
    }
    usize::try_from(phoff).map_err(|_| ElfError::ProgramHeaderTableOutOfBounds)
}

const fn map_arithmetic(_err: ArithmeticError) -> ElfError {
    ElfError::Overflow
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::E_TYPE_OFFSET;

    fn write_u16_le(buf: &mut [u8], offset: usize, value: u16) {
        buf[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32_le(buf: &mut [u8], offset: usize, value: u32) {
        buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64_le(buf: &mut [u8], offset: usize, value: u64) {
        buf[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_header(buf: &mut [u8], entry: u64, phoff: u64, phnum: u16) {
        buf[0..4].copy_from_slice(&ELF_MAGIC);
        buf[EI_CLASS] = ELFCLASS64;
        buf[EI_DATA] = ELFDATA2LSB;
        write_u16_le(buf, E_TYPE_OFFSET, 2); // ET_EXEC
        write_u16_le(buf, E_MACHINE_OFFSET, EM_X86_64);
        write_u64_le(buf, E_ENTRY_OFFSET, entry);
        write_u64_le(buf, E_PHOFF_OFFSET, phoff);
        write_u16_le(buf, E_PHENTSIZE_OFFSET, ELF64_PHDR_SIZE as u16);
        write_u16_le(buf, E_PHNUM_OFFSET, phnum);
    }

    fn write_load_phdr(
        buf: &mut [u8],
        offset: usize,
        file_offset: u64,
        virt_addr: u64,
        file_size: u64,
        mem_size: u64,
        flags: u32,
    ) {
        write_u32_le(buf, offset, PT_LOAD);
        write_u32_le(buf, offset + 4, flags);
        write_u64_le(buf, offset + 8, file_offset);
        write_u64_le(buf, offset + 16, virt_addr);
        write_u64_le(buf, offset + 24, virt_addr);
        write_u64_le(buf, offset + 32, file_size);
        write_u64_le(buf, offset + 40, mem_size);
        write_u64_le(buf, offset + 48, 0x1000);
    }

    fn minimal_valid_elf() -> alloc::vec::Vec<u8> {
        let phoff = ELF64_EHDR_SIZE as u64;
        let file_offset = phoff + ELF64_PHDR_SIZE as u64;
        let total = (file_offset + 4) as usize;
        let mut image = alloc::vec::Vec::with_capacity(total);
        image.resize(total, 0);
        write_header(&mut image, 0x1000, phoff, 1);
        write_load_phdr(&mut image, ELF64_EHDR_SIZE, file_offset, 0x1000, 4, 4, PF_R | PF_X);
        image[file_offset as usize..total].copy_from_slice(&[0x90, 0x90, 0x90, 0xC3]);
        image
    }

    #[test]
    fn parse_valid_minimal_elf() {
        let image = minimal_valid_elf();
        let elf = parse_elf64(&image).expect("valid ELF should parse");
        assert_eq!(elf.entry_point(), 0x1000);
        assert_eq!(elf.segments().len(), 1);
        let segment = &elf.segments()[0];
        assert!(segment.is_load());
        assert!(segment.is_readable());
        assert!(segment.is_executable());
        assert!(!segment.is_writable());
        assert_eq!(segment.file_size, 4);
        assert_eq!(segment.mem_size, 4);
    }

    #[test]
    fn rejects_truncated_header() {
        let err = parse_elf64(&[0x7f, b'E', b'L']).expect_err("short buffer");
        assert!(matches!(err, ElfError::BufferTooShort { needed: ELF64_EHDR_SIZE, available: 3 }));
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut image = minimal_valid_elf();
        image[0] = b'X';
        assert_eq!(parse_elf64(&image), Err(ElfError::InvalidMagic));
    }

    #[test]
    fn rejects_non_64_bit_class() {
        let mut image = minimal_valid_elf();
        image[EI_CLASS] = 1;
        assert_eq!(parse_elf64(&image), Err(ElfError::UnsupportedClass { found: 1 }));
    }

    #[test]
    fn rejects_big_endian() {
        let mut image = minimal_valid_elf();
        image[EI_DATA] = 2;
        assert_eq!(parse_elf64(&image), Err(ElfError::UnsupportedEndianness { found: 2 }));
    }

    #[test]
    fn rejects_non_x86_64_machine() {
        let mut image = minimal_valid_elf();
        write_u16_le(&mut image, E_MACHINE_OFFSET, 0xB7); // EM_AARCH64
        assert_eq!(parse_elf64(&image), Err(ElfError::UnsupportedMachine { found: 0xB7 }));
    }

    #[test]
    fn rejects_zero_program_headers() {
        let mut image = minimal_valid_elf();
        write_u16_le(&mut image, E_PHNUM_OFFSET, 0);
        assert_eq!(parse_elf64(&image), Err(ElfError::ProgramHeaderTableOutOfBounds));
    }

    #[test]
    fn rejects_program_header_table_out_of_bounds() {
        let mut image = minimal_valid_elf();
        write_u64_le(&mut image, E_PHOFF_OFFSET, 0x1000);
        assert_eq!(parse_elf64(&image), Err(ElfError::ProgramHeaderTableOutOfBounds));
    }

    #[test]
    fn rejects_invalid_program_header_entry_size() {
        let mut image = minimal_valid_elf();
        write_u16_le(&mut image, E_PHENTSIZE_OFFSET, 32);
        assert_eq!(parse_elf64(&image), Err(ElfError::InvalidProgramHeaderEntrySize { found: 32 }));
    }

    #[test]
    fn rejects_segment_file_data_out_of_bounds() {
        let mut image = minimal_valid_elf();
        write_u64_le(&mut image, ELF64_EHDR_SIZE + 32, 0x1000);
        assert_eq!(parse_elf64(&image), Err(ElfError::SegmentFileDataOutOfBounds { index: 0 }));
    }

    #[test]
    fn rejects_overlapping_load_segments() {
        let phoff = ELF64_EHDR_SIZE as u64;
        let file_offset = phoff + (2 * ELF64_PHDR_SIZE) as u64;
        let total = (file_offset + 8) as usize;
        let mut image = alloc::vec::Vec::with_capacity(total);
        image.resize(total, 0);
        write_header(&mut image, 0x2000, phoff, 2);
        write_load_phdr(&mut image, ELF64_EHDR_SIZE, file_offset, 0x1000, 4, 0x1000, PF_R | PF_X);
        write_load_phdr(
            &mut image,
            ELF64_EHDR_SIZE + ELF64_PHDR_SIZE,
            file_offset + 4,
            0x1800,
            4,
            0x1000,
            PF_R | PF_W,
        );
        image[file_offset as usize..total].copy_from_slice(&[0; 8]);

        assert_eq!(parse_elf64(&image), Err(ElfError::SegmentOverlap { first: 0, second: 1 }));
    }

    #[test]
    fn allows_adjacent_non_overlapping_load_segments() {
        let phoff = ELF64_EHDR_SIZE as u64;
        let file_offset = phoff + (2 * ELF64_PHDR_SIZE) as u64;
        let total = (file_offset + 8) as usize;
        let mut image = alloc::vec::Vec::with_capacity(total);
        image.resize(total, 0);
        write_header(&mut image, 0x1000, phoff, 2);
        write_load_phdr(&mut image, ELF64_EHDR_SIZE, file_offset, 0x1000, 4, 0x1000, PF_R | PF_X);
        write_load_phdr(
            &mut image,
            ELF64_EHDR_SIZE + ELF64_PHDR_SIZE,
            file_offset + 4,
            0x2000,
            4,
            0x1000,
            PF_R | PF_W,
        );
        image[file_offset as usize..total].copy_from_slice(&[0; 8]);

        let elf = parse_elf64(&image).expect("adjacent segments should parse");
        assert_eq!(elf.load_segments().count(), 2);
    }

    #[test]
    fn rejects_segment_file_range_overflow() {
        let mut image = minimal_valid_elf();
        write_u64_le(&mut image, ELF64_EHDR_SIZE + 8, u64::MAX);
        write_u64_le(&mut image, ELF64_EHDR_SIZE + 32, 1);
        assert_eq!(parse_elf64(&image), Err(ElfError::SegmentFileDataOutOfBounds { index: 0 }));
    }
}
