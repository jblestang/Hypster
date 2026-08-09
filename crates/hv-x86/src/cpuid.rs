//! CPUID leaf helpers.

use crate::error::X86Error;

/// Result registers from a CPUID invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuidLeaf {
    /// EAX.
    pub eax: u32,
    /// EBX.
    pub ebx: u32,
    /// ECX.
    pub ecx: u32,
    /// EDX.
    pub edx: u32,
}

/// Executes `CPUID` for the given leaf and sub-leaf.
///
/// # Errors
///
/// Returns [`X86Error::UnsupportedArchitecture`] when not compiled for x86_64.
pub fn cpuid(leaf: u32, subleaf: u32) -> Result<CpuidLeaf, X86Error> {
    #[cfg(target_arch = "x86_64")]
    {
        let result = unsafe { core::arch::x86_64::__cpuid_count(leaf, subleaf) };
        Ok(CpuidLeaf {
            eax: result.eax,
            ebx: result.ebx,
            ecx: result.ecx,
            edx: result.edx,
        })
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (leaf, subleaf);
        Err(X86Error::UnsupportedArchitecture)
    }
}

#[cfg(test)]
mod tests {
    use super::cpuid;

    #[test]
    fn cpuid_leaf_zero_returns_vendor_string() {
        let leaf = cpuid(0, 0).expect("cpuid leaf 0 should succeed on host");
        assert_ne!(leaf.eax, 0, "leaf 0 should report max basic leaf");
        assert_ne!(leaf.ebx, 0, "leaf 0 ebx should contain vendor string fragment");
        assert_ne!(leaf.ecx, 0, "leaf 0 ecx should contain vendor string fragment");
        assert_ne!(leaf.edx, 0, "leaf 0 edx should contain vendor string fragment");
    }
}
