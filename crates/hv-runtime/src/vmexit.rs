//! VM-exit handling skeleton for MMIO and EPT violations.

use hv_e1000::E1000Error;
use hv_types::GuestPhysAddr;

use crate::error::RuntimeError;
use crate::mmio::MmioDispatch;

/// Basic VM-exit reasons used by Gate D MMIO dispatch.
pub mod reason {
    /// Exception or NMI.
    pub const EXCEPTION_OR_NMI: u32 = 0;
    /// External interrupt.
    pub const EXTERNAL_INTERRUPT: u32 = 1;
    /// EPT violation (used for MMIO traps in the MVP model).
    pub const EPT_VIOLATION: u32 = 48;
}

/// Decoded MMIO exit information for one guest access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MmioExitInfo {
    /// Guest physical address of the access.
    pub guest_phys: GuestPhysAddr,
    /// Access size in bytes (1, 2, or 4).
    pub access_size: u8,
    /// True when the guest attempted a write.
    pub is_write: bool,
}

/// Result of handling one MMIO-related VM-exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MmioExitAction {
    /// MMIO read completed; advance guest RIP by `advance`.
    Read {
        /// Value returned to the guest.
        value: u32,
        /// Bytes to advance guest RIP.
        advance: u8,
    },
    /// MMIO write completed; advance guest RIP by `advance`.
    Write {
        /// Bytes to advance guest RIP.
        advance: u8,
    },
    /// Exit reason is not handled by this skeleton.
    Unhandled {
        /// Raw VM-exit reason.
        reason: u32,
    },
}

/// Handles one MMIO VM-exit against the runtime dispatcher.
///
/// # Errors
///
/// Returns [`RuntimeError::Mmio`] when the access targets an invalid device offset.
pub fn handle_mmio_exit(
    dispatch: &mut MmioDispatch,
    info: MmioExitInfo,
    write_value: u32,
) -> Result<MmioExitAction, RuntimeError> {
    if info.access_size != 4 {
        return Err(RuntimeError::Mmio(E1000Error::OffsetOutOfRange));
    }
    if info.is_write {
        dispatch.write32(info.guest_phys, 0, write_value)?;
        Ok(MmioExitAction::Write { advance: info.access_size })
    } else {
        let value = dispatch.read32(info.guest_phys, 0)?;
        Ok(MmioExitAction::Read { value, advance: info.access_size })
    }
}

/// Maps a raw VM-exit reason to a diagnostic action without dispatching MMIO.
#[must_use]
pub fn classify_exit_reason(reason: u32) -> MmioExitAction {
    MmioExitAction::Unhandled { reason }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use hv_ept::{EptMapping, EptMemoryType, EptPermissions};
    use hv_types::HostPhysAddr;

    use super::*;

    #[test]
    fn mmio_exit_read_returns_link_up_status() {
        let mappings = &[EptMapping {
            guest_phys: GuestPhysAddr::new(0xFEB0_0000),
            host_phys: HostPhysAddr::new(0x2300_0000),
            size: 128 * 1024,
            permissions: EptPermissions::MMIO,
            memory_type: EptMemoryType::Uncacheable,
        }];
        let mut dispatch = MmioDispatch::from_ept_mappings(mappings);
        let action = handle_mmio_exit(
            &mut dispatch,
            MmioExitInfo {
                guest_phys: GuestPhysAddr::new(0xFEB0_0000 + hv_e1000::REG_STATUS as u64),
                access_size: 4,
                is_write: false,
            },
            0,
        )
        .expect("handle");
        match action {
            MmioExitAction::Read { value, advance: 4 } => assert_ne!(value & 0x80, 0),
            other => panic!("unexpected action: {other:?}"),
        }
    }
}
