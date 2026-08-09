//! Strongly typed identifiers and addresses for the Hypster hypervisor.
//!
//! All address and identity types are newtypes over fixed-width integers to
//! prevent accidental mixing of host physical, guest physical, and virtual
//! addresses, or conflating PCI topology fields with VM identifiers.
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | Checked arithmetic | UNIT + PROPERTY |
//! | Newtype invariants | UNIT |

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

pub mod addr;
pub mod arithmetic;
pub mod ids;
pub mod pci;

pub use addr::{GuestPhysAddr, GuestVirtAddr, HostPhysAddr, HostVirtAddr, Iova};
pub use ids::{
    ApicId, InterruptVector, IommuDomainId, LogicalCpuId, PackageId, PhysicalCoreId, VcpuId,
    VmId,
};
pub use pci::{PciBdf, PciBus, PciDevice, PciFunction, PciSegment};
