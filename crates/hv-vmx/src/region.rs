//! VMXON region layout.

/// Size of a VMXON region in bytes.
pub const VMXON_REGION_SIZE: usize = 4096;

/// 4 KiB-aligned VMXON region.
#[repr(C, align(4096))]
#[derive(Clone, Copy)]
pub struct VmxonRegion {
    data: [u8; VMXON_REGION_SIZE],
}

impl VmxonRegion {
    /// Creates a zeroed VMXON region with the revision identifier written at offset 0.
    #[must_use]
    pub fn new(revision: u32) -> Self {
        let mut region = Self { data: [0u8; VMXON_REGION_SIZE] };
        region.write_revision(revision);
        region
    }

    /// Returns the revision identifier stored in the region header.
    #[must_use]
    pub fn revision(&self) -> u32 {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&self.data[..4]);
        u32::from_le_bytes(bytes)
    }

    /// Returns the raw region bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; VMXON_REGION_SIZE] {
        &self.data
    }

    fn write_revision(&mut self, revision: u32) {
        self.data[..4].copy_from_slice(&revision.to_le_bytes());
    }
}

impl core::fmt::Debug for VmxonRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VmxonRegion").field("revision", &self.revision()).finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{VmxonRegion, VMXON_REGION_SIZE};
    use core::mem::{align_of, size_of};

    #[test]
    fn vmxon_region_layout() {
        assert_eq!(size_of::<VmxonRegion>(), VMXON_REGION_SIZE);
        assert_eq!(align_of::<VmxonRegion>(), VMXON_REGION_SIZE);
        let region = VmxonRegion::new(0x1234_5678);
        assert_eq!(region.revision(), 0x1234_5678);
        assert!(region.as_bytes()[4..].iter().all(|byte| *byte == 0));
    }
}
