//! IPC ring errors.

use core::fmt;

/// Errors operating on an IPC ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcError {
    /// Shared memory is smaller than the fixed header.
    BufferTooSmall,
    /// Header magic or ABI version mismatch.
    InvalidHeader,
    /// Producer or consumer index exceeds `slot_count`.
    IndexOutOfRange,
    /// Header fields disagree with caller-provided channel parameters.
    ParameterMismatch,
    /// Ring state indicates corruption (policy: stop partition).
    CorruptionDetected,
    /// No free slot is available for a push.
    QueueFull,
    /// No slot is available for a pop.
    QueueEmpty,
    /// Slot payload would exceed the slot buffer.
    SlotTooSmall,
    /// Integer overflow computing layout sizes.
    Overflow,
}

impl IpcError {
    /// Returns a static diagnostic label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BufferTooSmall => "ipc buffer too small",
            Self::InvalidHeader => "ipc invalid header",
            Self::IndexOutOfRange => "ipc index out of range",
            Self::ParameterMismatch => "ipc parameter mismatch",
            Self::CorruptionDetected => "ipc corruption detected",
            Self::QueueFull => "ipc queue full",
            Self::QueueEmpty => "ipc queue empty",
            Self::SlotTooSmall => "ipc slot too small",
            Self::Overflow => "ipc overflow",
        }
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
