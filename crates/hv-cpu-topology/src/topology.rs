//! Discovered CPU topology.

use alloc::vec::Vec;

use hv_types::{ApicId, LogicalCpuId, PhysicalCoreId};

/// One physical core and its logical processors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuCore {
    /// Physical core identifier.
    pub physical_core_id: PhysicalCoreId,
    /// APIC ID of the bootstrap logical processor on this core.
    pub apic_id: ApicId,
    /// Logical processors belonging to the core.
    pub logical_cpus: Vec<LogicalCpuId>,
}

/// Platform CPU topology discovered from ACPI/CPUID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuTopology {
    /// Physical cores in deterministic order.
    pub cores: Vec<CpuCore>,
}

impl CpuTopology {
    /// Returns the number of physical cores.
    #[must_use]
    pub fn physical_core_count(&self) -> u32 {
        self.cores.len() as u32
    }

    /// Returns true when any core has more than one logical processor.
    #[must_use]
    pub fn smt_enabled(&self) -> bool {
        self.cores.iter().any(|core| core.logical_cpus.len() > 1)
    }
}
