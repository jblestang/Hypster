//! e1000 MMIO read/write decode.

use crate::error::E1000Error;
use crate::regs::{E1000_MMIO_SIZE, REG_CTRL, REG_STATUS, REG_TDH, REG_TDT};

/// Runtime state for one emulated e1000 instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E1000DeviceState {
    /// `REG_CTRL` value.
    pub ctrl: u32,
    /// `REG_STATUS` value.
    pub status: u32,
    /// `REG_TDH` value.
    pub tdh: u32,
    /// `REG_TDT` value.
    pub tdt: u32,
    /// Host-side RX queue for injected frames.
    pub rx_queue: crate::packet::PacketQueue,
    /// Host-side TX queue for extracted frames.
    pub tx_queue: crate::packet::PacketQueue,
}

impl E1000DeviceState {
    /// Returns powered-link-up status suitable for guest driver probe.
    #[must_use]
    pub fn new_link_up() -> Self {
        Self {
            ctrl: 0,
            status: 0x0000_0080,
            tdh: 0,
            tdt: 0,
            rx_queue: crate::packet::PacketQueue::new(),
            tx_queue: crate::packet::PacketQueue::new(),
        }
    }

    /// Injects one frame into the device RX queue.
    pub fn inject_rx_frame(&mut self, frame: &[u8]) {
        self.rx_queue.push(frame);
    }

    /// Takes one received frame from the device RX queue.
    pub fn take_rx_frame(&mut self, out: &mut [u8]) -> Option<usize> {
        self.rx_queue.pop(out)
    }

    /// Enqueues one frame on the device TX queue.
    pub fn enqueue_tx_frame(&mut self, frame: &[u8]) {
        self.tx_queue.push(frame);
    }

    /// Takes one transmitted frame from the device TX queue.
    pub fn take_tx_frame(&mut self, out: &mut [u8]) -> Option<usize> {
        self.tx_queue.pop(out)
    }
}

/// Reads a 32-bit MMIO register.
///
/// # Errors
///
/// Returns [`E1000Error::OffsetOutOfRange`] when `offset` exceeds the BAR size.
pub fn mmio_read(state: &E1000DeviceState, offset: u32) -> Result<u32, E1000Error> {
    if offset >= E1000_MMIO_SIZE {
        return Err(E1000Error::OffsetOutOfRange);
    }
    let value = match offset {
        REG_CTRL => state.ctrl,
        REG_STATUS => state.status,
        REG_TDH => state.tdh,
        REG_TDT => state.tdt,
        _ => 0,
    };
    Ok(value)
}

/// Writes a 32-bit MMIO register.
///
/// # Errors
///
/// Returns [`E1000Error`] when the offset is invalid or the register is read-only.
pub fn mmio_write(state: &mut E1000DeviceState, offset: u32, value: u32) -> Result<(), E1000Error> {
    if offset >= E1000_MMIO_SIZE {
        return Err(E1000Error::OffsetOutOfRange);
    }
    match offset {
        REG_CTRL => state.ctrl = value,
        REG_STATUS => return Err(E1000Error::ReadOnlyRegister),
        REG_TDH => state.tdh = value,
        REG_TDT => state.tdt = value,
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn status_is_readable_and_link_up_by_default() {
        let state = E1000DeviceState::new_link_up();
        let status = mmio_read(&state, REG_STATUS).expect("status");
        assert_eq!(status & 0x80, 0x80);
    }

    #[test]
    fn status_write_is_rejected() {
        let mut state = E1000DeviceState::new_link_up();
        assert!(matches!(mmio_write(&mut state, REG_STATUS, 0), Err(E1000Error::ReadOnlyRegister)));
    }

    #[test]
    fn out_of_range_access_is_rejected() {
        let state = E1000DeviceState::new_link_up();
        assert!(matches!(mmio_read(&state, E1000_MMIO_SIZE), Err(E1000Error::OffsetOutOfRange)));
    }
}
