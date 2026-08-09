//! VT-d / IOMMU planning and hardware table installation.

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

pub use error::VtdPlanError;
pub use install::{
    install_vtd_domains, VtdContextTables, VtdHardwareState, VtdInstallResult,
};
pub use model::{ObservedPciDevice, VtdDomainPlan, VtdMapping, VtdPlan};
#[cfg(feature = "plan")]
pub use plan::plan_vtd;
