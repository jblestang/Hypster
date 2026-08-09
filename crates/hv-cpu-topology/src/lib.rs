//! CPU topology types and exclusive-core planner.

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod plan;
pub mod topology;

pub use error::CpuTopologyError;
pub use plan::{plan_cpu, CpuAssignment, CpuPlan};
pub use topology::{CpuCore, CpuTopology};
