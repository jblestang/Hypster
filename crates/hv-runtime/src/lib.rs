//! Hypervisor runtime initialization after UEFI handoff (Gate C).

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Identity-mapped firmware tables and loader-reserved regions.

extern crate alloc;

mod error;
mod init;
mod launch;
mod partition;

pub use error::RuntimeError;
pub use init::{initialize, initialize_gate_d, GateCPlans, GateDInitReport, RuntimeInitReport};
pub use launch::{
    plan_partition_launches, prepare_partition_launches, LaunchPrepReport,
    DEFAULT_GUEST_BOOT_INFO_GPA, DEFAULT_GUEST_ENTRY, DEFAULT_GUEST_STACK,
};
pub use partition::{
    prepare_ipc_rings, verify_config_hash, GateDPlans, IpcChannelPlan, PartitionPrepReport,
};
