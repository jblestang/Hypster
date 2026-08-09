//! Hypervisor entrypoint.

use core::arch::asm;

use hv_boot_abi::BootInfo;
use hv_core::boot::BootPhase;

use crate::datapath;
use crate::plans;
use crate::serial;
#[cfg(feature = "hardware")]
use crate::launch;

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
    serial::write_str("hypster: entry\n");
    if !info.header.is_compatible() {
        serial::write_str("hypster: boot info invalid\n");
        fail();
    }

    match plans::run_gate_d_init(info) {
        Ok((plans, report)) => {
            let _ = report.gate_c.phase.transition(BootPhase::Running);
            serial::write_str("hypster: gate-d running\n");
            #[cfg(feature = "hardware")]
            if report.gate_c.vmx_enabled {
                // SAFETY: Gate C init enabled VMX and installed EPT/VMCS regions.
                if unsafe { launch::try_launch_in_guest(&plans) }.is_err() {
                    serial::write_str("hypster: vmlaunch skipped\n");
                }
            }
            datapath::run_steady_state_loop(&plans, report);
        }
        Err(err) => {
            serial::write_str("hypster: init fail: ");
            serial::write_str(err.as_str());
            serial::write_str("\n");
            fail();
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
