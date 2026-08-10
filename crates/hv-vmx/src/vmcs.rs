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

/// Host ES selector.
pub const HOST_ES_SELECTOR: u32 = 0x0C00;
/// Host CS selector.
pub const HOST_CS_SELECTOR: u32 = 0x0C02;
/// Host SS selector.
pub const HOST_SS_SELECTOR: u32 = 0x0C04;
/// Host DS selector.
pub const HOST_DS_SELECTOR: u32 = 0x0C06;
/// Host FS selector.
pub const HOST_FS_SELECTOR: u32 = 0x0C08;
/// Host GS selector.
pub const HOST_GS_SELECTOR: u32 = 0x0C0A;
/// Host TR selector.
pub const HOST_TR_SELECTOR: u32 = 0x0C0C;

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

/// Guest ES selector.
pub const GUEST_ES_SELECTOR: u32 = 0x0800;
/// Guest CS selector.
pub const GUEST_CS_SELECTOR: u32 = 0x0802;
/// Guest SS selector.
pub const GUEST_SS_SELECTOR: u32 = 0x0804;
/// Guest DS selector.
pub const GUEST_DS_SELECTOR: u32 = 0x0806;
/// Guest FS selector.
pub const GUEST_FS_SELECTOR: u32 = 0x0808;
/// Guest GS selector.
pub const GUEST_GS_SELECTOR: u32 = 0x080A;
/// Guest LDTR selector.
pub const GUEST_LDTR_SELECTOR: u32 = 0x080C;
/// Guest TR selector.
pub const GUEST_TR_SELECTOR: u32 = 0x080E;

/// Guest CR0.
pub const GUEST_CR0: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6800);
/// Guest CR3.
pub const GUEST_CR3: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6802);
/// Guest CR4.
pub const GUEST_CR4: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6804);
/// Guest ES base.
pub const GUEST_ES_BASE: u32 = 0x6806;
/// Guest CS base.
pub const GUEST_CS_BASE: u32 = 0x6808;
/// Guest SS base.
pub const GUEST_SS_BASE: u32 = 0x680A;
/// Guest DS base.
pub const GUEST_DS_BASE: u32 = 0x680C;
/// Guest FS base.
pub const GUEST_FS_BASE: u32 = 0x680E;
/// Guest GS base.
pub const GUEST_GS_BASE: u32 = 0x6810;
/// Guest LDTR base.
pub const GUEST_LDTR_BASE: u32 = 0x6812;
/// Guest TR base.
pub const GUEST_TR_BASE: u32 = 0x6814;
/// Guest GDTR base.
pub const GUEST_GDTR_BASE: u32 = 0x6816;
/// Guest IDTR base.
pub const GUEST_IDTR_BASE: u32 = 0x6818;
/// Guest DR7.
pub const GUEST_DR7: u32 = 0x681A;
/// Guest RSP.
pub const GUEST_RSP: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x681C);
/// Guest RIP.
pub const GUEST_RIP: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x681E);
/// Guest RFLAGS.
pub const GUEST_RFLAGS: u32 = vmcs_field(VMCS_WIDTH_NATURAL, 0x6820);
/// Guest pending debug exceptions.
pub const GUEST_PENDING_DBG_EXCEPTIONS: u32 = 0x6822;
/// Guest IA32_SYSENTER_ESP.
pub const GUEST_SYSENTER_ESP: u32 = 0x6824;
/// Guest IA32_SYSENTER_EIP.
pub const GUEST_SYSENTER_EIP: u32 = 0x6826;
/// Guest ES limit.
pub const GUEST_ES_LIMIT: u32 = 0x4800;
/// Guest CS limit.
pub const GUEST_CS_LIMIT: u32 = 0x4802;
/// Guest SS limit.
pub const GUEST_SS_LIMIT: u32 = 0x4804;
/// Guest DS limit.
pub const GUEST_DS_LIMIT: u32 = 0x4806;
/// Guest FS limit.
pub const GUEST_FS_LIMIT: u32 = 0x4808;
/// Guest GS limit.
pub const GUEST_GS_LIMIT: u32 = 0x480A;
/// Guest LDTR limit.
pub const GUEST_LDTR_LIMIT: u32 = 0x480C;
/// Guest TR limit.
pub const GUEST_TR_LIMIT: u32 = 0x480E;
/// Guest GDTR limit.
pub const GUEST_GDTR_LIMIT: u32 = 0x4810;
/// Guest IDTR limit.
pub const GUEST_IDTR_LIMIT: u32 = 0x4812;
/// Guest ES access rights.
pub const GUEST_ES_AR_BYTES: u32 = 0x4814;
/// Guest CS access rights.
pub const GUEST_CS_AR_BYTES: u32 = 0x4816;
/// Guest SS access rights.
pub const GUEST_SS_AR_BYTES: u32 = 0x4818;
/// Guest DS access rights.
pub const GUEST_DS_AR_BYTES: u32 = 0x481A;
/// Guest FS access rights.
pub const GUEST_FS_AR_BYTES: u32 = 0x481C;
/// Guest GS access rights.
pub const GUEST_GS_AR_BYTES: u32 = 0x481E;
/// Guest LDTR access rights.
pub const GUEST_LDTR_AR_BYTES: u32 = 0x4820;
/// Guest TR access rights.
pub const GUEST_TR_AR_BYTES: u32 = 0x4822;
/// Guest interruptibility state.
pub const GUEST_INTERRUPTIBILITY_STATE: u32 = 0x4824;
/// Guest activity state.
pub const GUEST_ACTIVITY_STATE: u32 = 0x4826;
/// Guest IA32_SYSENTER_CS.
pub const GUEST_SYSENTER_CS: u32 = 0x482A;
/// Guest IA32_DEBUGCTL.
pub const GUEST_IA32_DEBUGCTL: u32 = 0x2802;
/// Guest IA32_EFER.
pub const GUEST_IA32_EFER: u32 = 0x2806;
/// Host IA32_EFER.
pub const HOST_IA32_EFER: u32 = 0x2C02;
/// MSR bitmap address.
pub const MSR_BITMAP: u32 = 0x2004;

/// VMCS link pointer (64-bit).
pub const VMCS_LINK_POINTER: u32 = vmcs_field(VMCS_WIDTH_64, 0x2800);
/// EPT pointer (64-bit).
pub const EPT_POINTER: u32 = vmcs_field(VMCS_WIDTH_64, 0x201A);

/// Pin-based VM execution controls.
pub const PIN_BASED_VM_EXEC_CONTROL: u32 = 0x4000;
/// Primary processor-based VM execution controls.
pub const PROC_BASED_VM_EXEC_CONTROL: u32 = 0x4002;
/// Secondary processor-based VM execution controls.
pub const SECONDARY_PROC_BASED_VM_EXEC_CONTROL: u32 = 0x401E;
/// VM-exit controls.
pub const VM_EXIT_CONTROLS: u32 = 0x400C;
/// VM-entry controls.
pub const VM_ENTRY_CONTROLS: u32 = 0x4012;
/// VM-instruction error (valid after VMfailValid).
pub const VM_INSTRUCTION_ERROR: u32 = 0x4400;
/// VM-exit reason.
pub const VM_EXIT_REASON: u32 = vmcs_field(VMCS_WIDTH_32, 0x4402);
/// VM-exit instruction length.
pub const VM_EXIT_INSTRUCTION_LEN: u32 = vmcs_field(VMCS_WIDTH_32, 0x440C);
/// Guest-physical address at VM-exit.
pub const GUEST_PHYSICAL_ADDRESS: u32 = vmcs_field(VMCS_WIDTH_64, 0x2400);
/// Exit qualification for the current VM-exit.
pub const EXIT_QUALIFICATION: u32 = vmcs_field(VMCS_WIDTH_64, 0x6402);

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
