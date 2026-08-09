//! Hypster hypervisor binary entry.

#![no_std]
#![no_main]
#![allow(unsafe_code)]

extern crate alloc;

mod bump_alloc;

use hypster::hypster_entry;

#[global_allocator]
static ALLOCATOR: bump_alloc::BumpAllocator = bump_alloc::BumpAllocator::new();

/// Hypervisor entry called by the loader with a pointer to boot info.
///
/// # Safety
///
/// `boot_info` must point to a valid, aligned [`BootInfo`] structure.
#[no_mangle]
pub unsafe extern "C" fn _start(boot_info: *const hv_boot_abi::BootInfo) -> ! {
    hypster_entry(boot_info)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
