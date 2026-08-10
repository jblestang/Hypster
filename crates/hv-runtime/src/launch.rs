//! Gate D guest partition launch planning.

use hv_types::{GuestPhysAddr, HostPhysAddr, VmId};
use hv_vmx::{build_guest_vmcs_fields, GuestLaunchPlan};

use crate::error::RuntimeError;
use crate::partition::GateDPlans;

/// Gate D MVP guest entry when no ELF is loaded yet.
pub const DEFAULT_GUEST_ENTRY: u64 = 0x1000;
/// Gate D MVP guest stack top.
pub const DEFAULT_GUEST_STACK: u64 = 0x8000;
/// Gate D MVP guest boot info GPA.
pub const DEFAULT_GUEST_BOOT_INFO_GPA: u64 = 0x9000;

const VMCS_REGION_SIZE: u64 = 4096;

fn ept_root_for_vm(plans: &GateDPlans, vm_id: VmId) -> HostPhysAddr {
    plans
        .gate_c
        .ept_root_hp_as
        .iter()
        .find(|(id, _)| *id == vm_id)
        .map(|(_, root)| *root)
        .unwrap_or(plans.gate_c.ept_table_base)
}

/// Builds launch plans for all partitions described in Gate D plans.
#[must_use]
pub fn plan_partition_launches(plans: &GateDPlans) -> alloc::vec::Vec<GuestLaunchPlan> {
    let mut out = alloc::vec::Vec::with_capacity(plans.gate_c.ept.partitions.len());
    let vmcs_base = plans.gate_c.vmcs_region_base.raw();
    for partition in &plans.gate_c.ept.partitions {
        let vmcs_hpa =
            HostPhysAddr::new(vmcs_base + (partition.vm_id.raw() as u64) * VMCS_REGION_SIZE);
        out.push(GuestLaunchPlan {
            vm_id: partition.vm_id,
            vmcs_hpa,
            guest_entry: GuestPhysAddr::new(DEFAULT_GUEST_ENTRY),
            guest_stack: GuestPhysAddr::new(DEFAULT_GUEST_STACK),
            ept_root_hpa: ept_root_for_vm(plans, partition.vm_id),
            guest_boot_info_gpa: GuestPhysAddr::new(DEFAULT_GUEST_BOOT_INFO_GPA),
        });
    }
    out
}

/// Summary of guest launch preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaunchPrepReport {
    /// Number of partitions with valid launch plans.
    pub planned_launches: usize,
    /// Number of e1000 MMIO devices across all partitions.
    pub mmio_devices: usize,
    /// Number of VMCS field sets generated.
    pub vmcs_field_sets: usize,
}

/// Validates launch plans and builds VMCS field sets without executing VMLAUNCH.
///
/// # Errors
///
/// Returns [`RuntimeError::Vmx`] when a launch plan is invalid.
pub fn prepare_partition_launches(plans: &GateDPlans) -> Result<LaunchPrepReport, RuntimeError> {
    let launches = plan_partition_launches(plans);
    let mut vmcs_field_sets = 0usize;
    for launch in &launches {
        build_guest_vmcs_fields(launch).map_err(RuntimeError::Vmx)?;
        vmcs_field_sets += 1;
    }

    let mmio_devices = plans
        .gate_c
        .ept
        .partitions
        .iter()
        .map(|partition| {
            partition
                .mappings
                .iter()
                .filter(|mapping| mapping.memory_type == hv_ept::EptMemoryType::Uncacheable)
                .count()
        })
        .sum();

    Ok(LaunchPrepReport { planned_launches: launches.len(), mmio_devices, vmcs_field_sets })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::init::GateCPlans;
    use hv_ept::{EptMapping, EptMemoryType, EptPartitionPlan, EptPermissions, EptPlan};
    use hv_types::VmId;
    use hv_vtd::VtdPlan;

    fn sample_plans() -> GateDPlans {
        GateDPlans {
            gate_c: GateCPlans {
                ept: EptPlan {
                    partitions: alloc::vec![EptPartitionPlan {
                        vm_id: VmId::new(0),
                        mappings: alloc::vec![
                            EptMapping {
                                guest_phys: GuestPhysAddr::new(0),
                                host_phys: HostPhysAddr::new(0x4000_0000),
                                size: 1024 * 1024,
                                permissions: EptPermissions::GUEST_RAM,
                                memory_type: EptMemoryType::WriteBack,
                            },
                            EptMapping {
                                guest_phys: GuestPhysAddr::new(0xFEB0_0000),
                                host_phys: HostPhysAddr::new(0x1_1720_0000),
                                size: 128 * 1024,
                                permissions: EptPermissions::MMIO,
                                memory_type: EptMemoryType::Uncacheable,
                            },
                        ],
                    }],
                },
                vtd: VtdPlan { domains: alloc::vec::Vec::new() },
                ept_table_base: HostPhysAddr::new(0x1_1510_0000),
                vtd_table_base: HostPhysAddr::new(0x1_1610_0000),
                vmxon_region_base: HostPhysAddr::new(0x1_1710_0000),
                vmcs_region_base: HostPhysAddr::new(0x1_1800_0000),
                ept_root_hp_as: alloc::vec![(VmId::new(0), HostPhysAddr::new(0x1_1510_1000))],
            },
            ipc_channels: alloc::vec::Vec::new(),
            config_hash: hv_config_model::hash::ConfigHash([0; 32]),
            require_config_hash: false,
        }
    }

    #[test]
    fn launch_prep_builds_vmcs_fields_for_each_partition() {
        let report = prepare_partition_launches(&sample_plans()).expect("launch prep");
        assert_eq!(report.planned_launches, 1);
        assert_eq!(report.vmcs_field_sets, 1);
        assert_eq!(report.mmio_devices, 1);
    }
}
