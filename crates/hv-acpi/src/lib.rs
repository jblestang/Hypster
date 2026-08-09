//! Strict minimal ACPI table parsers for the Hypster hypervisor.
//!
//! This crate parses a fixed set of ACPI tables needed to construct an
//! [`AcpiPlatformSummary`] for runtime platform observation. It does not
//! implement an AML interpreter.
//!
//! Supported tables:
//!
//! - RSDP
//! - RSDT / XSDT
//! - MADT (Local APIC and x2APIC entries)
//! - DMAR (DRHD entries and interrupt remapping flag)
//! - MCFG
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | Table parsing | UNIT |
//! | Checksum validation | UNIT |
//! | Summary extraction | UNIT |

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(test, allow(clippy::expect_used, clippy::slow_vector_initialization))]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod dmar;
pub mod error;
pub mod madt;
pub mod mcfg;
pub mod rsdp;
pub mod sdt;
pub mod table;

pub use dmar::{parse_dmar, DmarSummary, DrhdEntry};
pub use error::AcpiError;
pub use madt::{parse_madt, LocalApicEntry, MadtSummary, X2ApicEntry};
pub use mcfg::{parse_mcfg, McfgEntry, McfgSummary};
pub use rsdp::{parse_rsdp, Rsdp, RSDP_SIGNATURE};
pub use sdt::{parse_rsdt, parse_xsdt, Rsdt, Xsdt, RSDT_SIGNATURE, XSDT_SIGNATURE};
pub use table::SdtHeader;

/// Aggregated ACPI observations used when constructing `ObservedPlatform`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AcpiPlatformSummary {
    /// Parsed RSDP when available.
    pub rsdp: Option<Rsdp>,
    /// Parsed XSDT when discovered.
    pub xsdt: Option<Xsdt>,
    /// Parsed RSDT when discovered (typically when XSDT is absent).
    pub rsdt: Option<Rsdt>,
    /// Parsed MADT summary when discovered.
    pub madt: Option<MadtSummary>,
    /// Parsed DMAR summary when discovered.
    pub dmar: Option<DmarSummary>,
    /// Parsed MCFG summary when discovered.
    pub mcfg: Option<McfgSummary>,
}

impl AcpiPlatformSummary {
    /// Creates an empty summary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns physical addresses of root SDT entries (XSDT preferred over RSDT).
    #[must_use]
    pub fn root_table_pointers(&self) -> alloc::vec::Vec<hv_types::HostPhysAddr> {
        if let Some(xsdt) = &self.xsdt {
            xsdt.entries.clone()
        } else if let Some(rsdt) = &self.rsdt {
            rsdt.entries.clone()
        } else {
            alloc::vec::Vec::new()
        }
    }

    /// Returns true when DMAR advertises interrupt remapping.
    #[must_use]
    pub fn interrupt_remapping_available(&self) -> bool {
        self.dmar
            .as_ref()
            .is_some_and(|dmar| dmar.interrupt_remapping)
    }

    /// Returns true when MADT contains x2APIC entries.
    #[must_use]
    pub fn x2apic_available(&self) -> bool {
        self.madt.as_ref().is_some_and(MadtSummary::has_x2apic)
    }

    /// Returns the enabled processor count from MADT when present.
    #[must_use]
    pub fn enabled_processor_count(&self) -> Option<usize> {
        self.madt.as_ref().map(MadtSummary::enabled_processor_count)
    }
}
