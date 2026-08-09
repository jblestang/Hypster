//! VT-d domain and DMA mapping planner.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;

use hv_config_model::intent::StaticIntentIR;
use hv_types::PciBdf;

use crate::error::VtdPlanError;
use crate::model::{ObservedPciDevice, VtdDomainPlan, VtdPlan};

/// Builds VT-d domain plans from intent and observed PCI devices.
pub fn plan_vtd(
    intent: &StaticIntentIR,
    observed_pci: &[ObservedPciDevice],
) -> Result<VtdPlan, VtdPlanError> {
    let mut seen = BTreeSet::new();
    let mut domains = Vec::with_capacity(intent.partitions.len());

    for partition in intent.partitions.iter() {
        let mut devices = Vec::new();
        for device in &partition.devices {
            let parsed = PciBdf::parse(&device.bdf)
                .map_err(|_| VtdPlanError::MissingPciDevice { bdf: device.bdf.clone() })?;
            let present = observed_pci.iter().any(|obs| obs.bdf == parsed);
            if !present {
                return Err(VtdPlanError::MissingPciDevice { bdf: device.bdf.clone() });
            }
            if !seen.insert(device.bdf.clone()) {
                return Err(VtdPlanError::DeviceOwnershipConflict { bdf: device.bdf.clone() });
            }
            devices.push(device.bdf.clone());
        }
        domains.push(VtdDomainPlan {
            domain_id: partition.iommu_domain,
            vm_id: partition.vm_id,
            devices,
            mappings: Vec::new(),
        });
    }

    Ok(VtdPlan { domains })
}
