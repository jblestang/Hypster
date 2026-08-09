//! VT-d / IOMMU planning (software-only, pre-hardware Gate B).

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod plan;

pub use error::VtdPlanError;
pub use plan::{plan_vtd, ObservedPciDevice, VtdDomainPlan, VtdMapping, VtdPlan};
