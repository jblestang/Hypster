//! Intel VMX hardware programming (Gate C).

#![no_std]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

pub mod caps;
pub mod error;
pub mod host;
mod instr;
pub mod region;
pub mod vmcs;

pub use caps::VmxCapabilities;
pub use error::VmxError;
pub use host::{VmxHostInit, VmxHostState};
pub use region::{VmxonRegion, VMXON_REGION_SIZE};
pub use vmcs::{
    VmcsRegion, HOST_CR0, HOST_CR3, HOST_CR4, HOST_FS_BASE, HOST_GDTR_BASE, HOST_GS_BASE,
    HOST_IDTR_BASE, HOST_RIP, HOST_RSP, HOST_SYSENTER_CS, HOST_SYSENTER_EIP, HOST_SYSENTER_ESP,
    HOST_TR_BASE, VMCS_REGION_SIZE,
};
