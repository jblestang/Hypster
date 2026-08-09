//! EPT planning and hardware table installation.

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod install;
pub mod model;
#[cfg(feature = "plan")]
pub mod plan;
pub mod table;
pub mod types;

pub use error::EptPlanError;
pub use install::{install_ept_mappings, EptInstallResult, EptPageTable};
pub use model::{EptPartitionPlan, EptPlan};
#[cfg(feature = "plan")]
pub use plan::plan_ept;
pub use types::{EptMapping, EptMemoryType, EptPermissions};
