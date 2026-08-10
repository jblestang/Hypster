//! Guest boot-info discovery helpers.

use hv_guest_abi::layout;
use hv_guest_abi::GuestBootInfo;
use hv_ipc::ipc_name_hash;

/// Guest-physical address where the hypervisor writes [`GuestBootInfo`].
pub const GUEST_BOOT_INFO_GPA: u64 = 0x9000;

/// Returns the boot-info pointer at [`GUEST_BOOT_INFO_GPA`].
///
/// # Safety
///
/// Caller must ensure the hypervisor mapped and initialized the boot-info blob.
pub const fn boot_info_ptr() -> *const GuestBootInfo {
    GUEST_BOOT_INFO_GPA as *const GuestBootInfo
}

/// Looks up the guest-physical base for one IPC channel by name.
#[must_use]
pub fn ipc_region_base(info: &GuestBootInfo, name: &str) -> Option<u64> {
    let hash = ipc_name_hash(name);
    // SAFETY: hypervisor provides a validated boot-info blob before guest entry.
    let regions = unsafe { layout::ipc_regions_slice(info) };
    regions.iter().find(|region| region.name_hash == hash).map(|region| region.base)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    extern crate alloc;

    use hv_config_model::pipeline::compile_config;
    use hv_config_model::yaml::read_yaml_file;
    use hv_core::fixture::qemu_validation_observed;
    use hv_core::resolve_platform;
    use hv_partition::build_guest_boot_info;
    use hv_types::{VcpuId, VmId};

    use super::*;
    use crate::topology::names;

    #[test]
    fn ipc_region_base_resolves_mid_channels_from_boot_info() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
        let raw = read_yaml_file(path).expect("read config");
        let compiled = compile_config(raw).expect("compile config");
        let observed = qemu_validation_observed().expect("fixture");
        let platform = resolve_platform(&compiled.intent, &observed).expect("resolve");
        let blob = build_guest_boot_info(&platform, VmId::new(1), VcpuId::new(0)).expect("build");
        let info = unsafe { &*(blob.bytes.as_ptr() as *const GuestBootInfo) };
        let in_to_mid = ipc_region_base(info, names::IN_TO_MID).expect("in_to_mid");
        let mid_to_out = ipc_region_base(info, names::MID_TO_OUT).expect("mid_to_out");
        assert!(mid_to_out > in_to_mid);
    }
}
