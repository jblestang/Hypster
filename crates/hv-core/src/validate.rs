//! Platform contract validation.

use alloc::string::String;
use core::fmt;

use hv_config_model::raw::RequirementLevel;
use hv_config_model::requirements::PlatformRequirements;
use hv_types::PciBdf;

use crate::observed::ObservedPlatform;
use crate::validated::ValidatedPlatform;

/// Platform validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlatformValidationError {
    /// Required architecture mismatch.
    ArchMismatch,
    /// Required feature missing (fail-closed).
    MissingRequiredFeature {
        /// Feature name.
        feature: &'static str,
    },
    /// Not enough physical cores.
    InsufficientPhysicalCores {
        /// Required cores.
        required: u32,
        /// Observed cores.
        observed: u32,
    },
    /// Host RAM below configured minimum.
    InsufficientRam {
        /// Required bytes.
        required: u64,
        /// Observed bytes.
        observed: u64,
    },
    /// Expected PCI device not discovered.
    MissingPciDevice {
        /// BDF string.
        bdf: String,
    },
    /// Required guest page size unsupported.
    MissingPageSize {
        /// Page size in bytes.
        size: u64,
    },
}

impl fmt::Display for PlatformValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchMismatch => f.write_str("architecture mismatch"),
            Self::MissingRequiredFeature { feature } => {
                write!(f, "missing required feature `{feature}`")
            }
            Self::InsufficientPhysicalCores { required, observed } => {
                write!(f, "insufficient physical cores: need {required}, saw {observed}")
            }
            Self::InsufficientRam { required, observed } => {
                write!(f, "insufficient RAM: need {required}, saw {observed}")
            }
            Self::MissingPciDevice { bdf } => write!(f, "missing PCI device `{bdf}`"),
            Self::MissingPageSize { size } => write!(f, "missing guest page size {size}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PlatformValidationError {}

/// Compares desired platform requirements against observed hardware.
pub fn validate_platform(
    requirements: &PlatformRequirements,
    observed: &ObservedPlatform,
) -> Result<ValidatedPlatform, PlatformValidationError> {
    if observed.arch != requirements.arch {
        return Err(PlatformValidationError::ArchMismatch);
    }

    check_feature(
        "vmx",
        requirements.vmx,
        observed.cpu.vmx,
    )?;
    check_feature(
        "ept",
        requirements.ept,
        observed.cpu.ept,
    )?;
    check_feature(
        "vtd",
        requirements.vtd,
        observed.cpu.vtd && observed.acpi.dmar.is_some(),
    )?;
    check_feature("nx", requirements.nx, observed.cpu.nx)?;
    check_feature(
        "interrupt_remapping",
        requirements.interrupt_remapping,
        observed.interrupt_remapping_available(),
    )?;
    check_feature(
        "x2apic",
        requirements.x2apic,
        observed.x2apic_available(),
    )?;
    check_feature(
        "invariant_tsc",
        requirements.invariant_tsc,
        observed.cpu.invariant_tsc,
    )?;
    check_feature("vpid", requirements.vpid, observed.cpu.vpid)?;
    check_feature(
        "vmx_preemption_timer",
        requirements.vmx_preemption_timer,
        observed.cpu.vmx_preemption_timer,
    )?;

    if observed.physical_core_count < requirements.min_physical_cores {
        return Err(PlatformValidationError::InsufficientPhysicalCores {
            required: requirements.min_physical_cores,
            observed: observed.physical_core_count,
        });
    }

    if observed.conventional_ram_bytes < requirements.min_ram_bytes {
        return Err(PlatformValidationError::InsufficientRam {
            required: requirements.min_ram_bytes,
            observed: observed.conventional_ram_bytes,
        });
    }

    for expected in &requirements.expected_pci_devices {
        let parsed = PciBdf::parse(&expected.bdf).map_err(|_| PlatformValidationError::MissingPciDevice {
            bdf: expected.bdf.clone(),
        })?;
        if !observed
            .pci_devices
            .iter()
            .any(|device| device.bdf == parsed)
        {
            return Err(PlatformValidationError::MissingPciDevice {
                bdf: expected.bdf.clone(),
            });
        }
    }

    for size in &requirements.page_sizes {
        if *size != 4096 && *size != 2_097_152 {
            return Err(PlatformValidationError::MissingPageSize { size: *size });
        }
    }

    Ok(ValidatedPlatform {
        config_name: requirements.config_name.clone(),
        observed: observed.clone(),
    })
}

fn check_feature(
    name: &'static str,
    level: RequirementLevel,
    present: bool,
) -> Result<(), PlatformValidationError> {
    if PlatformRequirements::is_fail_closed(level) && !present {
        return Err(PlatformValidationError::MissingRequiredFeature { feature: name });
    }
    Ok(())
}
