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
pub mod intent;
pub mod normalize;
pub mod pipeline;
pub mod raw;
pub mod requirements;

#[cfg(feature = "std")]
pub mod yaml;

#[cfg(feature = "std")]
pub use artifact::GeneratedArtifacts;
pub use error::ConfigError;
pub use hash::ConfigHash;
pub use intent::StaticIntentIR;
pub use normalize::NormalizedConfig;
pub use pipeline::{compile_config, validate_config};
pub use raw::RawConfig;
pub use requirements::PlatformRequirements;
