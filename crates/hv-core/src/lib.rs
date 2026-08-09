//! Hypster core: platform observation, contract validation, and resolution.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(feature = "std")]
extern crate alloc;

pub mod boot;
pub mod error;

#[cfg(feature = "std")]
pub mod fixture;
#[cfg(feature = "std")]
pub mod observed;
#[cfg(feature = "std")]
pub mod platform_ir;
#[cfg(feature = "std")]
pub mod resolve;
#[cfg(feature = "std")]
pub mod validate;
#[cfg(feature = "std")]
pub mod validated;

pub use boot::{BootPhase, BootTransitionError};
pub use error::CoreError;

#[cfg(feature = "std")]
pub use observed::{ObservedCpuFeatures, ObservedPlatform, ObservedPciDevice};
#[cfg(feature = "std")]
pub use platform_ir::StaticPlatformIR;
#[cfg(feature = "std")]
pub use resolve::resolve_platform;
#[cfg(feature = "std")]
pub use validate::validate_platform;
#[cfg(feature = "std")]
pub use validated::ValidatedPlatform;
