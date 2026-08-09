//! VMX instruction wrappers.

use crate::error::VmxError;

/// Executes VMXON with the given region physical address.
///
/// # Safety
///
/// Caller must ensure VMX is enabled, the region is valid, and the address is 4 KiB aligned.
pub unsafe fn vmxon(region_hpa: u64) -> Result<(), VmxError> {
    vmxon_impl(region_hpa)
}

/// Executes VMCLEAR with the given VMCS region physical address.
///
/// # Safety
///
/// Caller must ensure the region is valid and the address is 4 KiB aligned.
pub unsafe fn vmclear(vmcs_hpa: u64) -> Result<(), VmxError> {
    vmclear_impl(vmcs_hpa)
}

/// Executes VMPTRLD with the given VMCS region physical address.
///
/// # Safety
///
/// Caller must ensure the region is valid and the address is 4 KiB aligned.
pub unsafe fn vmptrld(vmcs_hpa: u64) -> Result<(), VmxError> {
    vmptrld_impl(vmcs_hpa)
}

#[cfg(target_arch = "x86_64")]
unsafe fn vmxon_impl(region_hpa: u64) -> Result<(), VmxError> {
    let failed: u8;
    // SAFETY: caller guarantees VMX prerequisites and region validity.
    unsafe {
        core::arch::asm!(
            "vmxon [{region}]",
            "setc {failed}",
            region = in(reg) region_hpa,
            failed = lateout(reg_byte) failed,
            options(nostack),
        );
    }
    if failed != 0 {
        return Err(VmxError::VmxonFailed);
    }
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn vmxon_impl(_region_hpa: u64) -> Result<(), VmxError> {
    Err(VmxError::UnsupportedArch)
}

#[cfg(target_arch = "x86_64")]
unsafe fn vmclear_impl(vmcs_hpa: u64) -> Result<(), VmxError> {
    let failed: u8;
    // SAFETY: caller guarantees VMCS region validity.
    unsafe {
        core::arch::asm!(
            "vmclear [{region}]",
            "setc {failed}",
            region = in(reg) vmcs_hpa,
            failed = lateout(reg_byte) failed,
            options(nostack),
        );
    }
    if failed != 0 {
        return Err(VmxError::VmclearFailed);
    }
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn vmclear_impl(_vmcs_hpa: u64) -> Result<(), VmxError> {
    Err(VmxError::UnsupportedArch)
}

#[cfg(target_arch = "x86_64")]
unsafe fn vmptrld_impl(vmcs_hpa: u64) -> Result<(), VmxError> {
    let failed: u8;
    // SAFETY: caller guarantees VMCS region validity.
    unsafe {
        core::arch::asm!(
            "vmptrld [{region}]",
            "setc {failed}",
            region = in(reg) vmcs_hpa,
            failed = lateout(reg_byte) failed,
            options(nostack),
        );
    }
    if failed != 0 {
        return Err(VmxError::VmptrldFailed);
    }
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn vmptrld_impl(_vmcs_hpa: u64) -> Result<(), VmxError> {
    Err(VmxError::UnsupportedArch)
}
