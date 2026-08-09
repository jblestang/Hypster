//! Gate D plan construction from resolved platform IR (host-only).

use hv_core::platform_ir::StaticPlatformIR;
use hv_memory::plan::MemoryPurpose;
use hv_runtime::{GateCPlans, GateDPlans, IpcChannelPlan};
use hv_types::HostPhysAddr;

/// Builds Gate D runtime plans from a resolved platform.
#[must_use]
pub fn gate_d_plans_from_platform(
    platform: &StaticPlatformIR,
    gate_c: GateCPlans,
    require_config_hash: bool,
) -> GateDPlans {
    let ipc_channels = platform
        .intent
        .ipc
        .iter()
        .filter_map(|channel| {
            let region = platform.memory.regions.iter().find(|region| {
                matches!(&region.purpose, MemoryPurpose::IpcChannel { name } if name == &channel.name)
            })?;
            Some(IpcChannelPlan {
                name: channel.name.clone(),
                slot_count: channel.slot_count,
                slot_size: channel.slot_size,
                host_base: region.base,
                shared_bytes: channel.shared_bytes,
            })
        })
        .collect();
    GateDPlans { gate_c, ipc_channels, config_hash: platform.config_hash, require_config_hash }
}

/// Gate C MVP table bases used by the hypervisor image.
pub const EPT_TABLE_BASE: HostPhysAddr = HostPhysAddr::new(0x2000_0000);
/// Gate C MVP VT-d table buffer base (16 MiB reserved).
pub const VTD_TABLE_BASE: HostPhysAddr = HostPhysAddr::new(0x2100_0000);
/// Gate C MVP VMXON region base (4 KiB reserved).
pub const VMXON_REGION_BASE: HostPhysAddr = HostPhysAddr::new(0x2200_0000);

/// Builds Gate D plans with resolved EPT/VT-d tables from platform resolution.
#[must_use]
pub fn gate_d_plans_from_resolved(platform: &StaticPlatformIR) -> GateDPlans {
    let gate_c = GateCPlans {
        ept: platform.ept.clone(),
        vtd: platform.vtd.clone(),
        ept_table_base: EPT_TABLE_BASE,
        vtd_table_base: VTD_TABLE_BASE,
        vmxon_region_base: VMXON_REGION_BASE,
    };
    gate_d_plans_from_platform(platform, gate_c, true)
}
