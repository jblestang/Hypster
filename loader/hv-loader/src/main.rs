//! UEFI loader entry for Hypster (Gate C boot handoff).

#![no_main]
#![no_std]

extern crate alloc;

mod boot_info;
mod handoff;

use core::panic::PanicInfo;

use uefi::allocator::Allocator;
use uefi::prelude::*;
use uefi::table::boot::MemoryType;
use uefi::table::cfg::{ACPI2_GUID, ACPI_GUID};

use boot_info::{build_boot_info_blob, load_hypervisor_image};
use handoff::jump_to_hypervisor;

#[global_allocator]
static GLOBAL: Allocator = Allocator;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

fn find_rsdp_address(system_table: &SystemTable<Boot>) -> u64 {
    system_table
        .config_table()
        .iter()
        .find_map(|entry| {
            if entry.guid == ACPI2_GUID || entry.guid == ACPI_GUID {
                Some(entry.address as u64)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

#[entry]
fn main(_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // SAFETY: required before ExitBootServices per uefi crate contract.
    unsafe {
        uefi::allocator::init(&mut system_table);
    }

    let rsdp_address = find_rsdp_address(&system_table);
    let boot_services = system_table.boot_services();

    let hypervisor = load_hypervisor_image(boot_services);

    let mmap_size = boot_services.memory_map_size();
    let mut mmap_buf = alloc::vec![0u8; mmap_size.map_size + mmap_size.entry_size * 8];
    let memory_map = match boot_services.memory_map(&mut mmap_buf) {
        Ok(map) => map,
        Err(_) => return Status::OUT_OF_RESOURCES,
    };

    let blob = match build_boot_info_blob(
        boot_services,
        &memory_map,
        rsdp_address,
        hypervisor,
    ) {
        Ok(blob) => blob,
        Err(status) => return status,
    };

    let boot_info_ptr = blob.phys_addr as *const hv_boot_abi::BootInfo;

    let (_runtime, final_map) = system_table.exit_boot_services(MemoryType::LOADER_DATA);

    let _ = final_map;

    jump_to_hypervisor(hypervisor.entry, boot_info_ptr);
}
