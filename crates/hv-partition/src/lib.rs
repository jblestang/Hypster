//! Guest partition boot info construction.
//!
//! Host-side builders translate resolved platform plans into guest-visible
//! [`GuestBootInfo`] blobs. Guests consume the blob at launch and must not
//! hardcode topology constants.
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | Guest boot info layout | UNIT |
//! | IPC role assignment | UNIT + Gate D integration |

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Guest boot info serialization uses raw struct writes.

extern crate alloc;

#[cfg(feature = "std")]
mod builder;
mod error;
#[cfg(feature = "std")]
mod gate_d;
#[cfg(feature = "std")]
mod ipc_guest_phys;
#[cfg(feature = "std")]
mod layout;

#[cfg(feature = "std")]
pub use builder::{build_guest_boot_info, GuestBootInfoBlob};
pub use error::PartitionError;
#[cfg(feature = "std")]
pub use gate_d::{
    gate_d_plans_from_platform, gate_d_plans_from_resolved, EPT_TABLE_BASE, VMXON_REGION_BASE,
    VTD_TABLE_BASE,
};
#[cfg(feature = "std")]
pub use ipc_guest_phys::{ipc_guest_phys_base, validate_ipc_layout};
#[cfg(feature = "std")]
pub use layout::{
    mmio_guest_phys, mmio_host_phys, vmcs_region_hpa, MMIO_GUEST_BASE, MMIO_GUEST_STRIDE,
    MMIO_HOST_BASE, MMIO_REGION_SIZE, VMCS_REGION_BASE, VMCS_REGION_SIZE,
};
