//! UEFI memory map helpers.

use hv_types::HostPhysAddr;

/// Subset of UEFI memory types used by the planner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum UefiMemoryType {
    /// Conventional free RAM.
    Conventional = 7,
    /// Loader/hypervisor reserved region marker.
    Reserved = 1,
}

impl UefiMemoryType {
    /// Converts a raw UEFI type value.
    #[must_use]
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            7 => Some(Self::Conventional),
            1 => Some(Self::Reserved),
            _ => None,
        }
    }
}

/// Conventional memory region from firmware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConventionalRegion {
    /// Region base address.
    pub base: HostPhysAddr,
    /// Region size in bytes.
    pub size: u64,
}

impl ConventionalRegion {
    /// Returns the exclusive end address if representable.
    pub const fn end(&self) -> Option<HostPhysAddr> {
        match self.base.raw().checked_add(self.size) {
            Some(v) => Some(HostPhysAddr::new(v)),
            None => None,
        }
    }
}

/// Collects conventional memory regions from raw firmware descriptors.
pub fn collect_conventional(
    descriptors: &[(u32, HostPhysAddr, u64)],
) -> alloc::vec::Vec<ConventionalRegion> {
    let mut regions = alloc::vec::Vec::new();
    for (typ, base, size) in descriptors {
        if UefiMemoryType::from_raw(*typ) == Some(UefiMemoryType::Conventional) && *size > 0 {
            regions.push(ConventionalRegion {
                base: *base,
                size: *size,
            });
        }
    }
    regions
}

/// Returns total conventional memory bytes.
pub fn total_conventional_bytes(regions: &[ConventionalRegion]) -> Option<u64> {
    regions
        .iter()
        .try_fold(0u64, |acc, region| acc.checked_add(region.size))
}
