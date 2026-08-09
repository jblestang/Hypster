//! QEMU validation fixtures for host-side Gate B tests.

use hv_acpi::table::SdtHeader;
use hv_acpi::{DmarSummary, DrhdEntry, MadtSummary, X2ApicEntry, AcpiPlatformSummary};
use hv_types::PciBdf;

use crate::observed::{ObservedCpuFeatures, ObservedPciDevice, ObservedPlatform};

fn literal_bdf(device: u8, function: u8) -> Result<PciBdf, &'static str> {
    PciBdf::from_components(0, 0, device, function).map_err(|_| "invalid fixture bdf")
}

/// Builds the Gate B QEMU validation observed platform fixture.
///
/// # Errors
///
/// Returns a static error label when fixture literals are malformed.
pub fn qemu_validation_observed() -> Result<ObservedPlatform, &'static str> {
    let acpi = AcpiPlatformSummary {
        dmar: Some(DmarSummary {
            header: SdtHeader {
                signature: *b"DMAR",
                length: 48,
                revision: 1,
            },
            host_address_width: 39,
            interrupt_remapping: true,
            drhd_entries: alloc::vec![DrhdEntry {
                segment: hv_types::PciSegment::new(0),
                register_base: hv_types::HostPhysAddr::new(0xFED9_0000),
                include_all_pci: true,
            }],
        }),
        madt: Some(MadtSummary {
            header: SdtHeader {
                signature: *b"APIC",
                length: 44,
                revision: 1,
            },
            local_apic_address: hv_types::HostPhysAddr::new(0xFEE0_0000),
            pc_at_compatible: false,
            local_apics: alloc::vec![],
            x2_apics: alloc::vec![X2ApicEntry {
                processor_uid: 0,
                apic_id: hv_types::ApicId::new(0),
                enabled: true,
            }],
        }),
        ..AcpiPlatformSummary::default()
    };

    let memory = [(
        7u32,
        hv_types::HostPhysAddr::new(0x1000_0000),
        6 * 1024 * 1024 * 1024u64,
    )];
    let pci = [
        ObservedPciDevice {
            bdf: literal_bdf(3, 0)?,
        },
        ObservedPciDevice {
            bdf: literal_bdf(4, 0)?,
        },
    ];

    ObservedPlatform::build(
        ObservedCpuFeatures::qemu_validation(),
        acpi,
        &memory,
        alloc::vec::Vec::from(pci),
        4,
    )
    .ok_or("invalid fixture platform")
}
