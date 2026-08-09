//! Observed platform capabilities.

use alloc::vec::Vec;

use hv_acpi::AcpiPlatformSummary;
use hv_config_model::normalize::ArchRequirement;
use hv_memory::map::{collect_conventional, ConventionalRegion};
use hv_types::{HostPhysAddr, PciBdf};

/// CPU/virtualization features observed on the platform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedCpuFeatures {
    /// VMX present.
    pub vmx: bool,
    /// EPT present.
    pub ept: bool,
    /// VT-d firmware tables present.
    pub vtd: bool,
    /// NX available.
    pub nx: bool,
    /// Invariant TSC advertised.
    pub invariant_tsc: bool,
    /// VPID supported.
    pub vpid: bool,
    /// VMX preemption timer supported.
    pub vmx_preemption_timer: bool,
}

impl ObservedCpuFeatures {
    /// Returns an all-capable profile for QEMU validation fixtures.
    #[must_use]
    pub const fn qemu_validation() -> Self {
        Self {
            vmx: true,
            ept: true,
            vtd: true,
            nx: true,
            invariant_tsc: true,
            vpid: true,
            vmx_preemption_timer: true,
        }
    }
}

/// PCI device discovered during enumeration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedPciDevice {
    /// Device BDF.
    pub bdf: PciBdf,
}

/// Hardware/firmware view of the platform at boot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedPlatform {
    /// Observed architecture.
    pub arch: ArchRequirement,
    /// CPU features.
    pub cpu: ObservedCpuFeatures,
    /// ACPI-derived information.
    pub acpi: AcpiPlatformSummary,
    /// Conventional memory regions.
    pub conventional_memory: Vec<ConventionalRegion>,
    /// Total conventional RAM in bytes.
    pub conventional_ram_bytes: u64,
    /// Discovered PCI devices.
    pub pci_devices: Vec<ObservedPciDevice>,
    /// Enabled processors reported by MADT.
    pub enabled_processors: u32,
    /// Physical core count used for planning.
    pub physical_core_count: u32,
}

impl ObservedPlatform {
    /// Builds an observed platform from firmware-derived inputs.
    ///
    /// # Errors
    ///
    /// Returns `None` when conventional memory totals overflow.
    pub fn build(
        cpu: ObservedCpuFeatures,
        acpi: AcpiPlatformSummary,
        memory_descriptors: &[(u32, HostPhysAddr, u64)],
        pci_devices: Vec<ObservedPciDevice>,
        physical_core_count: u32,
    ) -> Option<Self> {
        let conventional_memory = collect_conventional(memory_descriptors);
        let conventional_ram_bytes = hv_memory::map::total_conventional_bytes(&conventional_memory)?;
        let enabled_processors = acpi
            .enabled_processor_count()
            .map(|count| count as u32)
            .unwrap_or(physical_core_count);
        Some(Self {
            arch: ArchRequirement::X86_64,
            cpu,
            acpi,
            conventional_memory,
            conventional_ram_bytes,
            pci_devices,
            enabled_processors,
            physical_core_count,
        })
    }

    /// Returns true when interrupt remapping is available per ACPI DMAR.
    #[must_use]
    pub fn interrupt_remapping_available(&self) -> bool {
        self.acpi.interrupt_remapping_available()
    }

    /// Returns true when x2APIC is available per MADT.
    #[must_use]
    pub fn x2apic_available(&self) -> bool {
        self.acpi.x2apic_available()
    }
}
