//! Model-specific register helpers.

use crate::error::X86Error;

/// `IA32_FEATURE_CONTROL` — BIOS lock and VMX enable bits.
pub const IA32_FEATURE_CONTROL: u32 = 0x3A;
/// `IA32_VMX_BASIC` — VMX capability/version MSR.
pub const IA32_VMX_BASIC: u32 = 0x480;
/// `IA32_VMX_CR0_FIXED0` — required CR0 bits when VMX is enabled.
pub const IA32_VMX_CR0_FIXED0: u32 = 0x486;
/// `IA32_VMX_CR0_FIXED1` — restricted CR0 bits when VMX is enabled.
pub const IA32_VMX_CR0_FIXED1: u32 = 0x487;
/// `IA32_VMX_CR4_FIXED0` — required CR4 bits when VMX is enabled.
pub const IA32_VMX_CR4_FIXED0: u32 = 0x488;
/// `IA32_VMX_CR4_FIXED1` — restricted CR4 bits when VMX is enabled.
pub const IA32_VMX_CR4_FIXED1: u32 = 0x489;
/// `IA32_EFER` — extended feature enable register.
pub const IA32_EFER: u32 = 0xC000_0080;

/// Reads a 64-bit model-specific register.
///
/// # Safety
///
/// The caller must ensure the MSR index is valid for the current CPU and that
/// reading it will not cause a fault in the current privilege level.
///
/// # Errors
///
/// Returns [`X86Error::UnsupportedArchitecture`] when not compiled for x86_64.
pub unsafe fn rdmsr(msr: u32) -> Result<u64, X86Error> {
    #[cfg(target_arch = "x86_64")]
    {
        let low: u32;
        let high: u32;
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags)
        );
        Ok(((high as u64) << 32) | u64::from(low))
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = msr;
        Err(X86Error::UnsupportedArchitecture)
    }
}

/// Writes a 64-bit model-specific register.
///
/// # Safety
///
/// The caller must ensure the MSR index is valid for the current CPU and that
/// the value is legal for the platform.
///
/// # Errors
///
/// Returns [`X86Error::UnsupportedArchitecture`] when not compiled for x86_64.
pub unsafe fn wrmsr(msr: u32, value: u64) -> Result<(), X86Error> {
    #[cfg(target_arch = "x86_64")]
    {
        let low = value as u32;
        let high = (value >> 32) as u32;
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nostack, preserves_flags)
        );
        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (msr, value);
        Err(X86Error::UnsupportedArchitecture)
    }
}
