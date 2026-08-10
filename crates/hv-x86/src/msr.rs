//! Model-specific register access.

/// IA32_VMX_BASIC MSR.
pub const IA32_VMX_BASIC: u32 = 0x480;
/// IA32_FEATURE_CONTROL MSR.
pub const IA32_FEATURE_CONTROL: u32 = 0x3A;
/// IA32_VMX_PINBASED_CTLS MSR.
pub const IA32_VMX_PINBASED_CTLS: u32 = 0x481;
/// IA32_VMX_PROCBASED_CTLS MSR.
pub const IA32_VMX_PROCBASED_CTLS: u32 = 0x482;
/// IA32_VMX_EXIT_CTLS MSR.
pub const IA32_VMX_EXIT_CTLS: u32 = 0x483;
/// IA32_VMX_ENTRY_CTLS MSR.
pub const IA32_VMX_ENTRY_CTLS: u32 = 0x484;
/// IA32_VMX_PROCBASED_CTLS2 MSR.
pub const IA32_VMX_PROCBASED_CTLS2: u32 = 0x48B;
/// IA32_VMX_TRUE_PINBASED_CTLS MSR.
pub const IA32_VMX_TRUE_PINBASED_CTLS: u32 = 0x48D;
/// IA32_VMX_TRUE_PROCBASED_CTLS MSR.
pub const IA32_VMX_TRUE_PROCBASED_CTLS: u32 = 0x48E;
/// IA32_VMX_TRUE_EXIT_CTLS MSR.
pub const IA32_VMX_TRUE_EXIT_CTLS: u32 = 0x48F;
/// IA32_VMX_TRUE_ENTRY_CTLS MSR.
pub const IA32_VMX_TRUE_ENTRY_CTLS: u32 = 0x490;
/// IA32_VMX_CR0_FIXED0 MSR.
pub const IA32_VMX_CR0_FIXED0: u32 = 0x486;
/// IA32_VMX_CR0_FIXED1 MSR.
pub const IA32_VMX_CR0_FIXED1: u32 = 0x487;
/// IA32_VMX_CR4_FIXED0 MSR.
pub const IA32_VMX_CR4_FIXED0: u32 = 0x488;
/// IA32_VMX_CR4_FIXED1 MSR.
pub const IA32_VMX_CR4_FIXED1: u32 = 0x489;
/// IA32_EFER MSR.
pub const IA32_EFER: u32 = 0xC000_0080;
/// IA32_FS_BASE MSR.
pub const IA32_FS_BASE: u32 = 0xC000_0100;
/// IA32_GS_BASE MSR.
pub const IA32_GS_BASE: u32 = 0xC000_0101;
/// IA32_SYSENTER_CS MSR.
pub const IA32_SYSENTER_CS: u32 = 0x174;
/// IA32_SYSENTER_ESP MSR.
pub const IA32_SYSENTER_ESP: u32 = 0x175;
/// IA32_SYSENTER_EIP MSR.
pub const IA32_SYSENTER_EIP: u32 = 0x176;

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
