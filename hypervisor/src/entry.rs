//! Hypervisor entrypoint.

use core::arch::asm;

use hv_boot_abi::BootInfo;
use hv_core::boot::BootPhase;

use crate::plans;

/// Hypervisor entry called by the loader with a pointer to boot info.
///
/// # Safety
///
/// `boot_info` must point to a valid, aligned [`BootInfo`] structure.
pub unsafe extern "C" fn hypster_entry(boot_info: *const BootInfo) -> ! {
    if boot_info.is_null() {
        fail();
    }
    let info = unsafe { &*boot_info };
    if !info.header.is_compatible() {
        fail();
    }

    match plans::run_gate_c_init(info) {
        Ok(report) => {
            let _ = report.phase.transition(BootPhase::Running);
        }
        Err(_) => fail(),
    }

    loop {
        // SAFETY: halts until the next interrupt; Gate C has no IDT yet.
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

fn fail() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
