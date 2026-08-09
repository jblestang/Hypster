//! Versioned guest boot ABI.
//!
//! Guests learn their partition identity, memory layout, IPC regions, and
//! device assignments from a [`GuestBootInfo`] blob provided by the
//! hypervisor. Guests must not hardcode topology-specific constants.
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | Layout size/alignment | UNIT |
//! | Descriptor bounds checks | UNIT + REVIEW |

#![no_std]
#![warn(missing_docs)]

use hv_types::{GuestPhysAddr, VmId, VcpuId};

/// Major guest ABI version.
pub const GUEST_ABI_VERSION_MAJOR: u16 = 1;
/// Minor guest ABI version.
pub const GUEST_ABI_VERSION_MINOR: u16 = 0;

/// Magic signature for guest boot info.
pub const GUEST_BOOT_INFO_MAGIC: u64 = 0x4755_E572_5F42_494E;

/// Header for guest boot information.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestBootInfoHeader {
    /// Magic signature [`GUEST_BOOT_INFO_MAGIC`].
    pub magic: u64,
    /// Major ABI version.
    pub version_major: u16,
    /// Minor ABI version.
    pub version_minor: u16,
    /// Total structure size in bytes.
    pub total_size: u32,
    /// Assigned VM identifier.
    pub vm_id: u32,
    /// Current vCPU identifier.
    pub vcpu_id: u32,
}

impl GuestBootInfoHeader {
    /// Returns true when the header matches the compiled ABI major version.
    #[must_use]
    pub const fn is_compatible(&self) -> bool {
        self.magic == GUEST_BOOT_INFO_MAGIC
            && self.version_major == GUEST_ABI_VERSION_MAJOR
    }

    /// Returns the typed VM identifier.
    #[must_use]
    pub const fn vm_id(&self) -> VmId {
        VmId::new(self.vm_id)
    }

    /// Returns the typed vCPU identifier.
    #[must_use]
    pub const fn vcpu_id(&self) -> VcpuId {
        VcpuId::new(self.vcpu_id)
    }
}

/// Guest RAM region descriptor.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestMemoryRegion {
    /// Guest physical base address.
    pub base: u64,
    /// Region size in bytes.
    pub size: u64,
    /// Attribute flags defined by future ABI versions.
    pub flags: u32,
    /// Reserved.
    pub reserved: u32,
}

impl GuestMemoryRegion {
    /// Returns the typed guest physical base address.
    #[must_use]
    pub const fn base(&self) -> GuestPhysAddr {
        GuestPhysAddr::new(self.base)
    }
}

/// IPC region visible to a guest.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestIpcRegion {
    /// Channel name hash for stable identification.
    pub name_hash: u64,
    /// Producer VM ID.
    pub producer_vm_id: u32,
    /// Consumer VM ID.
    pub consumer_vm_id: u32,
    /// Shared memory guest physical base.
    pub base: u64,
    /// Shared memory size in bytes.
    pub size: u64,
    /// Role flags: bit0 producer, bit1 consumer.
    pub role_flags: u32,
    /// Reserved.
    pub reserved: u32,
}

/// MMIO region descriptor for an assigned device BAR.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestMmioRegion {
    /// Guest physical base address.
    pub base: u64,
    /// Region size in bytes.
    pub size: u64,
    /// Device identifier hash.
    pub device_hash: u64,
}

/// Fixed-layout guest boot info prefix.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuestBootInfo {
    /// Header and identity.
    pub header: GuestBootInfoHeader,
    /// Number of memory regions following the prefix.
    pub memory_region_count: u32,
    /// Number of IPC regions following the memory table.
    pub ipc_region_count: u32,
    /// Number of MMIO regions following the IPC table.
    pub mmio_region_count: u32,
    /// Reserved.
    pub reserved: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_boot_info_header_layout() {
        assert_eq!(core::mem::size_of::<GuestBootInfoHeader>(), 24);
        assert_eq!(core::mem::align_of::<GuestBootInfoHeader>(), 8);
    }

    #[test]
    fn guest_header_reports_compatible_version() {
        let header = GuestBootInfoHeader {
            magic: GUEST_BOOT_INFO_MAGIC,
            version_major: GUEST_ABI_VERSION_MAJOR,
            version_minor: GUEST_ABI_VERSION_MINOR,
            total_size: core::mem::size_of::<GuestBootInfo>() as u32,
            vm_id: 1,
            vcpu_id: 0,
        };
        assert!(header.is_compatible());
        assert_eq!(header.vm_id().raw(), 1);
    }
}
