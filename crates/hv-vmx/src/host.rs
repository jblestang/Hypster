//! VMX host enablement state machine.

use hv_types::HostPhysAddr;
use hv_x86::cr::Cr4Flags;
use hv_x86::msr::IA32_FEATURE_CONTROL;

use crate::caps::VmxCapabilities;
use crate::error::VmxError;
use crate::instr;

const FEATURE_CONTROL_LOCK: u64 = 1 << 0;
const FEATURE_CONTROL_VMX_OUTSIDE_SMX: u64 = 1 << 2;

/// VMX host bring-up state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmxHostState {
    /// Initial state before VMX enablement.
    Initial,
    /// VMX enabled (CR4.VMXE set, VMXON executed).
    Vmxon,
}

/// VMX host initialization context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VmxHostInit {
    state: VmxHostState,
    caps: VmxCapabilities,
}

impl VmxHostInit {
    /// Creates a host init context using hardware capability discovery.
    ///
    /// # Errors
    ///
    /// Returns [`VmxError::CapabilitiesUnavailable`] when MSRs cannot be read.
    pub fn from_hardware() -> Result<Self, VmxError> {
        Ok(Self { state: VmxHostState::Initial, caps: VmxCapabilities::from_hardware()? })
    }

    /// Creates a host init context from assumed QEMU capabilities (tests).
    #[must_use]
    pub const fn from_assumed_qemu() -> Self {
        Self { state: VmxHostState::Initial, caps: VmxCapabilities::from_assumed_qemu() }
    }

    /// Creates a host init context with explicit capabilities.
    #[must_use]
    pub const fn new(caps: VmxCapabilities) -> Self {
        Self { state: VmxHostState::Initial, caps }
    }

    /// Returns the current state.
    #[must_use]
    pub const fn state(&self) -> VmxHostState {
        self.state
    }

    /// Returns parsed VMX capabilities.
    #[must_use]
    pub const fn capabilities(&self) -> VmxCapabilities {
        self.caps
    }

    /// Enables VMX and executes VMXON using the provided VMXON region HPA.
    ///
    /// # Errors
    ///
    /// Returns [`VmxError`] when prerequisites are not met or VMXON fails.
    ///
    /// # Safety
    ///
    /// Caller must ensure `vmxon_region_hpa` points to a valid, 4 KiB-aligned VMXON region
    /// initialized with the correct revision identifier.
    pub unsafe fn try_enable_vmx(
        &mut self,
        vmxon_region_hpa: HostPhysAddr,
    ) -> Result<(), VmxError> {
        if self.state == VmxHostState::Vmxon {
            return Ok(());
        }

        if vmxon_region_hpa.raw() & 0xFFF != 0 {
            return Err(VmxError::MisalignedVmxonRegion);
        }

        let cr0 = hv_x86::read_cr0().ok_or(VmxError::Cr4Unavailable)?;
        let new_cr0 = (cr0 | self.caps.cr0_fixed0) & self.caps.cr0_fixed1;
        if new_cr0 != cr0 && !unsafe { hv_x86::write_cr0(new_cr0) } {
            return Err(VmxError::Cr0UpdateFailed);
        }

        let cr4 = hv_x86::read_cr4().ok_or(VmxError::Cr4Unavailable)?;
        if (cr4 & Cr4Flags::VMXE) != 0 {
            return Err(VmxError::VmxAlreadyEnabled);
        }

        let feature_control = unsafe { hv_x86::read_msr(IA32_FEATURE_CONTROL) }
            .ok_or(VmxError::FeatureControlUnavailable)?;
        let fc = feature_control.raw();
        let locked = (fc & FEATURE_CONTROL_LOCK) != 0;
        let vmx_outside_smx = (fc & FEATURE_CONTROL_VMX_OUTSIDE_SMX) != 0;
        if locked && !vmx_outside_smx {
            return Err(VmxError::FeatureControlLocked);
        }
        if !vmx_outside_smx {
            let updated = fc | FEATURE_CONTROL_VMX_OUTSIDE_SMX;
            if !unsafe { hv_x86::write_msr(IA32_FEATURE_CONTROL, updated) } {
                return Err(VmxError::FeatureControlUnavailable);
            }
        }

        let new_cr4 = (cr4 | Cr4Flags::VMXE) & self.caps.cr4_fixed1 | self.caps.cr4_fixed0;
        if !unsafe { hv_x86::write_cr4(new_cr4) } {
            return Err(VmxError::Cr4UpdateFailed);
        }

        // SAFETY: caller guarantees VMXON region validity; VMX is now enabled.
        unsafe { instr::vmxon(vmxon_region_hpa.raw())? };

        self.state = VmxHostState::Vmxon;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{VmxHostInit, VmxHostState};

    #[test]
    fn assumed_qemu_starts_in_initial_state() {
        let init = VmxHostInit::from_assumed_qemu();
        assert_eq!(init.state(), VmxHostState::Initial);
        assert_eq!(init.capabilities().revision_id, 1);
    }
}
