//! VMX capability discovery from MSRs.

use hv_x86::msr::{
    IA32_VMX_BASIC, IA32_VMX_CR0_FIXED0, IA32_VMX_CR0_FIXED1, IA32_VMX_CR4_FIXED0,
    IA32_VMX_CR4_FIXED1,
};

use crate::error::VmxError;

/// Parsed VMX capabilities from IA32_VMX_BASIC and fixed MSRs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VmxCapabilities {
    /// VMCS revision identifier written to VMXON/VMCS regions.
    pub revision_id: u32,
    /// VMXON/VMCS region size in bytes.
    pub region_size: usize,
    /// IA32_VMX_CR0_FIXED0 value.
    pub cr0_fixed0: u64,
    /// IA32_VMX_CR0_FIXED1 value.
    pub cr0_fixed1: u64,
    /// IA32_VMX_CR4_FIXED0 value.
    pub cr4_fixed0: u64,
    /// IA32_VMX_CR4_FIXED1 value.
    pub cr4_fixed1: u64,
}

impl VmxCapabilities {
    /// Reads VMX capabilities from the current CPU.
    ///
    /// # Errors
    ///
    /// Returns [`VmxError::CapabilitiesUnavailable`] when MSRs cannot be read.
    pub fn from_hardware() -> Result<Self, VmxError> {
        // SAFETY: VMX capability MSRs are architecturally defined on VMX-capable CPUs.
        let basic = unsafe { hv_x86::read_msr(IA32_VMX_BASIC) }
            .ok_or(VmxError::CapabilitiesUnavailable)?;
        let cr0_fixed0 = unsafe { hv_x86::read_msr(IA32_VMX_CR0_FIXED0) }
            .ok_or(VmxError::CapabilitiesUnavailable)?;
        let cr0_fixed1 = unsafe { hv_x86::read_msr(IA32_VMX_CR0_FIXED1) }
            .ok_or(VmxError::CapabilitiesUnavailable)?;
        let cr4_fixed0 = unsafe { hv_x86::read_msr(IA32_VMX_CR4_FIXED0) }
            .ok_or(VmxError::CapabilitiesUnavailable)?;
        let cr4_fixed1 = unsafe { hv_x86::read_msr(IA32_VMX_CR4_FIXED1) }
            .ok_or(VmxError::CapabilitiesUnavailable)?;

        Ok(Self::from_vmx_basic(
            basic.raw(),
            cr0_fixed0.raw(),
            cr0_fixed1.raw(),
            cr4_fixed0.raw(),
            cr4_fixed1.raw(),
        ))
    }

    /// Returns assumed QEMU/KVM capability values for host unit tests.
    #[must_use]
    pub const fn from_assumed_qemu() -> Self {
        Self {
            revision_id: 0x0000_0001,
            region_size: 4096,
            cr0_fixed0: 0x8000_0001,
            cr0_fixed1: 0xFFFF_FFFF,
            cr4_fixed0: 0x0000_0000_0000_2000,
            cr4_fixed1: 0xFFFF_FFFF_FFFF_FFFF,
        }
    }

    const fn from_vmx_basic(
        basic: u64,
        cr0_fixed0: u64,
        cr0_fixed1: u64,
        cr4_fixed0: u64,
        cr4_fixed1: u64,
    ) -> Self {
        let revision_id = (basic & 0x7FFF_FFFF) as u32;
        let region_size = 4096;
        Self {
            revision_id,
            region_size,
            cr0_fixed0,
            cr0_fixed1,
            cr4_fixed0,
            cr4_fixed1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VmxCapabilities;

    #[test]
    fn assumed_qemu_revision_and_region_size() {
        let caps = VmxCapabilities::from_assumed_qemu();
        assert_eq!(caps.revision_id, 1);
        assert_eq!(caps.region_size, 4096);
    }
}
