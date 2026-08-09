//! Boot info blob construction in UEFI allocated memory.

use core::mem::{align_of, size_of};
use core::ptr;

use uefi::table::boot::{AllocateType, MemoryType};
use uefi::table::boot::{BootServices, MemoryMap};

use hv_boot_abi::{
    BootAcpiInfo, BootInfo, BootInfoHeader, BootMemoryDescriptor, BOOT_ABI_VERSION_MAJOR,
    BOOT_ABI_VERSION_MINOR, BOOT_INFO_MAGIC,
};

/// Placeholder config hash until Gate D wires validated configuration.
pub const HV_CONFIG_HASH_PLACEHOLDER: [u8; 32] = [0; 32];

/// Hypervisor load result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HypervisorImage {
    /// Entry point physical address.
    pub entry: u64,
    /// Physical base where the image was loaded.
    pub load_base: u64,
}

impl HypervisorImage {
    /// Placeholder entry used when `\EFI\hypster\hypster.bin` is unavailable.
    pub const PLACEHOLDER_ENTRY: u64 = 0x100000;
}

/// Allocated boot info buffer and associated metadata.
pub struct BootInfoBlob {
    /// Physical address of the boot info allocation.
    pub phys_addr: u64,
    /// Total byte size of the allocation.
    pub total_bytes: usize,
    /// Entry count copied from the memory map.
    pub entry_count: u32,
}

impl BootInfoBlob {
    /// Returns a pointer to the fixed [`BootInfo`] prefix.
    ///
    /// # Safety
    ///
    /// Must only be called while the allocation remains valid.
    pub unsafe fn boot_info_ptr(&self) -> *mut BootInfo {
        self.phys_addr as *mut BootInfo
    }
}

/// Builds a boot info blob from the UEFI memory map and ACPI RSDP pointer.
///
/// # Errors
///
/// Returns [`uefi::Status`] when allocation fails.
pub fn build_boot_info_blob(
    boot_services: &BootServices,
    memory_map: &MemoryMap,
    rsdp_address: u64,
    hypervisor: HypervisorImage,
) -> Result<BootInfoBlob, uefi::Status> {
    let entry_count = memory_map.entries().count() as u32;
    let desc_bytes = entry_count as usize * size_of::<BootMemoryDescriptor>();
    let prefix = size_of::<BootInfo>();
    let total_bytes = prefix
        .checked_add(desc_bytes)
        .ok_or(uefi::Status::OUT_OF_RESOURCES)?;

    let page_count = ((total_bytes + 4095) / 4096) as usize;
    let phys = boot_services.allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        page_count,
    )?;

    // SAFETY: UEFI returned freshly allocated loader data pages.
    unsafe {
        ptr::write_bytes(phys as *mut u8, 0, page_count * 4096);
    }

    let total_size = total_bytes as u32;
    let info = BootInfo {
        header: BootInfoHeader {
            magic: BOOT_INFO_MAGIC,
            version_major: BOOT_ABI_VERSION_MAJOR,
            version_minor: BOOT_ABI_VERSION_MINOR,
            total_size,
            config_hash: HV_CONFIG_HASH_PLACEHOLDER,
        },
        acpi: BootAcpiInfo { rsdp_address },
        memory_map_entry_count: entry_count,
        flags: 0,
        hypervisor_load_address: hypervisor.load_base,
        boot_info_bytes: total_size,
    };

    // SAFETY: `phys` points to `total_bytes` writable memory.
    unsafe {
        ptr::write(phys as *mut BootInfo, info);
    }

    copy_memory_map(memory_map, phys, prefix)?;

    Ok(BootInfoBlob {
        phys_addr: phys,
        total_bytes,
        entry_count,
    })
}

fn copy_memory_map(
    memory_map: &MemoryMap,
    phys: u64,
    prefix: usize,
) -> Result<(), uefi::Status> {
    if prefix % align_of::<BootMemoryDescriptor>() != 0 {
        return Err(uefi::Status::INVALID_PARAMETER);
    }
    let dst = (phys as usize + prefix) as *mut BootMemoryDescriptor;
    for (idx, desc) in memory_map.entries().enumerate() {
        let out = BootMemoryDescriptor {
            typ: desc.ty.0,
            physical_start: desc.phys_start,
            number_of_bytes: desc.page_count * 4096,
            attribute: desc.att.bits(),
        };
        // SAFETY: destination slice sized for all entries.
        unsafe {
            ptr::write(dst.add(idx), out);
        }
    }
    Ok(())
}

/// Loads the hypervisor flat binary from the boot volume when present.
pub fn load_hypervisor_image(boot_services: &BootServices) -> HypervisorImage {
    match try_load_hypster_bin(boot_services) {
        Ok(image) => image,
        Err(_) => HypervisorImage {
            entry: HypervisorImage::PLACEHOLDER_ENTRY,
            load_base: HypervisorImage::PLACEHOLDER_ENTRY,
        },
    }
}

fn try_load_hypster_bin(boot_services: &BootServices) -> Result<HypervisorImage, uefi::Status> {
    use uefi::cstr16;
    use uefi::fs::FileSystem;

    let fs = boot_services
        .get_image_file_system(boot_services.image_handle())
        .map_err(|err| err.status())?;
    let mut file_system = FileSystem::new(fs);
    let data = file_system
        .read(cstr16!("\\EFI\\hypster\\hypster.bin").as_ref())
        .map_err(|_| uefi::Status::NOT_FOUND)?;

    if data.is_empty() {
        return Err(uefi::Status::LOAD_ERROR);
    }

    let page_count = ((data.len() + 4095) / 4096) as usize;
    let load_base = boot_services.allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        page_count,
    )?;

    // SAFETY: writing into UEFI allocated pages.
    unsafe {
        ptr::copy_nonoverlapping(data.as_ptr(), load_base as *mut u8, data.len());
    }

    Ok(HypervisorImage {
        entry: load_base,
        load_base,
    })
}
