//! Gate D partition preparation and IPC ring initialization.

use hv_config_model::hash::ConfigHash;
use hv_ipc::{init_ring, IpcError};
use hv_types::HostPhysAddr;

use crate::error::RuntimeError;
use crate::init::GateCPlans;

/// One IPC channel with host backing for ring initialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpcChannelPlan {
    /// Channel name.
    pub name: alloc::string::String,
    /// Ring slot count.
    pub slot_count: u32,
    /// Ring slot size in bytes.
    pub slot_size: u32,
    /// Host physical base of shared memory.
    pub host_base: HostPhysAddr,
    /// Total shared bytes for the channel.
    pub shared_bytes: u64,
}

/// Gate D plans extending Gate C table install data.
#[derive(Clone, Debug)]
pub struct GateDPlans {
    /// Gate C EPT/VT-d/VMX plans and table bases.
    pub gate_c: GateCPlans,
    /// IPC channels to initialize before guest launch.
    pub ipc_channels: alloc::vec::Vec<IpcChannelPlan>,
    /// Expected configuration hash from validated YAML.
    pub config_hash: ConfigHash,
    /// When true, boot info hash must match [`Self::config_hash`].
    pub require_config_hash: bool,
}

/// Summary of partition preparation work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionPrepReport {
    /// Number of guest partitions described by the platform intent.
    pub partitions: usize,
    /// Number of IPC rings initialized.
    pub ipc_rings: usize,
}

/// Verifies boot info config hash when enforcement is required.
pub fn verify_config_hash(
    expected: ConfigHash,
    boot_hash: [u8; 32],
    require: bool,
) -> Result<(), RuntimeError> {
    if !require {
        return Ok(());
    }
    if boot_hash != expected.bytes() {
        return Err(RuntimeError::ConfigHashMismatch);
    }
    Ok(())
}

/// Assigns host backing slices for IPC channels in declaration order.
///
/// # Errors
///
/// Returns [`RuntimeError::Ipc`] when the backing buffer is too small.
pub fn assign_ipc_host_backing(
    plans: &mut GateDPlans,
    backing: &mut [u8],
) -> Result<(), RuntimeError> {
    let mut offset = 0usize;
    for channel in &mut plans.ipc_channels {
        let size = ring_backing_bytes(channel)?;
        let end = offset.checked_add(size).ok_or(IpcError::Overflow)?;
        if end > backing.len() {
            return Err(RuntimeError::Ipc(IpcError::BufferTooSmall));
        }
        channel.host_base = HostPhysAddr::new(backing[offset..].as_ptr() as u64);
        channel.shared_bytes = size as u64;
        offset = end;
    }
    Ok(())
}

fn ring_backing_bytes(channel: &IpcChannelPlan) -> Result<usize, RuntimeError> {
    hv_ipc::compute_shared_bytes(channel.slot_count, channel.slot_size)
        .map(|bytes| bytes as usize)
        .map_err(RuntimeError::Ipc)
}

fn init_host_ipc_ring(channel: &IpcChannelPlan) -> Result<(), RuntimeError> {
    if channel.host_base.raw() == 0 {
        return Err(RuntimeError::TableRegionUnavailable);
    }
    let size = ring_backing_bytes(channel)?;
    let ptr = channel.host_base.raw() as *mut u8;
    let backing = unsafe { core::slice::from_raw_parts_mut(ptr, size) };
    init_ring(backing, &channel.name, channel.slot_count, channel.slot_size)
        .map_err(RuntimeError::Ipc)?;
    Ok(())
}

/// Initializes IPC rings in host backing memory.
///
/// # Errors
///
/// Returns [`RuntimeError::Ipc`] when ring initialization fails.
pub fn prepare_ipc_rings(
    plans: &GateDPlans,
    partition_count: usize,
) -> Result<PartitionPrepReport, RuntimeError> {
    let mut ipc_rings = 0usize;
    for channel in &plans.ipc_channels {
        init_host_ipc_ring(channel)?;
        ipc_rings += 1;
    }
    Ok(PartitionPrepReport { partitions: partition_count, ipc_rings })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_hash_mismatch_is_rejected_when_required() {
        let expected = ConfigHash([1; 32]);
        let boot = [2; 32];
        assert!(matches!(
            verify_config_hash(expected, boot, true),
            Err(RuntimeError::ConfigHashMismatch)
        ));
    }
}
