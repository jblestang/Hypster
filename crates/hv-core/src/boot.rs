//! Boot phase state machine.

use core::fmt;

/// Boot lifecycle phases for loader and hypervisor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootPhase {
    /// UEFI firmware active.
    Firmware,
    /// Loader running with boot services available.
    Loader,
    /// Boot services have been exited.
    BootServicesExited,
    /// Memory map captured and validated.
    MemoryReady,
    /// CPU features and topology validated.
    CpuReady,
    /// VMX prerequisites validated (hardware programming later).
    VmxReady,
    /// VT-d prerequisites validated.
    IommuReady,
    /// Interrupt remapping prerequisites validated.
    InterruptsReady,
    /// Partition descriptors prepared.
    PartitionsPrepared,
    /// Hypervisor running partitions.
    Running,
    /// Unrecoverable boot failure.
    Failed,
}

impl BootPhase {
    /// Returns allowed next phases.
    #[must_use]
    pub const fn allowed_successors(self) -> &'static [Self] {
        match self {
            Self::Firmware => &[Self::Loader, Self::Failed],
            Self::Loader => &[Self::BootServicesExited, Self::Failed],
            Self::BootServicesExited => &[Self::MemoryReady, Self::Failed],
            Self::MemoryReady => &[Self::CpuReady, Self::Failed],
            Self::CpuReady => &[Self::VmxReady, Self::Failed],
            Self::VmxReady => &[Self::IommuReady, Self::Failed],
            Self::IommuReady => &[Self::InterruptsReady, Self::Failed],
            Self::InterruptsReady => &[Self::PartitionsPrepared, Self::Failed],
            Self::PartitionsPrepared => &[Self::Running, Self::Failed],
            Self::Running => &[Self::Failed],
            Self::Failed => &[],
        }
    }

    /// Attempts to transition to `next`.
    ///
    /// # Errors
    ///
    /// Returns [`BootTransitionError`] when the transition is not allowed.
    pub const fn transition(self, next: Self) -> Result<Self, BootTransitionError> {
        let allowed = self.allowed_successors();
        let mut idx = 0;
        while idx < allowed.len() {
            if allowed[idx] as u8 == next as u8 {
                return Ok(next);
            }
            idx += 1;
        }
        Err(BootTransitionError { from: self, to: next })
    }
}

/// Invalid boot phase transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootTransitionError {
    /// Current phase.
    pub from: BootPhase,
    /// Requested next phase.
    pub to: BootPhase,
}

impl fmt::Display for BootTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid boot transition {:?} -> {:?}", self.from, self.to)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn happy_path_boot_transitions() {
        let mut phase = BootPhase::Firmware;
        for next in [
            BootPhase::Loader,
            BootPhase::BootServicesExited,
            BootPhase::MemoryReady,
            BootPhase::CpuReady,
            BootPhase::VmxReady,
            BootPhase::IommuReady,
            BootPhase::InterruptsReady,
            BootPhase::PartitionsPrepared,
            BootPhase::Running,
        ] {
            phase = phase.transition(next).expect("valid transition");
        }
    }

    #[test]
    fn invalid_transition_is_rejected() {
        assert!(BootPhase::Firmware
            .transition(BootPhase::Running)
            .is_err());
    }
}
