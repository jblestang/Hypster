//! Runtime CPU capability probing via CPUID.

use hv_x86::cpuid;

use crate::error::CpuProbeError;

/// CPU and virtualization features observed via CPUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuCapabilities {
    /// VMX present in CPUID.1:ECX.
    pub vmx: bool,
    /// EPT advertised when extended leaf `0x8000000A` is available.
    pub ept: bool,
    /// VT-d is not discoverable via CPUID; defaults to false for ACPI override.
    pub vtd: bool,
    /// NX available in CPUID.1:EDX.
    pub nx: bool,
    /// Invariant TSC advertised in CPUID leaf `0x80000007`.
    pub invariant_tsc: bool,
    /// VPID supported when extended leaf `0x8000000A` is available.
    pub vpid: bool,
    /// VMX preemption timer supported when extended leaf `0x8000000A` is available.
    pub vmx_preemption_timer: bool,
    /// x2APIC supported via CPUID.1:ECX or extended leaf `0x8000001A`.
    pub x2apic: bool,
}

const CPUID_LEAF_VERSION_INFO: u32 = 1;
const CPUID_LEAF_EXTENDED_MAX: u32 = 0x8000_0000;
const CPUID_LEAF_VMX_FEATURES: u32 = 0x8000_000A;
const CPUID_LEAF_EXTENDED_APIC: u32 = 0x8000_001A;
const CPUID_LEAF_POWER_MANAGEMENT: u32 = 0x8000_0007;

const VMX_BIT: u32 = 1 << 5;
const X2APIC_BIT: u32 = 1 << 21;
const NX_BIT: u32 = 1 << 20;
const INVARIANT_TSC_BIT: u32 = 1 << 8;

const EPT_BIT: u32 = 1 << 0;
const VPID_BIT: u32 = 1 << 5;
const VMX_PREEMPTION_TIMER_BIT: u32 = 1 << 6;

const fn bit_set(value: u32, bit: u32) -> bool {
    value & bit != 0
}

fn max_extended_leaf() -> Result<u32, CpuProbeError> {
    let leaf = cpuid::cpuid(CPUID_LEAF_EXTENDED_MAX, 0).map_err(map_x86_error)?;
    Ok(leaf.eax)
}

fn vmx_extended_features_available(max_extended: u32) -> bool {
    max_extended >= CPUID_LEAF_VMX_FEATURES
}

/// Probes CPU capabilities from CPUID leaves available on the current processor.
///
/// VT-d is always reported as false here; firmware ACPI tables may override it
/// later during platform observation.
///
/// # Errors
///
/// Returns [`CpuProbeError::UnsupportedArchitecture`] when CPUID is unavailable.
pub fn probe_cpu_capabilities() -> Result<CpuCapabilities, CpuProbeError> {
    let version = cpuid::cpuid(CPUID_LEAF_VERSION_INFO, 0).map_err(map_x86_error)?;

    let vmx = bit_set(version.ecx, VMX_BIT);
    let nx = bit_set(version.edx, NX_BIT);
    let x2apic_leaf1 = bit_set(version.ecx, X2APIC_BIT);

    let max_extended = max_extended_leaf()?;
    let x2apic_extended = if max_extended >= CPUID_LEAF_EXTENDED_APIC {
        cpuid::cpuid(CPUID_LEAF_EXTENDED_APIC, 0)
            .map_err(map_x86_error)?
            .ebx
            & 1
            != 0
    } else {
        false
    };

    let invariant_tsc = if max_extended >= CPUID_LEAF_POWER_MANAGEMENT {
        cpuid::cpuid(CPUID_LEAF_POWER_MANAGEMENT, 0)
            .map_err(map_x86_error)?
            .edx
            & INVARIANT_TSC_BIT
            != 0
    } else {
        false
    };

    let (ept, vpid, vmx_preemption_timer) = if vmx_extended_features_available(max_extended) {
        let vmx_features = cpuid::cpuid(CPUID_LEAF_VMX_FEATURES, 0).map_err(map_x86_error)?;
        (
            bit_set(vmx_features.ebx, EPT_BIT),
            bit_set(vmx_features.ebx, VPID_BIT),
            bit_set(vmx_features.ebx, VMX_PREEMPTION_TIMER_BIT),
        )
    } else {
        (false, false, false)
    };

    Ok(CpuCapabilities {
        vmx,
        ept,
        vtd: false,
        nx,
        invariant_tsc,
        vpid,
        vmx_preemption_timer,
        x2apic: x2apic_leaf1 || x2apic_extended,
    })
}

/// Validates that the CPU exposes the minimum features required for VMX bring-up.
///
/// # Errors
///
/// Returns [`CpuProbeError::MissingRequiredFeature`] when VMX, EPT, or NX is absent.
pub fn validate_vmx_prerequisites(cap: &CpuCapabilities) -> Result<(), CpuProbeError> {
    if !cap.vmx {
        return Err(CpuProbeError::MissingRequiredFeature { feature: "vmx" });
    }
    if !cap.ept {
        return Err(CpuProbeError::MissingRequiredFeature { feature: "ept" });
    }
    if !cap.nx {
        return Err(CpuProbeError::MissingRequiredFeature { feature: "nx" });
    }
    Ok(())
}

fn map_x86_error(err: hv_x86::X86Error) -> CpuProbeError {
    match err {
        hv_x86::X86Error::UnsupportedArchitecture => CpuProbeError::UnsupportedArchitecture,
    }
}

#[cfg(test)]
mod tests {
    use super::{probe_cpu_capabilities, validate_vmx_prerequisites, CpuCapabilities};

    #[test]
    fn probe_cpu_capabilities_returns_structured_result() {
        let caps = probe_cpu_capabilities().expect("host cpuid probe should succeed");
        let _ = (
            caps.vmx,
            caps.ept,
            caps.vtd,
            caps.nx,
            caps.invariant_tsc,
            caps.vpid,
            caps.vmx_preemption_timer,
            caps.x2apic,
        );
        assert!(!caps.vtd, "vtd must remain false until ACPI override");
    }

    #[test]
    fn validate_vmx_prerequisites_accepts_capable_profile() {
        let caps = CpuCapabilities {
            vmx: true,
            ept: true,
            vtd: false,
            nx: true,
            invariant_tsc: true,
            vpid: true,
            vmx_preemption_timer: true,
            x2apic: true,
        };
        validate_vmx_prerequisites(&caps).expect("capable profile should validate");
    }

    #[test]
    fn validate_vmx_prerequisites_rejects_missing_nx() {
        let caps = CpuCapabilities {
            vmx: true,
            ept: true,
            vtd: false,
            nx: false,
            invariant_tsc: true,
            vpid: true,
            vmx_preemption_timer: true,
            x2apic: true,
        };
        let err = validate_vmx_prerequisites(&caps).expect_err("missing nx should fail");
        assert!(matches!(
            err,
            crate::CpuProbeError::MissingRequiredFeature { feature: "nx" }
        ));
    }
}
