//! Runtime initialization errors.

use core::fmt;

use hv_acpi::AcpiError;
use hv_boot_abi::BootLayoutError;
use hv_cpu::CpuProbeError;
use hv_ept::EptPlanError;
use hv_ipc::IpcError;
use hv_vmx::VmxError;
use hv_vtd::VtdPlanError;
use hv_x86::X86Error;

/// Aggregated runtime initialization failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    /// Boot info layout validation failed.
    BootLayout(BootLayoutError),
    /// CPU probing failed.
    Cpu(CpuProbeError),
    /// ACPI RSDP verification failed.
    Acpi(AcpiError),
    /// Physical memory read failed.
    Phys(X86Error),
    /// EPT table installation failed.
    Ept(EptPlanError),
    /// VT-d table installation failed.
    Vtd(VtdPlanError),
    /// VMX initialization failed.
    Vmx(VmxError),
    /// Table region is inaccessible.
    TableRegionUnavailable,
    /// Configuration hash mismatch when enforcement is enabled.
    ConfigHashMismatch,
    /// IPC ring initialization or validation failed.
    Ipc(IpcError),
}

impl RuntimeError {
    /// Returns a static diagnostic label.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BootLayout(_) => "boot layout error",
            Self::Cpu(_) => "cpu probe error",
            Self::Acpi(_) => "acpi error",
            Self::Phys(_) => "physical read error",
            Self::Ept(_) => "ept install error",
            Self::Vtd(_) => "vtd install error",
            Self::Vmx(_) => "vmx init error",
            Self::TableRegionUnavailable => "table region unavailable",
            Self::ConfigHashMismatch => "config hash mismatch",
            Self::Ipc(_) => "ipc error",
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<BootLayoutError> for RuntimeError {
    fn from(value: BootLayoutError) -> Self {
        Self::BootLayout(value)
    }
}

impl From<CpuProbeError> for RuntimeError {
    fn from(value: CpuProbeError) -> Self {
        Self::Cpu(value)
    }
}

impl From<AcpiError> for RuntimeError {
    fn from(value: AcpiError) -> Self {
        Self::Acpi(value)
    }
}

impl From<X86Error> for RuntimeError {
    fn from(value: X86Error) -> Self {
        Self::Phys(value)
    }
}

impl From<EptPlanError> for RuntimeError {
    fn from(value: EptPlanError) -> Self {
        Self::Ept(value)
    }
}

impl From<VtdPlanError> for RuntimeError {
    fn from(value: VtdPlanError) -> Self {
        Self::Vtd(value)
    }
}

impl From<VmxError> for RuntimeError {
    fn from(value: VmxError) -> Self {
        Self::Vmx(value)
    }
}

impl From<IpcError> for RuntimeError {
    fn from(value: IpcError) -> Self {
        Self::Ipc(value)
    }
}
