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

/// Executes VMWRITE for one VMCS field.
///
/// # Safety
///
/// Caller must ensure a VMCS is loaded and the field encoding is valid.
pub unsafe fn vmwrite(field: u32, value: u64) -> Result<(), VmxError> {
    vmwrite_impl(field, value)
}

/// Executes VMREAD for one VMCS field.
///
/// # Safety
///
/// Caller must ensure a VMCS is loaded and the field encoding is valid.
pub unsafe fn vmread(field: u32) -> Result<u64, VmxError> {
    vmread_impl(field)
}

/// Executes VMLAUNCH.
///
/// # Safety
///
/// Caller must ensure guest VMCS state is valid and host state is configured.
pub unsafe fn vmlaunch() -> Result<(), VmxError> {
    vmlaunch_impl()
}

/// Executes VMRESUME.
///
/// # Safety
///
/// Caller must ensure guest VMCS state is valid and host state is configured.
pub unsafe fn vmresume() -> Result<(), VmxError> {
    vmresume_impl()
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

#[cfg(target_arch = "x86_64")]
unsafe fn vmwrite_impl(field: u32, value: u64) -> Result<(), VmxError> {
    let failed: u8;
    // SAFETY: caller guarantees VMCS is loaded and field is valid.
    unsafe {
        core::arch::asm!(
            "vmwrite {value}, {field}",
            "setc {failed}",
            field = in(reg) field as u64,
            value = in(reg) value,
            failed = lateout(reg_byte) failed,
            options(nostack),
        );
    }
    if failed != 0 {
        return Err(VmxError::VmwriteFailed);
    }
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn vmwrite_impl(_field: u32, _value: u64) -> Result<(), VmxError> {
    Err(VmxError::UnsupportedArch)
}

#[cfg(target_arch = "x86_64")]
unsafe fn vmread_impl(field: u32) -> Result<u64, VmxError> {
    let failed: u8;
    let mut value: u64;
    // SAFETY: caller guarantees VMCS is loaded and field is valid.
    unsafe {
        core::arch::asm!(
            "vmread [{value_out}], {field}",
            "setc {failed}",
            field = in(reg) field as u64,
            value_out = lateout(reg) value,
            failed = lateout(reg_byte) failed,
            options(nostack),
        );
    }
    if failed != 0 {
        return Err(VmxError::VmreadFailed);
    }
    Ok(value)
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn vmread_impl(_field: u32) -> Result<u64, VmxError> {
    Err(VmxError::UnsupportedArch)
}

#[cfg(target_arch = "x86_64")]
unsafe fn vmlaunch_impl() -> Result<(), VmxError> {
    let failed: u8;
    // SAFETY: caller guarantees guest VMCS is valid.
    unsafe {
        core::arch::asm!(
            "vmlaunch",
            "setc {failed}",
            failed = lateout(reg_byte) failed,
            options(nostack),
        );
    }
    if failed != 0 {
        return Err(VmxError::VmlaunchFailed);
    }
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn vmlaunch_impl() -> Result<(), VmxError> {
    Err(VmxError::UnsupportedArch)
}

#[cfg(target_arch = "x86_64")]
unsafe fn vmresume_impl() -> Result<(), VmxError> {
    let failed: u8;
    // SAFETY: caller guarantees guest VMCS is valid.
    unsafe {
        core::arch::asm!(
            "vmresume",
            "setc {failed}",
            failed = lateout(reg_byte) failed,
            options(nostack),
        );
    }
    if failed != 0 {
        return Err(VmxError::VmresumeFailed);
    }
    Ok(())
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn vmresume_impl() -> Result<(), VmxError> {
    Err(VmxError::UnsupportedArch)
}
