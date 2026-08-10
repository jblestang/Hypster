//! CPUID-based feature detection.

use crate::error::CpuProbeError;

/// Observed CPU virtualization features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuFeatures {
    /// VMX present.
    pub vmx: bool,
    /// EPT present.
    pub ept: bool,
    /// NX available.
    pub nx: bool,
}

impl CpuFeatures {
    /// Returns true when VMX, EPT, and NX are all available.
    #[must_use]
    pub const fn gate_c_ready(self) -> bool {
        self.vmx && self.ept && self.nx
    }
}

/// Probes CPU features without enabling VMX.
///
/// # Errors
///
/// Returns [`CpuProbeError`] when CPUID is unavailable or required features
/// are missing.
pub fn probe_cpu() -> Result<CpuFeatures, CpuProbeError> {
    let features = raw_probe().ok_or(CpuProbeError::CpuidUnavailable)?;
    if !features.vmx {
        return Err(CpuProbeError::MissingVmx);
    }
    if !features.ept {
        return Err(CpuProbeError::MissingEpt);
    }
    if !features.nx {
        return Err(CpuProbeError::MissingNx);
    }
    Ok(features)
}

/// Returns raw CPU features without failing on missing capabilities.
#[must_use]
pub fn probe_cpu_optional() -> Option<CpuFeatures> {
    raw_probe()
}

fn raw_probe() -> Option<CpuFeatures> {
    #[cfg(target_arch = "x86_64")]
    {
        let leaf0 = cpuid(0)?;
        if leaf0.eax == 0 {
            return None;
        }
        let leaf1 = cpuid(1)?;
        let vmx = (leaf1.ecx & (1 << 5)) != 0;
        let nx_leaf1 = (leaf1.edx & (1 << 20)) != 0;

        let max_ext = cpuid(0x8000_0000).map(|leaf| leaf.eax).unwrap_or(0);
        let (nx, ept) = if max_ext >= 0x8000_0001 {
            let leaf_ext = cpuid(0x8000_0001)?;
            let nx = nx_leaf1 || (leaf_ext.edx & (1 << 20)) != 0;
            let ept = if max_ext >= 0x8000_0008 {
                cpuid(0x8000_0008).map(|leaf| (leaf.ebx & (1 << 0)) != 0).unwrap_or(vmx)
            } else {
                vmx
            };
            (nx, ept)
        } else {
            (nx_leaf1, vmx)
        };

        Some(CpuFeatures { vmx, ept, nx })
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        None
    }
}

#[derive(Clone, Copy)]
struct CpuidLeaf {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
}

#[cfg(target_arch = "x86_64")]
fn cpuid(leaf: u32) -> Option<CpuidLeaf> {
    let result = unsafe { core::arch::x86_64::__cpuid(leaf) };
    Some(CpuidLeaf { eax: result.eax, ebx: result.ebx, ecx: result.ecx, edx: result.edx })
}

#[cfg(not(target_arch = "x86_64"))]
fn cpuid(_leaf: u32) -> Option<CpuidLeaf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_cpu_on_host() {
        let _ = probe_cpu_optional();
    }
}
