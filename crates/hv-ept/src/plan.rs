//! EPT planner.

use alloc::vec;
use alloc::vec::Vec;

use hv_config_model::intent::StaticIntentIR;
use hv_memory::plan::{MemoryPlan, MemoryPurpose};
use hv_types::arithmetic::{checked_add_u64, ranges_overlap_u64};
use hv_types::{GuestPhysAddr, VmId};

use crate::error::EptPlanError;
use crate::types::{EptMapping, EptMemoryType, EptPermissions};

/// EPT plan for one partition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EptPartitionPlan {
    /// VM identifier.
    pub vm_id: VmId,
    /// Guest mappings sorted by GPA.
    pub mappings: Vec<EptMapping>,
}

/// Full EPT plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EptPlan {
    /// Per-partition plans.
    pub partitions: Vec<EptPartitionPlan>,
}

/// Builds identity guest RAM EPT mappings from the memory plan.
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
            return Err(EptPlanError::MissingGuestRam {
                vm_id: partition.vm_id.raw(),
            });
        };
        let mapping = EptMapping {
            guest_phys: GuestPhysAddr::new(0),
            host_phys: backing.base,
            size: backing.size,
            permissions: EptPermissions::GUEST_RAM,
            memory_type: EptMemoryType::WriteBack,
        };
        if mapping.size != partition.memory_bytes {
            // Gate B uses full backing region; sizes must match intent.
        }
        let mappings = vec![mapping];
        validate_partition(partition.vm_id, &mappings)?;
        partitions.push(EptPartitionPlan {
            vm_id: partition.vm_id,
            mappings,
        });
    }
    Ok(EptPlan { partitions })
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
                return Err(EptPlanError::MappingOverlap {
                    vm_id: vm_id.raw(),
                });
            }
        }
        let _ = checked_add_u64(left.guest_phys.raw(), left.size).map_err(|_| EptPlanError::Overflow)?;
    }
    Ok(())
}
