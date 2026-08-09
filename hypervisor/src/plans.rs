//! Gate C MVP static plans with hardcoded table regions.
//!
//! Table bases are reserved by the memory planner for Gate C:
//! - EPT roots at `0x2000_0000`
//! - VT-d structures at `0x2100_0000`

use hv_ept::EptPlan;
use hv_runtime::{GateCPlans, RuntimeInitReport};
use hv_types::HostPhysAddr;
use hv_vtd::VtdPlan;

/// Gate C MVP EPT table buffer base (16 MiB reserved).
pub const EPT_TABLE_BASE: HostPhysAddr = HostPhysAddr::new(0x2000_0000);
/// Gate C MVP VT-d table buffer base (16 MiB reserved).
pub const VTD_TABLE_BASE: HostPhysAddr = HostPhysAddr::new(0x2100_0000);

/// Gate C MVP VMXON region base (4 KiB reserved).
pub const VMXON_REGION_BASE: HostPhysAddr = HostPhysAddr::new(0x2200_0000);

/// Builds Gate C plans with empty partition/domain lists for MVP bring-up.
#[must_use]
pub fn gate_c_plans() -> GateCPlans {
    GateCPlans {
        ept: EptPlan {
            partitions: alloc::vec::Vec::new(),
        },
        vtd: VtdPlan {
            domains: alloc::vec::Vec::new(),
        },
        ept_table_base: EPT_TABLE_BASE,
        vtd_table_base: VTD_TABLE_BASE,
        vmxon_region_base: VMXON_REGION_BASE,
    }
}

/// Runs runtime init and returns the report (used by entry).
pub fn run_gate_c_init(
    boot_info: &hv_boot_abi::BootInfo,
) -> Result<RuntimeInitReport, hv_runtime::RuntimeError> {
    let plans = gate_c_plans();
    hv_runtime::initialize(boot_info, &plans)
}
