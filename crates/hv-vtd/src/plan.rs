//! VT-d domain and DMA mapping planner.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use hv_config_model::intent::StaticIntentIR;
use hv_types::{HostPhysAddr, IommuDomainId, Iova, PciBdf, VmId};

use crate::error::VtdPlanError;

/// One IOVA mapping inside a domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtdMapping {
    /// IOVA base.
    pub iova: Iova,
    /// Host physical base.
    pub host_phys: HostPhysAddr,
    /// Mapping size in bytes.
    pub size: u64,
}

/// VT-d domain plan for one partition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtdDomainPlan {
    /// Domain identifier.
    pub domain_id: IommuDomainId,
    /// Owning VM.
    pub vm_id: VmId,
    /// Assigned device BDF strings.
    pub devices: Vec<String>,
    /// DMA mappings (identity for MVP guest RAM pages used by devices is out of scope).
    pub mappings: Vec<VtdMapping>,
}

/// Full VT-d plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtdPlan {
    /// Domain plans.
    pub domains: Vec<VtdDomainPlan>,
}

/// Observed PCI device on the platform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedPciDevice {
    /// Parsed BDF.
    pub bdf: PciBdf,
}

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
            let parsed = PciBdf::parse(&device.bdf).map_err(|_| VtdPlanError::MissingPciDevice {
                bdf: device.bdf.clone(),
            })?;
            let present = observed_pci.iter().any(|obs| obs.bdf == parsed);
            if !present {
                return Err(VtdPlanError::MissingPciDevice {
                    bdf: device.bdf.clone(),
                });
            }
            if !seen.insert(device.bdf.clone()) {
                return Err(VtdPlanError::DeviceOwnershipConflict {
                    bdf: device.bdf.clone(),
                });
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
