//! Model-specific register access.

/// IA32_VMX_BASIC MSR.
pub const IA32_VMX_BASIC: u32 = 0x480;
/// IA32_FEATURE_CONTROL MSR.
pub const IA32_FEATURE_CONTROL: u32 = 0x3A;
/// IA32_VMX_CR0_FIXED0 MSR.
pub const IA32_VMX_CR0_FIXED0: u32 = 0x486;
/// IA32_VMX_CR0_FIXED1 MSR.
pub const IA32_VMX_CR0_FIXED1: u32 = 0x487;
/// IA32_VMX_CR4_FIXED0 MSR.
pub const IA32_VMX_CR4_FIXED0: u32 = 0x488;
/// IA32_VMX_CR4_FIXED1 MSR.
pub const IA32_VMX_CR4_FIXED1: u32 = 0x489;

/// Raw MSR value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Msr(pub u64);

impl Msr {
    /// Creates an MSR value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw 64-bit value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Reads a model-specific register.
///
/// # Errors
///
/// Returns `None` when the target architecture does not support MSR reads.
///
/// # Safety
///
/// Caller must ensure the MSR index is valid on the current CPU.
#[must_use]
pub unsafe fn read_msr(index: u32) -> Option<Msr> {
    read_msr_impl(index).map(Msr::new)
}

/// Writes a model-specific register.
///
/// # Errors
///
/// Returns `false` when the target architecture does not support MSR writes.
///
/// # Safety
///
/// Caller must ensure the MSR index and value are valid on the current CPU.
#[must_use]
pub unsafe fn write_msr(index: u32, value: u64) -> bool {
    write_msr_impl(index, value)
}

#[cfg(target_arch = "x86_64")]
fn read_msr_impl(index: u32) -> Option<u64> {
    let low: u32;
    let high: u32;
    // SAFETY: caller guarantees MSR validity; RDMSR is defined on x86_64 hosts.
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") index,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags),
        );
    }
    Some(((high as u64) << 32) | low as u64)
}

#[cfg(not(target_arch = "x86_64"))]
fn read_msr_impl(_index: u32) -> Option<u64> {
    None
}

#[cfg(target_arch = "x86_64")]
fn write_msr_impl(index: u32, value: u64) -> bool {
    let low = value as u32;
    let high = (value >> 32) as u32;
    // SAFETY: caller guarantees MSR validity; WRMSR is defined on x86_64 hosts.
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") index,
            in("eax") low,
            in("edx") high,
            options(nostack, preserves_flags),
        );
    }
    true
}

#[cfg(not(target_arch = "x86_64"))]
fn write_msr_impl(_index: u32, _value: u64) -> bool {
    false
}
