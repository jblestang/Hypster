//! Platform resolution pipeline.

use hv_config_model::intent::StaticIntentIR;
use hv_cpu_topology::{plan_cpu, CpuTopology};
use hv_ept::plan_ept;
use hv_memory::plan_memory;
use hv_types::{ApicId, LogicalCpuId, PhysicalCoreId};
use hv_vtd::{plan_vtd, ObservedPciDevice as VtdObservedPci};

use crate::error::CoreError;
use crate::observed::ObservedPlatform;
use crate::platform_ir::StaticPlatformIR;
use crate::validate::validate_platform;

/// Validates the platform and runs Gate B planners.
pub fn resolve_platform(
    intent: &StaticIntentIR,
    observed: &ObservedPlatform,
) -> Result<StaticPlatformIR, CoreError> {
    let validated = validate_platform(&intent.platform_requirements, observed)
        .map_err(CoreError::Validation)?;

    let topology = topology_from_observed(observed);
    let cpu = plan_cpu(intent, &topology).map_err(CoreError::Cpu)?;
    let memory = plan_memory(intent, &observed.conventional_memory).map_err(CoreError::Memory)?;
    let ept = plan_ept(intent, &memory).map_err(CoreError::Ept)?;
    let observed_pci: alloc::vec::Vec<VtdObservedPci> = observed
        .pci_devices
        .iter()
        .map(|dev| VtdObservedPci { bdf: dev.bdf })
        .collect();
    let vtd = plan_vtd(intent, &observed_pci).map_err(CoreError::Vtd)?;

    Ok(StaticPlatformIR {
        config_hash: intent.config_hash,
        validated,
        intent: intent.clone(),
        cpu,
        memory,
        ept,
        vtd,
    })
}

fn topology_from_observed(observed: &ObservedPlatform) -> CpuTopology {
    let mut cores = alloc::vec::Vec::with_capacity(observed.physical_core_count as usize);
    let mut idx = 0u32;
    while idx < observed.physical_core_count {
        cores.push(hv_cpu_topology::CpuCore {
            physical_core_id: PhysicalCoreId::new(idx),
            apic_id: ApicId::new(idx),
            logical_cpus: alloc::vec![LogicalCpuId::new(idx)],
        });
        idx += 1;
    }
    CpuTopology { cores }
}
