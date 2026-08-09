//! Host memory planning from UEFI memory map and configuration intent.

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod error;
pub mod map;
pub mod plan;

pub use error::MemoryPlanError;
pub use map::{ConventionalRegion, UefiMemoryType};
pub use plan::{plan_memory, MemoryPlan, MemoryPurpose, MemoryRegionPlan};

/// Hypervisor private reservation in bytes (MVP Gate B constant).
pub const HYPERVISOR_RESERVE_BYTES: u64 = 64 * 1024 * 1024;

/// Default page size used for planner alignment.
pub const PLANNER_PAGE_SIZE: u64 = 4096;
