//! e1000 frame queue for host-assisted datapath injection and extraction.

/// Maximum stored frame size for Gate D MVP.
pub const MAX_FRAME_BYTES: usize = 2048;

/// Simple bounded queue of Ethernet frames for host datapath simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PacketQueue {
    frames: alloc::vec::Vec<[u8; MAX_FRAME_BYTES]>,
    lengths: alloc::vec::Vec<usize>,
}

impl PacketQueue {
    /// Creates an empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self { frames: alloc::vec::Vec::new(), lengths: alloc::vec::Vec::new() }
    }

    /// Enqueues one frame, dropping the oldest when at capacity.
    pub fn push(&mut self, frame: &[u8]) {
        let mut slot = [0u8; MAX_FRAME_BYTES];
        let len = frame.len().min(MAX_FRAME_BYTES);
        slot[..len].copy_from_slice(&frame[..len]);
        if self.frames.len() >= 64 {
            self.frames.remove(0);
            self.lengths.remove(0);
        }
        self.frames.push(slot);
        self.lengths.push(len);
    }

    /// Dequeues one frame into `out`, returning the number of valid bytes.
    pub fn pop(&mut self, out: &mut [u8]) -> Option<usize> {
        let slot = self.frames.first()?;
        let len = *self.lengths.first()?;
        let copy_len = len.min(out.len());
        out[..copy_len].copy_from_slice(&slot[..copy_len]);
        self.frames.remove(0);
        self.lengths.remove(0);
        Some(copy_len)
    }

    /// Returns the number of queued frames.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns true when the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

impl Default for PacketQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn queue_round_trip_preserves_payload() {
        let mut queue = PacketQueue::new();
        queue.push(b"frame-a");
        let mut out = [0u8; MAX_FRAME_BYTES];
        let len = queue.pop(&mut out).expect("pop");
        assert_eq!(&out[..len], b"frame-a");
    }
}
