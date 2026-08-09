//! VM, vCPU, and platform identity newtypes.

use core::fmt;

/// Stable partition identifier assigned deterministically from configuration order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct VmId(pub u32);

impl VmId {
    /// Creates a VM identifier.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for VmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VmId({})", self.0)
    }
}

impl fmt::Display for VmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vm{}", self.0)
    }
}

/// Virtual CPU index within a partition.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct VcpuId(pub u32);

impl VcpuId {
    /// Creates a vCPU identifier.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for VcpuId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VcpuId({})", self.0)
    }
}

/// Logical processor identifier as reported by firmware or ACPI.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct LogicalCpuId(pub u32);

impl LogicalCpuId {
    /// Creates a logical CPU identifier.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for LogicalCpuId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LogicalCpuId({})", self.0)
    }
}

/// Physical core identifier within a package.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PhysicalCoreId(pub u32);

impl PhysicalCoreId {
    /// Creates a physical core identifier.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for PhysicalCoreId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysicalCoreId({})", self.0)
    }
}

/// CPU package (socket) identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PackageId(pub u32);

impl PackageId {
    /// Creates a package identifier.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackageId({})", self.0)
    }
}

/// IOMMU domain identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct IommuDomainId(pub u16);

impl IommuDomainId {
    /// Creates an IOMMU domain identifier.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for IommuDomainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IommuDomainId({})", self.0)
    }
}

/// APIC identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct ApicId(pub u32);

impl ApicId {
    /// Creates an APIC identifier.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw identifier value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for ApicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApicId({})", self.0)
    }
}

/// Interrupt vector number (0..=255).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InterruptVector(u8);

impl InterruptVector {
    /// Creates an interrupt vector.
    #[must_use]
    pub const fn new(vector: u8) -> Self {
        Self(vector)
    }

    /// Returns the raw vector number.
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

impl fmt::Debug for InterruptVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InterruptVector({})", self.0)
    }
}
