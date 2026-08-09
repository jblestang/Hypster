//! Configuration error taxonomy.

use alloc::string::String;
use core::fmt;

/// Errors produced while parsing or validating configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Unsupported configuration schema version.
    UnsupportedVersion {
        /// Version found in YAML.
        found: u32,
        /// Supported versions.
        supported: &'static [u32],
    },
    /// A required field is missing or empty.
    MissingField {
        /// Dotted path to the field.
        path: &'static str,
    },
    /// A field has an invalid value.
    InvalidValue {
        /// Dotted path to the field.
        path: &'static str,
        /// Human-readable reason.
        reason: String,
    },
    /// Duplicate identifier within a namespace.
    DuplicateId {
        /// Namespace, e.g. `partition` or `ipc`.
        namespace: &'static str,
        /// Duplicate name.
        name: String,
    },
    /// Referenced entity does not exist.
    UnknownReference {
        /// Reference kind.
        kind: &'static str,
        /// Referenced name.
        name: String,
    },
    /// Datapath topology violates unidirectional constraints.
    DatapathViolation {
        /// Explanation of the violation.
        reason: String,
    },
    /// PCI device ownership conflict.
    PciOwnershipConflict {
        /// Conflicting BDF string.
        bdf: String,
        /// First owner partition.
        first: String,
        /// Second owner partition.
        second: String,
    },
    /// Resource totals exceed declared platform minimums.
    ResourceBudgetExceeded {
        /// Resource kind.
        kind: &'static str,
        /// Required amount.
        required: u64,
        /// Available amount.
        available: u64,
    },
    /// YAML could not be parsed.
    #[cfg(feature = "std")]
    YamlParse(String),
    /// I/O error while reading configuration.
    #[cfg(feature = "std")]
    Io(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { found, supported } => {
                write!(f, "unsupported config version {found}; supported: {supported:?}")
            }
            Self::MissingField { path } => write!(f, "missing required field `{path}`"),
            Self::InvalidValue { path, reason } => {
                write!(f, "invalid value at `{path}`: {reason}")
            }
            Self::DuplicateId { namespace, name } => {
                write!(f, "duplicate {namespace} name `{name}`")
            }
            Self::UnknownReference { kind, name } => {
                write!(f, "unknown {kind} `{name}`")
            }
            Self::DatapathViolation { reason } => write!(f, "datapath violation: {reason}"),
            Self::PciOwnershipConflict { bdf, first, second } => {
                write!(f, "PCI device `{bdf}` assigned to both `{first}` and `{second}`")
            }
            Self::ResourceBudgetExceeded { kind, required, available } => {
                write!(f, "{kind} budget exceeded: required {required}, available {available}")
            }
            #[cfg(feature = "std")]
            Self::YamlParse(msg) => write!(f, "yaml parse error: {msg}"),
            #[cfg(feature = "std")]
            Self::Io(msg) => write!(f, "io error: {msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConfigError {}

impl ConfigError {
    /// Helper for invalid value errors.
    pub fn invalid(path: &'static str, reason: impl Into<String>) -> Self {
        Self::InvalidValue { path, reason: reason.into() }
    }
}
