//! Hypervisor entrypoint.

use core::arch::asm;

use hv_boot_abi::BootInfo;
use hv_core::boot::BootPhase;

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

    let mut phase = BootPhase::BootServicesExited;
    phase = advance(phase, BootPhase::MemoryReady);
    phase = advance(phase, BootPhase::CpuReady);
    phase = advance(phase, BootPhase::VmxReady);
    phase = advance(phase, BootPhase::IommuReady);
    phase = advance(phase, BootPhase::InterruptsReady);
    phase = advance(phase, BootPhase::PartitionsPrepared);
    let _ = advance(phase, BootPhase::Running);

    loop {
        // SAFETY: halts until the next interrupt; Gate B has no IDT yet.
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

fn advance(current: BootPhase, next: BootPhase) -> BootPhase {
    match current.transition(next) {
        Ok(value) => value,
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
