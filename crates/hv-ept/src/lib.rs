//! EPT planning (software-only, pre-VMX Gate B).

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod plan;
pub mod types;

pub use error::EptPlanError;
pub use plan::{plan_ept, EptPartitionPlan, EptPlan};
pub use types::{EptMapping, EptMemoryType, EptPermissions};
