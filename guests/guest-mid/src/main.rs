#![no_std]
#![no_main]

use core::panic::PanicInfo;

use guest_common::{boot_info_ptr, is_boot_info_compatible, relay_once_then_halt};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // SAFETY: hypervisor writes boot info at GPA 0x9000 before VMLAUNCH.
    let info = unsafe { &*boot_info_ptr() };
    if !is_boot_info_compatible(info) {
        fail();
    }
    // SAFETY: hypervisor maps initialized IPC rings at boot-info GPAs for MID.
    unsafe {
        relay_once_then_halt();
    }
}

fn fail() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    fail()
}
