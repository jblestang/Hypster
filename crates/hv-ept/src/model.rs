//! EPT plan model types (no configuration dependency).

use alloc::vec::Vec;

use hv_types::VmId;

use crate::types::EptMapping;

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
