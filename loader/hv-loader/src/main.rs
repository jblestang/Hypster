//! UEFI loader entry for Hypster (Gate B skeleton).
//!
//! Gate B validates BootInfo construction and firmware table discovery.
//! Transfer to the hypervisor after `ExitBootServices` is completed in Gate C.

#![no_main]
#![no_std]

use core::panic::PanicInfo;

use uefi::allocator::Allocator;
use uefi::prelude::*;
use uefi::table::cfg::{ACPI2_GUID, ACPI_GUID};

use hv_boot_abi::{
    BootAcpiInfo, BootInfo, BootInfoHeader, BOOT_ABI_VERSION_MAJOR, BOOT_ABI_VERSION_MINOR,
    BOOT_INFO_MAGIC,
};

const HV_CONFIG_HASH_PLACEHOLDER: [u8; 32] = [0; 32];

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
    // SAFETY: Gate C must call `uefi::allocator::exit_boot_services` before ExitBootServices.
    unsafe {
        uefi::allocator::init(&mut system_table);
    }

    let rsdp_address = find_rsdp_address(&system_table);

    let boot_info = BootInfo {
        header: BootInfoHeader {
            magic: BOOT_INFO_MAGIC,
            version_major: BOOT_ABI_VERSION_MAJOR,
            version_minor: BOOT_ABI_VERSION_MINOR,
            total_size: core::mem::size_of::<BootInfo>() as u32,
            config_hash: HV_CONFIG_HASH_PLACEHOLDER,
        },
        acpi: BootAcpiInfo { rsdp_address },
        memory_map_entry_count: 0,
        flags: 0,
    };

    if boot_info.header.is_compatible() {
        Status::SUCCESS
    } else {
        Status::LOAD_ERROR
    }
}
