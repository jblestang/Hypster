//! Control register access helpers.

use crate::error::X86Error;

/// Reads CR0.
///
/// # Safety
///
/// Reading CR0 is privileged and must only be used when the current execution
/// environment permits it.
pub unsafe fn read_cr0() -> Result<u64, X86Error> {
    #[cfg(target_arch = "x86_64")]
    {
        let value: u64;
        core::arch::asm!(
            "mov {}, cr0",
            out(reg) value,
            options(nostack, preserves_flags)
        );
        Ok(value)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Err(X86Error::UnsupportedArchitecture)
    }
}

/// Reads CR4.
///
/// # Safety
///
/// Reading CR4 is privileged and must only be used when the current execution
/// environment permits it.
pub unsafe fn read_cr4() -> Result<u64, X86Error> {
    #[cfg(target_arch = "x86_64")]
    {
        let value: u64;
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) value,
            options(nostack, preserves_flags)
        );
        Ok(value)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        Err(X86Error::UnsupportedArchitecture)
    }
}

/// Writes CR4.
///
/// # Safety
///
/// Writing CR4 is privileged and can change CPU behavior immediately. The
/// caller must preserve required fixed bits and avoid invalid combinations.
///
/// # Errors
///
/// Returns [`X86Error::UnsupportedArchitecture`] when not compiled for x86_64.
pub unsafe fn write_cr4(value: u64) -> Result<(), X86Error> {
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::asm!(
            "mov cr4, {}",
            in(reg) value,
            options(nostack, preserves_flags)
        );
        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = value;
        Err(X86Error::UnsupportedArchitecture)
    }
}
