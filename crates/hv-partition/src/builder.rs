//! Guest boot info builder from resolved platform plans.

use alloc::vec;
use alloc::vec::Vec;

use hv_config_model::intent::{PartitionIntent, StaticIntentIR};
use hv_core::platform_ir::StaticPlatformIR;
use hv_guest_abi::layout::{compute_guest_boot_info_bytes, GUEST_BOOT_INFO_FIXED_PREFIX_BYTES};
use hv_guest_abi::{
    GuestBootInfo, GuestBootInfoHeader, GuestIpcRegion, GuestMemoryRegion, GuestMmioRegion,
    GUEST_ABI_VERSION_MAJOR, GUEST_ABI_VERSION_MINOR, GUEST_BOOT_INFO_MAGIC,
};
use hv_ipc::{ipc_name_hash, ROLE_CONSUMER, ROLE_PRODUCER};
use hv_memory::plan::MemoryPurpose;
use hv_types::VcpuId;
use hv_types::VmId;

use crate::ipc_guest_phys::{ipc_guest_phys_base, visible_ipc_channels};
use crate::PartitionError;

/// Serialized guest boot info blob and metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestBootInfoBlob {
    /// Raw bytes written for the guest.
    pub bytes: Vec<u8>,
    /// VM identifier for the partition.
    pub vm_id: VmId,
    /// vCPU identifier encoded in the header.
    pub vcpu_id: VcpuId,
}

/// Builds guest boot info for one partition from a resolved platform plan.
///
/// # Errors
///
/// Returns [`PartitionError`] when the partition is unknown or layout fails.
pub fn build_guest_boot_info(
    platform: &StaticPlatformIR,
    vm_id: VmId,
    vcpu_id: VcpuId,
) -> Result<GuestBootInfoBlob, PartitionError> {
    let partition = platform
        .intent
        .partitions
        .iter()
        .find(|part| part.vm_id == vm_id)
        .ok_or(PartitionError::UnknownPartition { vm_id: vm_id.raw() })?;

    let memory_regions = build_memory_regions(platform, partition)?;
    let ipc_regions = build_ipc_regions(&platform.intent, vm_id)?;
    let mmio_regions = build_mmio_regions(partition);

    let total = compute_guest_boot_info_bytes(
        memory_regions.len() as u32,
        ipc_regions.len() as u32,
        mmio_regions.len() as u32,
    )?;

    let mut bytes = vec![0u8; total];
    let prefix = GuestBootInfo {
        header: GuestBootInfoHeader {
            magic: GUEST_BOOT_INFO_MAGIC,
            version_major: GUEST_ABI_VERSION_MAJOR,
            version_minor: GUEST_ABI_VERSION_MINOR,
            total_size: total as u32,
            vm_id: vm_id.raw(),
            vcpu_id: vcpu_id.raw(),
        },
        memory_region_count: memory_regions.len() as u32,
        ipc_region_count: ipc_regions.len() as u32,
        mmio_region_count: mmio_regions.len() as u32,
        reserved: 0,
    };

    write_struct(&mut bytes, 0, &prefix)?;
    let mut offset = GUEST_BOOT_INFO_FIXED_PREFIX_BYTES;
    offset = write_table(&mut bytes, offset, &memory_regions)?;
    offset = write_table(&mut bytes, offset, &ipc_regions)?;
    write_table(&mut bytes, offset, &mmio_regions)?;

    Ok(GuestBootInfoBlob { bytes, vm_id, vcpu_id })
}

fn build_memory_regions(
    platform: &StaticPlatformIR,
    partition: &PartitionIntent,
) -> Result<Vec<GuestMemoryRegion>, PartitionError> {
    let backing = platform.memory.regions.iter().find(|region| {
        matches!(
            region.purpose,
            MemoryPurpose::PartitionGuestRam {
                vm_id
            } if vm_id == partition.vm_id.raw()
        )
    });
    let Some(backing) = backing else {
        return Err(PartitionError::UnknownPartition { vm_id: partition.vm_id.raw() });
    };
    Ok(vec![GuestMemoryRegion { base: 0, size: backing.size, flags: 0, reserved: 0 }])
}

fn build_ipc_regions(
    intent: &StaticIntentIR,
    vm_id: VmId,
) -> Result<Vec<GuestIpcRegion>, PartitionError> {
    let mut out = Vec::new();
    for channel in visible_ipc_channels(intent, vm_id) {
        let base =
            ipc_guest_phys_base(intent, vm_id, &channel.name).ok_or(PartitionError::Overflow)?;
        let mut role_flags = 0;
        if channel.producer == vm_id {
            role_flags |= ROLE_PRODUCER;
        }
        if channel.consumer == vm_id {
            role_flags |= ROLE_CONSUMER;
        }
        out.push(GuestIpcRegion {
            name_hash: ipc_name_hash(&channel.name),
            producer_vm_id: channel.producer.raw(),
            consumer_vm_id: channel.consumer.raw(),
            base,
            size: channel.shared_bytes,
            role_flags,
            reserved: 0,
        });
    }
    Ok(out)
}

fn build_mmio_regions(partition: &PartitionIntent) -> Vec<GuestMmioRegion> {
    let mut out = Vec::new();
    for (idx, device) in partition.devices.iter().enumerate() {
        let device_hash = ipc_name_hash(&device.bdf);
        out.push(GuestMmioRegion {
            base: crate::layout::mmio_guest_phys(idx),
            size: crate::layout::MMIO_REGION_SIZE,
            device_hash,
        });
    }
    out
}

fn write_struct<T: Copy>(bytes: &mut [u8], offset: usize, value: &T) -> Result<(), PartitionError> {
    let size = core::mem::size_of::<T>();
    let end = offset.checked_add(size).ok_or(PartitionError::Overflow)?;
    if end > bytes.len() {
        return Err(PartitionError::BufferTooSmall { required: end, provided: bytes.len() });
    }
    // SAFETY: T is Copy and destination has sufficient size/alignment at prefix offset.
    unsafe {
        core::ptr::write_unaligned(bytes.as_mut_ptr().add(offset) as *mut T, *value);
    }
    Ok(())
}

fn write_table<T: Copy>(
    bytes: &mut [u8],
    offset: usize,
    table: &[T],
) -> Result<usize, PartitionError> {
    let entry_size = core::mem::size_of::<T>();
    let total = table.len().checked_mul(entry_size).ok_or(PartitionError::Overflow)?;
    let end = offset.checked_add(total).ok_or(PartitionError::Overflow)?;
    if end > bytes.len() {
        return Err(PartitionError::BufferTooSmall { required: end, provided: bytes.len() });
    }
    for (idx, entry) in table.iter().enumerate() {
        let entry_offset = offset + idx * entry_size;
        write_struct(bytes, entry_offset, entry)?;
    }
    Ok(end)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use hv_config_model::pipeline::compile_config;
    use hv_config_model::yaml::read_yaml_file;
    use hv_core::fixture::qemu_validation_observed;
    use hv_core::resolve_platform;
    use hv_guest_abi::layout;
    use hv_types::VmId;

    use super::*;

    #[test]
    fn mid_partition_boot_info_layout_validates() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
        let raw = read_yaml_file(path).expect("read config");
        let compiled = compile_config(raw).expect("compile config");
        let observed = qemu_validation_observed().expect("fixture");
        let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
        let blob = build_guest_boot_info(&platform, VmId::new(1), VcpuId::new(0)).expect("build");
        let info = unsafe { &*(blob.bytes.as_ptr() as *const GuestBootInfo) };
        let result = unsafe { layout::validate_guest_boot_info_layout(info, blob.bytes.len()) };
        assert!(result.is_ok());
        let ipc = unsafe { layout::ipc_regions_slice(info) };
        assert_eq!(ipc.len(), 2);
    }
}
