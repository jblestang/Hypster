//! Control register access.

/// CR4 flag bits used by VMX bring-up.
pub struct Cr4Flags;

impl Cr4Flags {
    /// VMX enable bit (bit 13).
    pub const VMXE: u64 = 1 << 13;
}

/// Reads CR0.
///
/// # Errors
///
/// Returns `None` when the target architecture does not support CR0 reads.
#[must_use]
pub fn read_cr0() -> Option<u64> {
    read_cr0_impl()
}

/// Writes CR0.
///
/// # Errors
///
/// Returns `false` when the target architecture does not support CR0 writes.
///
/// # Safety
///
/// Caller must ensure the value satisfies platform fixed-bit constraints.
#[must_use]
pub unsafe fn write_cr0(value: u64) -> bool {
    write_cr0_impl(value)
}

/// Reads CR4.
///
/// # Errors
///
/// Returns `None` when the target architecture does not support CR4 reads.
#[must_use]
pub fn read_cr4() -> Option<u64> {
    read_cr4_impl()
}

/// Writes CR4.
///
/// # Errors
///
/// Returns `false` when the target architecture does not support CR4 writes.
///
/// # Safety
///
/// Caller must ensure the value satisfies platform fixed-bit constraints.
#[must_use]
pub unsafe fn write_cr4(value: u64) -> bool {
    write_cr4_impl(value)
}

#[cfg(target_arch = "x86_64")]
fn read_cr0_impl() -> Option<u64> {
    let value: u64;
    // SAFETY: MOV CR0 is defined on x86_64 hosts.
    unsafe {
        core::arch::asm!(
            "mov {0}, cr0",
            out(reg) value,
            options(nostack, preserves_flags),
        );
    }
    Some(value)
}

#[cfg(not(target_arch = "x86_64"))]
fn read_cr0_impl() -> Option<u64> {
    None
}

#[cfg(target_arch = "x86_64")]
fn write_cr0_impl(value: u64) -> bool {
    // SAFETY: caller validates fixed-bit constraints; MOV CR0 is defined on x86_64.
    unsafe {
        core::arch::asm!(
            "mov cr0, {0}",
            in(reg) value,
            options(nostack, preserves_flags),
        );
    }
    true
}

#[cfg(not(target_arch = "x86_64"))]
fn write_cr0_impl(_value: u64) -> bool {
    false
}

#[cfg(target_arch = "x86_64")]
fn read_cr4_impl() -> Option<u64> {
    let value: u64;
    // SAFETY: MOV CR4 is defined on x86_64 hosts.
    unsafe {
        core::arch::asm!(
            "mov {0}, cr4",
            out(reg) value,
            options(nostack, preserves_flags),
        );
    }
    Some(value)
}

#[cfg(not(target_arch = "x86_64"))]
fn read_cr4_impl() -> Option<u64> {
    None
}

#[cfg(target_arch = "x86_64")]
fn write_cr4_impl(value: u64) -> bool {
    // SAFETY: caller validates fixed-bit constraints; MOV CR4 is defined on x86_64.
    unsafe {
        core::arch::asm!(
            "mov cr4, {0}",
            in(reg) value,
            options(nostack, preserves_flags),
        );
    }
    true
}

#[cfg(not(target_arch = "x86_64"))]
fn write_cr4_impl(_value: u64) -> bool {
    false
}
