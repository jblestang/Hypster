//! Resolved static platform intermediate representation.

use hv_config_model::hash::ConfigHash;
use hv_config_model::intent::StaticIntentIR;
use hv_cpu_topology::CpuPlan;
use hv_ept::EptPlan;
use hv_memory::MemoryPlan;
use hv_vtd::VtdPlan;

use crate::validated::ValidatedPlatform;

/// Fully resolved static platform plan (pre-VMX hardware install).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticPlatformIR {
    /// Configuration hash.
    pub config_hash: ConfigHash,
    /// Validated platform contract.
    pub validated: ValidatedPlatform,
    /// Original static intent.
    pub intent: StaticIntentIR,
    /// CPU plan.
    pub cpu: CpuPlan,
    /// Memory plan.
    pub memory: MemoryPlan,
    /// EPT plan.
    pub ept: EptPlan,
    /// VT-d plan.
    pub vtd: VtdPlan,
}
