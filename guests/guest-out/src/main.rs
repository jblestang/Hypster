#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use guest_common::is_boot_info_compatible;
use hv_guest_abi::GuestBootInfo;

static BOOT_INFO: GuestBootInfo = GuestBootInfo {
    header: hv_guest_abi::GuestBootInfoHeader {
        magic: hv_guest_abi::GUEST_BOOT_INFO_MAGIC,
        version_major: hv_guest_abi::GUEST_ABI_VERSION_MAJOR,
        version_minor: hv_guest_abi::GUEST_ABI_VERSION_MINOR,
        total_size: core::mem::size_of::<GuestBootInfo>() as u32,
        vm_id: 2,
        vcpu_id: 0,
    },
    memory_region_count: 0,
    ipc_region_count: 0,
    mmio_region_count: 0,
    reserved: 0,
};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    if !is_boot_info_compatible(&BOOT_INFO) {
        fail();
    }
    idle();
}

fn idle() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

fn fail() -> ! {
    idle()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    fail()
}
