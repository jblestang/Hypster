//! VMCS region layout and field encodings.

/// Size of a VMCS region in bytes.
pub const VMCS_REGION_SIZE: usize = 4096;

/// 16-bit VMCS component width encoding.
pub const VMCS_WIDTH_16: u32 = 0;
/// 64-bit VMCS component width encoding.
pub const VMCS_WIDTH_64: u32 = 1;
/// 32-bit VMCS component width encoding.
pub const VMCS_WIDTH_32: u32 = 2;
/// Natural-width VMCS component width encoding.
pub const VMCS_WIDTH_NATURAL: u32 = 3;

/// Encodes a VMCS field number with width and type.
#[must_use]
pub const fn vmcs_field(width: u32, index: u32) -> u32 {
    (width << 13) | index
}

/// Host selector for the TR segment.
pub const HOST_TR_SELECTOR: u32 = vmcs_field(VMCS_WIDTH_16, 0x0000);
/// Host selector for the FS segment.
pub const HOST_FS_SELECTOR: u32 = vmcs_field(VMCS_WIDTH_16, 0x0002);
/// Host selector for the GS segment.
pub const HOST_GS_SELECTOR: u32 = vmcs_field(VMCS_WIDTH_16, 0x0004);
/// Host selector for the LDTR segment.
pub const HOST_LDTR_SELECTOR: u32 = vmcs_field(VMCS_WIDTH_16, 0x0006);

/// Host CR0.
pub const HOST_CR0: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6C00);
/// Host CR3.
pub const HOST_CR3: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6C02);
/// Host CR4.
pub const HOST_CR4: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6C04);

/// Host FS base.
pub const HOST_FS_BASE: u32 = vmcs_field(VMCS_WIDTH_64, 0x6C06);
/// Host GS base.
pub const HOST_GS_BASE: u32 = vmcs_field(VMCS_WIDTH_64, 0x6C08);
/// Host TR base.
pub const HOST_TR_BASE: u32 = vmcs_field(VMCS_WIDTH_64, 0x6C0A);
/// Host GDTR base.
pub const HOST_GDTR_BASE: u32 = vmcs_field(VMCS_WIDTH_64, 0x6C0C);
/// Host IDTR base.
pub const HOST_IDTR_BASE: u32 = vmcs_field(VMCS_WIDTH_64, 0x6C0E);

/// Host IA32_SYSENTER CS.
pub const HOST_SYSENTER_CS: u32 = vmcs_field(VMCS_WIDTH_32, 0x4C00);
/// Host IA32_SYSENTER ESP.
pub const HOST_SYSENTER_ESP: u32 = vmcs_field(VMCS_WIDTH_64, 0x6C10);
/// Host IA32_SYSENTER EIP.
pub const HOST_SYSENTER_EIP: u32 = vmcs_field(VMCS_WIDTH_64, 0x6C12);
/// Host RSP.
pub const HOST_RSP: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6C14);
/// Host RIP.
pub const HOST_RIP: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6C16);

/// Guest CR0.
pub const GUEST_CR0: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6800);
/// Guest CR3.
pub const GUEST_CR3: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6802);
/// Guest CR4.
pub const GUEST_CR4: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6804);
/// Guest RSP.
pub const GUEST_RSP: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x681C);
/// Guest RIP.
pub const GUEST_RIP: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x681E);
/// Guest RFLAGS.
pub const GUEST_RFLAGS: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6820);

/// VMCS link pointer (64-bit).
pub const VMCS_LINK_POINTER: u32 = vmcs_field(VMCS_WIDTH_64, 0x2800);
/// EPT pointer (64-bit).
pub const EPT_POINTER: u32 = vmcs_field(VMCS_WIDTH_64, 0x201A);

/// Pin-based VM execution controls.
pub const PIN_BASED_VM_EXEC_CONTROL: u32 = vmcs_field(VMCS_WIDTH_32, 0x4000);
/// Primary processor-based VM execution controls.
pub const PROC_BASED_VM_EXEC_CONTROL: u32 = vmcs_field(VMCS_WIDTH_32, 0x401E);
/// Secondary processor-based VM execution controls.
pub const SECONDARY_PROC_BASED_VM_EXEC_CONTROL: u32 = vmcs_field(VMCS_WIDTH_32, 0x401B);
/// VM-exit controls.
pub const VM_EXIT_CONTROLS: u32 = vmcs_field(VMCS_WIDTH_32, 0x400C);
/// VM-entry controls.
pub const VM_ENTRY_CONTROLS: u32 = vmcs_field(VMCS_WIDTH_32, 0x4012);

/// 4 KiB-aligned VMCS region.
#[repr(C, align(4096))]
#[derive(Clone, Copy)]
pub struct VmcsRegion {
    data: [u8; VMCS_REGION_SIZE],
}

impl VmcsRegion {
    /// Creates a zeroed VMCS region with the revision identifier written at offset 0.
    #[must_use]
    pub fn new(revision: u32) -> Self {
        let mut region = Self { data: [0u8; VMCS_REGION_SIZE] };
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
    pub fn as_bytes(&self) -> &[u8; VMCS_REGION_SIZE] {
        &self.data
    }

    fn write_revision(&mut self, revision: u32) {
        self.data[..4].copy_from_slice(&revision.to_le_bytes());
    }
}

impl core::fmt::Debug for VmcsRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VmcsRegion").field("revision", &self.revision()).finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{VmcsRegion, HOST_CR3, HOST_RIP, VMCS_REGION_SIZE, VMCS_WIDTH_NATURAL};
    use core::mem::{align_of, size_of};

    #[test]
    fn vmcs_region_layout() {
        assert_eq!(size_of::<VmcsRegion>(), VMCS_REGION_SIZE);
        assert_eq!(align_of::<VmcsRegion>(), VMCS_REGION_SIZE);
        let region = VmcsRegion::new(1);
        assert_eq!(region.revision(), 1);
    }

    #[test]
    fn host_field_constants_use_natural_width() {
        assert_eq!(HOST_CR3 & (3 << 13), VMCS_WIDTH_NATURAL << 13);
        assert_eq!(HOST_RIP & (3 << 13), VMCS_WIDTH_NATURAL << 13);
    }
}
