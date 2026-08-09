//! Hypervisor runtime initialization after UEFI handoff (Gate C).

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Identity-mapped firmware tables and loader-reserved regions.

extern crate alloc;

mod error;
mod init;
mod partition;

pub use error::RuntimeError;
pub use init::{initialize, initialize_gate_d, GateCPlans, GateDInitReport, RuntimeInitReport};
pub use partition::{
    prepare_ipc_rings, verify_config_hash, GateDPlans, IpcChannelPlan, PartitionPrepReport,
};
