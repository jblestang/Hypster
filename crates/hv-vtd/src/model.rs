//! VT-d plan model types (no configuration dependency).

use alloc::string::String;
use alloc::vec::Vec;

use hv_types::{HostPhysAddr, IommuDomainId, Iova, VmId};

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
    /// DMA mappings.
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
    pub bdf: hv_types::PciBdf,
}
