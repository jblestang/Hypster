//! Exclusive-core CPU assignment planner.

use alloc::string::String;
use alloc::vec::Vec;

use hv_config_model::intent::StaticIntentIR;
use hv_config_model::raw::SmtPolicy;
use hv_types::{LogicalCpuId, PhysicalCoreId, VmId};

use crate::error::CpuTopologyError;
use crate::topology::CpuTopology;

/// Assignment of logical CPUs to a partition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuAssignment {
    /// Target VM.
    pub vm_id: VmId,
    /// Partition name.
    pub name: String,
    /// Dedicated physical core.
    pub physical_core: PhysicalCoreId,
    /// Logical CPU used as bootstrap vCPU for the partition.
    pub bootstrap_logical_cpu: LogicalCpuId,
}

/// CPU plan derived from topology and configuration intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuPlan {
    /// Per-partition core assignments.
    pub assignments: Vec<CpuAssignment>,
}

/// Builds an exclusive-core CPU plan for the configured partitions.
pub fn plan_cpu(
    intent: &StaticIntentIR,
    topology: &CpuTopology,
) -> Result<CpuPlan, CpuTopologyError> {
    let policy = intent.platform_requirements.smt_policy;
    if policy == SmtPolicy::Disabled && topology.smt_enabled() {
        return Err(CpuTopologyError::SmtPolicyViolation {
            reason: String::from("platform SMT is enabled but policy requires disabled"),
        });
    }

    let required_cores: u32 = intent.partitions.iter().map(|p| p.vcpus).sum();
    let available = topology.physical_core_count();
    if required_cores > available {
        return Err(CpuTopologyError::InsufficientCores { required: required_cores, available });
    }

    let mut assignments = Vec::with_capacity(intent.partitions.len());
    for (idx, partition) in intent.partitions.iter().enumerate() {
        if partition.vcpus != 1 && policy == SmtPolicy::ExclusiveCore {
            return Err(CpuTopologyError::TooManyVcpusPerCore {
                partition: partition.name.clone(),
                requested: partition.vcpus,
            });
        }
        let core = topology
            .cores
            .get(idx)
            .ok_or(CpuTopologyError::InsufficientCores { required: required_cores, available })?;
        let bootstrap = match core.logical_cpus.first() {
            Some(id) => *id,
            None => {
                return Err(CpuTopologyError::SmtPolicyViolation {
                    reason: String::from("core has no logical processors"),
                });
            }
        };
        assignments.push(CpuAssignment {
            vm_id: partition.vm_id,
            name: partition.name.clone(),
            physical_core: core.physical_core_id,
            bootstrap_logical_cpu: bootstrap,
        });
    }

    Ok(CpuPlan { assignments })
}
