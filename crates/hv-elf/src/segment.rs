//! Program header and loadable segment types.

/// Program header type for a loadable segment (`PT_LOAD`).
pub const PT_LOAD: u32 = 1;

/// Segment is readable (`PF_R`).
pub const PF_R: u32 = 4;
/// Segment is writable (`PF_W`).
pub const PF_W: u32 = 2;
/// Segment is executable (`PF_X`).
pub const PF_X: u32 = 1;

/// A validated ELF64 program header describing one loadable segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elf64Segment {
    /// Program header type (`p_type`).
    pub typ: u32,
    /// Segment flags (`p_flags`).
    pub flags: u32,
    /// File offset of the segment (`p_offset`).
    pub file_offset: u64,
    /// Virtual address of the segment (`p_vaddr`).
    pub virt_addr: u64,
    /// Physical address of the segment (`p_paddr`).
    pub phys_addr: u64,
    /// Size of the segment in the file (`p_filesz`).
    pub file_size: u64,
    /// Size of the segment in memory (`p_memsz`).
    pub mem_size: u64,
    /// Segment alignment (`p_align`).
    pub align: u64,
}

impl Elf64Segment {
    /// Returns true when this is a loadable segment.
    #[must_use]
    pub const fn is_load(&self) -> bool {
        self.typ == PT_LOAD
    }

    /// Returns true when the segment is mapped read-only.
    #[must_use]
    pub const fn is_readable(&self) -> bool {
        self.flags & PF_R != 0
    }

    /// Returns true when the segment is mapped writable.
    #[must_use]
    pub const fn is_writable(&self) -> bool {
        self.flags & PF_W != 0
    }

    /// Returns true when the segment is mapped executable.
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }
}
