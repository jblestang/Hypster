//! Gate D partition preparation and IPC ring initialization.

use hv_config_model::hash::ConfigHash;
use hv_ipc::init_ring;
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

fn init_host_ipc_ring(channel: &IpcChannelPlan) -> Result<(), RuntimeError> {
    if channel.host_base.raw() == 0 {
        return Err(RuntimeError::TableRegionUnavailable);
    }
    let ptr = channel.host_base.raw() as *mut u8;
    let backing = unsafe { core::slice::from_raw_parts_mut(ptr, channel.shared_bytes as usize) };
    init_ring(backing, &channel.name, channel.slot_count, channel.slot_size)
        .map_err(RuntimeError::Ipc)?;
    Ok(())
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
