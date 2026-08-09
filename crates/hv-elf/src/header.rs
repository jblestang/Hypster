//! ELF identification and executable header constants.

/// ELF magic bytes `\x7fELF`.
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// Index of the ELF class byte in `e_ident`.
pub const EI_CLASS: usize = 4;
/// Index of the ELF data-encoding byte in `e_ident`.
pub const EI_DATA: usize = 5;

/// 64-bit ELF objects.
pub const ELFCLASS64: u8 = 2;
/// Little-endian data encoding.
pub const ELFDATA2LSB: u8 = 1;

/// x86-64 machine type (`EM_X86_64`).
pub const EM_X86_64: u16 = 62;

/// Size of the ELF64 executable header in bytes.
pub const ELF64_EHDR_SIZE: usize = 64;
/// Size of one ELF64 program header in bytes.
pub const ELF64_PHDR_SIZE: usize = 56;

/// Byte offset of `e_type` in an ELF64 header.
pub const E_TYPE_OFFSET: usize = 16;
/// Byte offset of `e_machine` in an ELF64 header.
pub const E_MACHINE_OFFSET: usize = 18;
/// Byte offset of `e_entry` in an ELF64 header.
pub const E_ENTRY_OFFSET: usize = 24;
/// Byte offset of `e_phoff` in an ELF64 header.
pub const E_PHOFF_OFFSET: usize = 32;
/// Byte offset of `e_phentsize` in an ELF64 header.
pub const E_PHENTSIZE_OFFSET: usize = 54;
/// Byte offset of `e_phnum` in an ELF64 header.
pub const E_PHNUM_OFFSET: usize = 56;
