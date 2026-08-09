//! EPT planner.

use alloc::vec;
use alloc::vec::Vec;

use hv_config_model::intent::StaticIntentIR;
use hv_memory::plan::{MemoryPlan, MemoryPurpose};
use hv_types::arithmetic::{checked_add_u64, ranges_overlap_u64};
use hv_types::{GuestPhysAddr, VmId};

use crate::error::EptPlanError;
use crate::model::{EptPartitionPlan, EptPlan};
use crate::types::{EptMapping, EptMemoryType, EptPermissions};

/// Builds identity guest RAM and IPC shared-memory EPT mappings from the memory plan.
pub fn plan_ept(intent: &StaticIntentIR, memory: &MemoryPlan) -> Result<EptPlan, EptPlanError> {
    let mut partitions = Vec::with_capacity(intent.partitions.len());
    for partition in intent.partitions.iter() {
        let backing = memory.regions.iter().find(|region| {
            matches!(
                region.purpose,
                MemoryPurpose::PartitionGuestRam {
                    vm_id
                } if vm_id == partition.vm_id.raw()
            )
        });
        let Some(backing) = backing else {
            return Err(EptPlanError::MissingGuestRam { vm_id: partition.vm_id.raw() });
        };
        let mut mappings = vec![EptMapping {
            guest_phys: GuestPhysAddr::new(0),
            host_phys: backing.base,
            size: backing.size,
            permissions: EptPermissions::GUEST_RAM,
            memory_type: EptMemoryType::WriteBack,
        }];

        append_ipc_mappings(intent, memory, partition.vm_id, &mut mappings)?;
        append_mmio_mappings(partition, &mut mappings)?;
        validate_partition(partition.vm_id, &mappings)?;
        partitions.push(EptPartitionPlan { vm_id: partition.vm_id, mappings });
    }
    Ok(EptPlan { partitions })
}

fn append_ipc_mappings(
    intent: &StaticIntentIR,
    memory: &MemoryPlan,
    vm_id: VmId,
    mappings: &mut Vec<EptMapping>,
) -> Result<(), EptPlanError> {
    let partition = intent
        .partitions
        .iter()
        .find(|part| part.vm_id == vm_id)
        .ok_or(EptPlanError::MissingGuestRam { vm_id: vm_id.raw() })?;
    let mut cursor = partition.memory_bytes;
    for channel in intent.ipc.iter() {
        if channel.producer != vm_id && channel.consumer != vm_id {
            continue;
        }
        let host = memory.regions.iter().find(|region| {
            matches!(&region.purpose, MemoryPurpose::IpcChannel { name } if name == &channel.name)
        });
        let Some(host) = host else {
            return Err(EptPlanError::MissingIpcChannel {
                vm_id: vm_id.raw(),
                channel: channel.name.clone(),
            });
        };
        mappings.push(EptMapping {
            guest_phys: GuestPhysAddr::new(cursor),
            host_phys: host.base,
            size: channel.shared_bytes,
            permissions: EptPermissions::GUEST_RAM,
            memory_type: EptMemoryType::WriteBack,
        });
        cursor = cursor.checked_add(channel.shared_bytes).ok_or(EptPlanError::Overflow)?;
    }
    Ok(())
}

fn append_mmio_mappings(
    partition: &hv_config_model::intent::PartitionIntent,
    mappings: &mut Vec<EptMapping>,
) -> Result<(), EptPlanError> {
    const MMIO_GUEST_BASE: u64 = 0xFEB0_0000;
    const MMIO_GUEST_STRIDE: u64 = 0x10_0000;
    const MMIO_REGION_SIZE: u64 = 128 * 1024;
    const MMIO_HOST_BASE: u64 = 0x1_1720_0000;
    const MMIO_HOST_STRIDE: u64 = 0x10_0000;

    for (idx, _device) in partition.devices.iter().enumerate() {
        mappings.push(EptMapping {
            guest_phys: GuestPhysAddr::new(MMIO_GUEST_BASE + (idx as u64) * MMIO_GUEST_STRIDE),
            host_phys: hv_types::HostPhysAddr::new(
                MMIO_HOST_BASE
                    + (partition.vm_id.raw() as u64) * MMIO_HOST_STRIDE
                    + (idx as u64) * MMIO_HOST_STRIDE,
            ),
            size: MMIO_REGION_SIZE,
            permissions: EptPermissions::MMIO,
            memory_type: EptMemoryType::Uncacheable,
        });
    }
    Ok(())
}

fn validate_partition(vm_id: VmId, mappings: &[EptMapping]) -> Result<(), EptPlanError> {
    for (i, left) in mappings.iter().enumerate() {
        for right in mappings.iter().skip(i + 1) {
            if ranges_overlap_u64(
                left.guest_phys.raw(),
                left.size,
                right.guest_phys.raw(),
                right.size,
            ) {
                return Err(EptPlanError::MappingOverlap { vm_id: vm_id.raw() });
            }
        }
        let _ = checked_add_u64(left.guest_phys.raw(), left.size)
            .map_err(|_| EptPlanError::Overflow)?;
    }
    Ok(())
}
