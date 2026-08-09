//! VMX hardware programming errors.

use core::fmt;

/// Errors produced while enabling VMX or managing VMCS regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmxError {
    /// Target architecture does not support VMX instructions.
    UnsupportedArch,
    /// CPU does not expose CR4 (non-x86 host).
    Cr4Unavailable,
    /// CR4.VMXE is already set; caller must follow the already-enabled path.
    VmxAlreadyEnabled,
    /// IA32_FEATURE_CONTROL could not be read.
    FeatureControlUnavailable,
    /// IA32_FEATURE_CONTROL locked without VMX enabled outside SMX.
    FeatureControlLocked,
    /// VMX is disabled in IA32_FEATURE_CONTROL.
    VmxDisabledInFeatureControl,
    /// VMX capability MSRs could not be read.
    CapabilitiesUnavailable,
    /// CR4 could not be updated with VMXE.
    Cr4UpdateFailed,
    /// VMXON instruction failed.
    VmxonFailed,
    /// VMXON region physical address is not 4 KiB aligned.
    MisalignedVmxonRegion,
    /// VMCS region physical address is not 4 KiB aligned.
    MisalignedVmcsRegion,
    /// VMCLEAR instruction failed.
    VmclearFailed,
    /// VMPTRLD instruction failed.
    VmptrldFailed,
    /// VMREAD instruction failed.
    VmreadFailed,
    /// VMWRITE instruction failed.
    VmwriteFailed,
    /// VMLAUNCH instruction failed.
    VmlaunchFailed,
    /// Guest launch plan violates alignment or layout invariants.
    InvalidGuestLaunchPlan,
    /// EPT root is not page aligned.
    MisalignedEptRoot,
}

impl fmt::Display for VmxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedArch => f.write_str("VMX unsupported on this architecture"),
            Self::VmxAlreadyEnabled => f.write_str("CR4.VMXE already set"),
            Self::Cr4Unavailable => f.write_str("CR4 unavailable"),
            Self::FeatureControlUnavailable => f.write_str("IA32_FEATURE_CONTROL unavailable"),
            Self::FeatureControlLocked => {
                f.write_str("IA32_FEATURE_CONTROL locked without VMX enable")
            }
            Self::VmxDisabledInFeatureControl => f.write_str("VMX disabled in feature control"),
            Self::CapabilitiesUnavailable => f.write_str("VMX capability MSRs unavailable"),
            Self::Cr4UpdateFailed => f.write_str("failed to set CR4.VMXE"),
            Self::VmxonFailed => f.write_str("VMXON failed"),
            Self::MisalignedVmxonRegion => f.write_str("VMXON region not 4 KiB aligned"),
            Self::MisalignedVmcsRegion => f.write_str("VMCS region not 4 KiB aligned"),
            Self::VmclearFailed => f.write_str("VMCLEAR failed"),
            Self::VmptrldFailed => f.write_str("VMPTRLD failed"),
            Self::VmreadFailed => f.write_str("VMREAD failed"),
            Self::VmwriteFailed => f.write_str("VMWRITE failed"),
            Self::VmlaunchFailed => f.write_str("VMLAUNCH failed"),
            Self::InvalidGuestLaunchPlan => f.write_str("invalid guest launch plan"),
            Self::MisalignedEptRoot => f.write_str("EPT root not page aligned"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VmxError {}
