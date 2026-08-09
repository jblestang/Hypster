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
/// Also rewrites matching EPT mapping host physical addresses so guest IPC
/// translations target the same backing installed for the hypervisor datapath.
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
        let planned_hpa = channel.host_base;
        let new_hpa = HostPhysAddr::new(backing[offset..].as_ptr() as u64);
        channel.host_base = new_hpa;
        channel.shared_bytes = size as u64;
        if planned_hpa != new_hpa {
            patch_ept_ipc_host_phys(&mut plans.gate_c.ept, planned_hpa, new_hpa);
        }
        offset = end;
    }
    Ok(())
}

fn patch_ept_ipc_host_phys(
    ept: &mut hv_ept::EptPlan,
    planned_hpa: HostPhysAddr,
    new_hpa: HostPhysAddr,
) {
    for partition in &mut ept.partitions {
        for mapping in &mut partition.mappings {
            if mapping.host_phys == planned_hpa {
                mapping.host_phys = new_hpa;
            }
        }
    }
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
    #![allow(clippy::expect_used)]

    extern crate alloc;

    use alloc::vec;

    use hv_ept::{EptMapping, EptMemoryType, EptPartitionPlan, EptPermissions, EptPlan};
    use hv_types::{GuestPhysAddr, VmId};

    use super::*;
    use crate::init::GateCPlans;

    #[test]
    fn config_hash_mismatch_is_rejected_when_required() {
        let expected = ConfigHash([1; 32]);
        let boot = [2; 32];
        assert!(matches!(
            verify_config_hash(expected, boot, true),
            Err(RuntimeError::ConfigHashMismatch)
        ));
    }

    #[test]
    fn assign_ipc_host_backing_patches_ept_ipc_mappings() {
        let planned = HostPhysAddr::new(0x5000_0000);
        let mut backing = vec![0u8; 524_328];
        let mut plans = GateDPlans {
            gate_c: GateCPlans {
                ept: EptPlan {
                    partitions: vec![
                        EptPartitionPlan {
                            vm_id: VmId::new(1),
                            mappings: vec![
                                EptMapping {
                                    guest_phys: GuestPhysAddr::new(0),
                                    host_phys: HostPhysAddr::new(0x4000_0000),
                                    size: 1024,
                                    permissions: EptPermissions::GUEST_RAM,
                                    memory_type: EptMemoryType::WriteBack,
                                },
                                EptMapping {
                                    guest_phys: GuestPhysAddr::new(1024),
                                    host_phys: planned,
                                    size: 524_328,
                                    permissions: EptPermissions::GUEST_RAM,
                                    memory_type: EptMemoryType::WriteBack,
                                },
                            ],
                        },
                        EptPartitionPlan {
                            vm_id: VmId::new(2),
                            mappings: vec![EptMapping {
                                guest_phys: GuestPhysAddr::new(2048),
                                host_phys: planned,
                                size: 524_328,
                                permissions: EptPermissions::GUEST_RAM,
                                memory_type: EptMemoryType::WriteBack,
                            }],
                        },
                    ],
                },
                vtd: hv_vtd::VtdPlan { domains: vec![] },
                ept_table_base: HostPhysAddr::new(0x1_1510_0000),
                vtd_table_base: HostPhysAddr::new(0x1_1610_0000),
                vmxon_region_base: HostPhysAddr::new(0x1_1710_0000),
                ept_root_hp_as: vec![],
            },
            ipc_channels: vec![IpcChannelPlan {
                name: alloc::string::String::from("in_to_mid"),
                slot_count: 256,
                slot_size: 2048,
                host_base: planned,
                shared_bytes: 524_328,
            }],
            config_hash: ConfigHash([0; 32]),
            require_config_hash: false,
        };

        assign_ipc_host_backing(&mut plans, &mut backing).expect("assign");
        let expected = plans.ipc_channels[0].host_base;
        assert_ne!(expected, planned);
        for partition in &plans.gate_c.ept.partitions {
            for mapping in &partition.mappings {
                if mapping.size == 524_328 {
                    assert_eq!(mapping.host_phys, expected);
                }
            }
        }
    }
}
