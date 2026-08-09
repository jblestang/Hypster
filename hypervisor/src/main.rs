//! Hypster hypervisor binary entry.

#![no_std]
#![no_main]

use hypster::hypster_entry;

#[no_mangle]
pub extern "C" fn _start(boot_info: *const hv_boot_abi::BootInfo) -> ! {
    // SAFETY: loader passes a valid BootInfo pointer per boot ABI contract.
    unsafe { hypster_entry(boot_info) }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
