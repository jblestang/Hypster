//! Hypervisor entrypoint.

use core::arch::asm;

use hv_boot_abi::BootInfo;
use hv_core::boot::BootPhase;

use crate::datapath;
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

    match plans::run_gate_d_init(info) {
        Ok((plans, report)) => {
            let _ = report.gate_c.phase.transition(BootPhase::Running);
            datapath::run_steady_state_loop(&plans, report);
        }
        Err(_) => fail(),
    }
}

fn fail() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
