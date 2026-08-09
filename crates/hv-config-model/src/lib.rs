//! Configuration model for the Hypster static hypervisor.
//!
//! Pipeline:
//!
//! ```text
//! YAML -> RawConfig -> validation -> NormalizedConfig
//!      -> PlatformRequirements -> StaticIntentIR
//! ```
//!
//! The runtime never hardcodes partition names or counts; it consumes the
//! validated IR produced from configuration YAML.
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | YAML parsing | UNIT + FUZZ |
//! | Semantic validation | UNIT + PROPERTY |
//! | Deterministic IDs | UNIT + PROPERTY |
//! | Config hash | UNIT |
//! | IR determinism | UNIT + PROPERTY |

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

extern crate alloc;

#[cfg(feature = "std")]
pub mod artifact;
pub mod error;
pub mod hash;
#[cfg(feature = "std")]
pub mod intent;
#[cfg(feature = "std")]
pub mod normalize;
#[cfg(feature = "std")]
pub mod pipeline;
#[cfg(feature = "std")]
pub mod raw;
#[cfg(feature = "std")]
pub mod requirements;

#[cfg(feature = "std")]
pub mod yaml;

#[cfg(feature = "std")]
pub use artifact::GeneratedArtifacts;
pub use error::ConfigError;
pub use hash::ConfigHash;
#[cfg(feature = "std")]
pub use intent::StaticIntentIR;
#[cfg(feature = "std")]
pub use normalize::NormalizedConfig;
#[cfg(feature = "std")]
pub use pipeline::{compile_config, validate_config};
#[cfg(feature = "std")]
pub use raw::RawConfig;
#[cfg(feature = "std")]
pub use requirements::PlatformRequirements;
