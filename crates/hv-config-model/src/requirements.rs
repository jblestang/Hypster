//! Platform requirements contract derived from normalized configuration.

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

use crate::normalize::{ArchRequirement, NormalizedConfig, NormalizedPlatformRequirements};
use crate::raw::{RequirementLevel, SmtPolicy};

/// Formal platform contract that must be satisfied by observed hardware.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformRequirements {
    /// Configuration name.
    pub config_name: String,
    /// Architecture requirement.
    pub arch: ArchRequirement,
    /// VMX requirement.
    pub vmx: RequirementLevel,
    /// EPT requirement.
    pub ept: RequirementLevel,
    /// VT-d requirement.
    pub vtd: RequirementLevel,
    /// Minimum physical cores.
    pub min_physical_cores: u32,
    /// Total vCPUs allocated to partitions.
    pub total_partition_vcpus: u32,
    /// SMT policy.
    pub smt_policy: SmtPolicy,
    /// Minimum host RAM in bytes including hypervisor reservation headroom.
    pub min_ram_bytes: u64,
    /// Sum of partition-private RAM in bytes.
    pub total_partition_ram_bytes: u64,
    /// Interrupt remapping requirement.
    pub interrupt_remapping: RequirementLevel,
    /// x2APIC requirement.
    pub x2apic: RequirementLevel,
    /// Invariant TSC requirement.
    pub invariant_tsc: RequirementLevel,
    /// VPID requirement.
    pub vpid: RequirementLevel,
    /// VMX preemption timer requirement.
    pub vmx_preemption_timer: RequirementLevel,
    /// NX requirement.
    pub nx: RequirementLevel,
    /// Required guest page sizes.
    pub page_sizes: Vec<u64>,
    /// PCI devices that must exist and be assignable.
    pub expected_pci_devices: Vec<ExpectedPciDevice>,
}

/// PCI device expected by the validated configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedPciDevice {
    /// Owning partition name.
    pub owner: alloc::string::String,
    /// Device kind string.
    pub kind: alloc::string::String,
    /// BDF string in canonical form.
    pub bdf: alloc::string::String,
}

impl PlatformRequirements {
    /// Builds the platform contract from a normalized configuration.
    #[must_use]
    pub fn from_normalized(config: &NormalizedConfig) -> Self {
        let req: &NormalizedPlatformRequirements = &config.platform.requirements;
        let total_partition_vcpus = config.partitions.iter().map(|p| p.vcpus).sum();
        let total_partition_ram_bytes = config
            .partitions
            .iter()
            .map(|p| p.memory_bytes)
            .fold(0u64, |acc, v| acc.saturating_add(v));

        let mut expected_pci_devices = Vec::new();
        for partition in &config.partitions {
            for device in &partition.devices {
                expected_pci_devices.push(ExpectedPciDevice {
                    owner: partition.name.clone(),
                    kind: format!("{:?}", device.kind).to_ascii_lowercase(),
                    bdf: device.bdf.to_string(),
                });
            }
        }

        Self {
            config_name: config.name.clone(),
            arch: req.arch,
            vmx: req.vmx,
            ept: req.ept,
            vtd: req.vtd,
            min_physical_cores: req.min_physical_cores,
            total_partition_vcpus,
            smt_policy: req.smt_policy,
            min_ram_bytes: req.min_ram_bytes,
            total_partition_ram_bytes,
            interrupt_remapping: req.interrupt_remapping,
            x2apic: req.x2apic,
            invariant_tsc: req.invariant_tsc,
            vpid: req.vpid,
            vmx_preemption_timer: req.vmx_preemption_timer,
            nx: req.nx,
            page_sizes: req.page_sizes.clone(),
            expected_pci_devices,
        }
    }

    /// Returns true when a required feature must fail closed if absent.
    #[must_use]
    pub const fn is_fail_closed(level: RequirementLevel) -> bool {
        matches!(level, RequirementLevel::Required)
    }
}
