//! EPT mapping types.

use hv_types::{GuestPhysAddr, HostPhysAddr};

/// EPT page permissions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EptPermissions {
    /// Read allowed.
    pub read: bool,
    /// Write allowed.
    pub write: bool,
    /// Execute allowed.
    pub execute: bool,
}

impl EptPermissions {
    /// Guest RAM permissions: RW, no execute at EPT level (guest may still execute internally).
    pub const GUEST_RAM: Self = Self { read: true, write: true, execute: true };

    /// MMIO permissions: RW, no execute.
    pub const MMIO: Self = Self { read: true, write: true, execute: false };
}

/// EPT memory type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EptMemoryType {
    /// Write-back memory.
    WriteBack,
    /// Uncacheable MMIO.
    Uncacheable,
}

/// One EPT mapping entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EptMapping {
    /// Guest physical base.
    pub guest_phys: GuestPhysAddr,
    /// Host physical base.
    pub host_phys: HostPhysAddr,
    /// Mapping size in bytes.
    pub size: u64,
    /// Permissions.
    pub permissions: EptPermissions,
    /// Memory type.
    pub memory_type: EptMemoryType,
}
