//! Configuration compilation pipeline.

use crate::error::ConfigError;
use crate::hash::ConfigHash;
use crate::intent::StaticIntentIR;
use crate::normalize::{normalize, NormalizedConfig};
use crate::raw::RawConfig;
use crate::requirements::PlatformRequirements;

/// Output of successful configuration validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedConfig {
    /// Normalized configuration.
    pub normalized: NormalizedConfig,
    /// Platform requirements contract.
    pub platform_requirements: PlatformRequirements,
    /// Canonical configuration hash.
    pub config_hash: ConfigHash,
}

/// Output of full configuration compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledConfig {
    /// Validated configuration layers.
    pub validated: ValidatedConfig,
    /// Static intent IR.
    pub intent: StaticIntentIR,
}

/// Validates raw configuration through normalization and requirement extraction.
pub fn validate_config(raw: RawConfig) -> Result<ValidatedConfig, ConfigError> {
    let normalized = normalize(raw)?;
    let platform_requirements = PlatformRequirements::from_normalized(&normalized);
    let config_hash = compute_config_hash(&normalized);
    Ok(ValidatedConfig {
        normalized,
        platform_requirements,
        config_hash,
    })
}

/// Compiles raw configuration into validated layers and static intent IR.
pub fn compile_config(raw: RawConfig) -> Result<CompiledConfig, ConfigError> {
    let validated = validate_config(raw)?;
    let intent = StaticIntentIR::from_normalized(&validated.normalized, validated.config_hash)?;
    Ok(CompiledConfig { validated, intent })
}

#[cfg(feature = "std")]
fn compute_config_hash(normalized: &NormalizedConfig) -> ConfigHash {
    let canonical = crate::yaml::canonicalize_normalized(normalized);
    ConfigHash::compute(canonical.as_bytes())
}

#[cfg(not(feature = "std"))]
fn compute_config_hash(_normalized: &NormalizedConfig) -> ConfigHash {
    ConfigHash([0u8; 32])
}
