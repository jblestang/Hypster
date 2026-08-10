//! Guest MMIO layout constants shared with partition boot info.

/// Guest physical base for the first device MMIO BAR.
pub const MMIO_GUEST_BASE: u64 = 0xFEB0_0000;
/// Stride between consecutive device BAR guest addresses.
pub const MMIO_GUEST_STRIDE: u64 = 0x10_0000;
/// Size of each emulated device BAR.
pub const MMIO_REGION_SIZE: u64 = 128 * 1024;

/// Host physical base for emulated device MMIO backing.
pub const MMIO_HOST_BASE: u64 = 0x1_1720_0000;
/// Host stride between partition device MMIO backing regions.
pub const MMIO_HOST_STRIDE: u64 = 0x10_0000;

/// Returns guest physical base for a device index within a partition.
#[must_use]
pub fn mmio_guest_phys(device_index: usize) -> u64 {
    MMIO_GUEST_BASE + (device_index as u64) * MMIO_GUEST_STRIDE
}

/// Returns host physical base for a partition/device MMIO backing page.
#[must_use]
pub fn mmio_host_phys(vm_id: u32, device_index: usize) -> u64 {
    MMIO_HOST_BASE + (vm_id as u64) * MMIO_HOST_STRIDE + (device_index as u64) * MMIO_HOST_STRIDE
}

/// Gate D MVP VMCS region base (4 KiB per partition).
pub const VMCS_REGION_BASE: u64 = 0x1_1800_0000;
/// Size of one VMCS region.
pub const VMCS_REGION_SIZE: u64 = 4096;

/// Returns host physical base for a partition VMCS region.
#[must_use]
pub fn vmcs_region_hpa(vm_id: u32) -> u64 {
    VMCS_REGION_BASE + (vm_id as u64) * VMCS_REGION_SIZE
}
