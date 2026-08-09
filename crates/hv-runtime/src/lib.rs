//! Hypervisor runtime initialization after UEFI handoff (Gate C).

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Identity-mapped firmware tables and loader-reserved regions.

extern crate alloc;

mod datapath;
mod error;
mod init;
mod execute;
mod launch;
mod load;
mod mmio;
mod partition;
mod run;
mod vmexit;

pub use datapath::{
    channel_backings_from_plans, drain_outbound_payload, inject_inbound_payload, run_datapath_step,
    ChannelBacking, DatapathChannels, DatapathStepReport,
};
pub use error::RuntimeError;
pub use init::{initialize, initialize_gate_d, GateCPlans, GateDInitReport, RuntimeInitReport};
pub use execute::{
    capture_host_launch_context, launch_plan_for_vm, read_vmexit_reason, resume_guest,
    stage_guest_image, vmlaunch_guest, HostLaunchContext,
};
pub use launch::{
    plan_partition_launches, prepare_partition_launches, LaunchPrepReport,
    DEFAULT_GUEST_BOOT_INFO_GPA, DEFAULT_GUEST_ENTRY, DEFAULT_GUEST_STACK,
};
pub use load::{load_elf_into_guest_ram, LoadedGuestImage};
pub use mmio::{MmioDevice, MmioDispatch};
pub use partition::{
    assign_ipc_host_backing, prepare_ipc_rings, verify_config_hash, GateDPlans, IpcChannelPlan,
    PartitionPrepReport,
};
pub use run::{prepare_datapath, DatapathEngine, DatapathPrepReport, E2eTransferReport};
pub use vmexit::{
    classify_exit_reason, handle_mmio_exit, reason as vmexit_reason, MmioExitAction, MmioExitInfo,
};
