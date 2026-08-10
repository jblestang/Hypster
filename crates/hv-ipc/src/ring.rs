//! IPC ring layout and SPSC operations.

use core::mem::{align_of, size_of};

use hv_types::arithmetic::{align_up_u64, checked_add_u64};

use crate::IpcError;

/// Magic signature for IPC ring headers.
pub const IPC_RING_MAGIC: u64 = 0x4950_4352_494E_4748;

/// Fixed header size in bytes.
pub const IPC_RING_HEADER_BYTES: usize = size_of::<IpcRingHeader>();

/// Role flag: this partition is the producer.
pub const ROLE_PRODUCER: u32 = 1 << 0;
/// Role flag: this partition is the consumer.
pub const ROLE_CONSUMER: u32 = 1 << 1;

/// Ring control block at the base of shared IPC memory.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IpcRingHeader {
    /// Magic signature [`IPC_RING_MAGIC`].
    pub magic: u64,
    /// Stable channel name hash.
    pub name_hash: u64,
    /// Number of payload slots following the header.
    pub slot_count: u32,
    /// Maximum payload bytes per slot (excluding slot metadata).
    pub slot_size: u32,
    /// Producer write index in `[0, slot_count)`.
    pub head: u32,
    /// Consumer read index in `[0, slot_count)`.
    pub tail: u32,
    /// Monotonic generation bumped on init for stale-view detection.
    pub generation: u32,
    /// Reserved.
    pub reserved: u32,
}

/// Computes deterministic FNV-1a hash for a channel name.
#[must_use]
pub fn ipc_name_hash(name: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Returns total shared bytes for a channel ring.
///
/// # Errors
///
/// Returns [`IpcError::Overflow`] when the layout size overflows.
pub fn compute_shared_bytes(slot_count: u32, slot_size: u32) -> Result<u64, IpcError> {
    let slots = u64::from(slot_count);
    let slot_bytes = u64::from(slot_size);
    let payload = slots.checked_mul(slot_bytes).ok_or(IpcError::Overflow)?;
    checked_add_u64(IPC_RING_HEADER_BYTES as u64, payload).map_err(|_| IpcError::Overflow)
}

/// Page size used when placing IPC rings in guest/host physical maps.
pub const IPC_MAPPING_PAGE_SIZE: u64 = 4096;

/// Returns the page-rounded footprint for an IPC ring mapping.
///
/// Logical ring bytes from [`compute_shared_bytes`] may include a header that
/// is not page-aligned; EPT/GPA placement must advance by this rounded size so
/// consecutive channels stay page-aligned.
///
/// # Errors
///
/// Returns [`IpcError::Overflow`] when rounding overflows.
pub fn ipc_mapping_bytes(shared_bytes: u64) -> Result<u64, IpcError> {
    align_up_u64(shared_bytes, IPC_MAPPING_PAGE_SIZE).map_err(|_| IpcError::Overflow)
}

/// Initializes a ring in freshly zeroed shared memory.
///
/// # Errors
///
/// Returns [`IpcError`] when the buffer is too small or parameters overflow.
pub fn init_ring<'a>(
    backing: &'a mut [u8],
    name: &str,
    slot_count: u32,
    slot_size: u32,
) -> Result<&'a mut IpcRingHeader, IpcError> {
    let required = compute_shared_bytes(slot_count, slot_size)?;
    if backing.len() < required as usize {
        return Err(IpcError::BufferTooSmall);
    }
    if backing.len() < IPC_RING_HEADER_BYTES {
        return Err(IpcError::BufferTooSmall);
    }
    if align_of::<IpcRingHeader>() > 1
        && (backing.as_ptr() as usize) % align_of::<IpcRingHeader>() != 0
    {
        return Err(IpcError::InvalidHeader);
    }

    let header = header_mut(backing)?;
    header.magic = IPC_RING_MAGIC;
    header.name_hash = ipc_name_hash(name);
    header.slot_count = slot_count;
    header.slot_size = slot_size;
    header.head = 0;
    header.tail = 0;
    header.generation = header.generation.wrapping_add(1);
    header.reserved = 0;
    Ok(header)
}

/// Validates a ring header and backing size against expected channel parameters.
///
/// # Errors
///
/// Returns [`IpcError::CorruptionDetected`] or related errors when invariants fail.
pub fn validate_ring<'a>(
    backing: &'a [u8],
    name: &str,
    slot_count: u32,
    slot_size: u32,
) -> Result<&'a IpcRingHeader, IpcError> {
    let required = compute_shared_bytes(slot_count, slot_size)?;
    if backing.len() < required as usize {
        return Err(IpcError::BufferTooSmall);
    }
    let header = header_ref(backing)?;
    if header.magic != IPC_RING_MAGIC {
        return Err(IpcError::InvalidHeader);
    }
    if header.name_hash != ipc_name_hash(name) {
        return Err(IpcError::ParameterMismatch);
    }
    if header.slot_count != slot_count || header.slot_size != slot_size {
        return Err(IpcError::ParameterMismatch);
    }
    if header.head >= header.slot_count || header.tail >= header.slot_count {
        return Err(IpcError::CorruptionDetected);
    }
    Ok(header)
}

/// Attempts to push one payload slot as producer.
///
/// # Errors
///
/// Returns [`IpcError::QueueFull`] when no slot is available.
pub fn try_push(
    backing: &mut [u8],
    name: &str,
    slot_count: u32,
    slot_size: u32,
    payload: &[u8],
) -> Result<(), IpcError> {
    if payload.len() > slot_size as usize {
        return Err(IpcError::SlotTooSmall);
    }
    validate_ring(backing, name, slot_count, slot_size)?;
    let (head, slot_size, slot_count, tail) = {
        let header = header_ref(backing)?;
        (header.head, header.slot_size, header.slot_count, header.tail)
    };
    let next = (head + 1) % slot_count;
    if next == tail {
        return Err(IpcError::QueueFull);
    }
    let slot_offset = slot_byte_offset(head, slot_size)?;
    let slot = slot_mut(backing, slot_offset, slot_size as usize)?;
    slot.fill(0);
    slot[..payload.len()].copy_from_slice(payload);
    header_mut(backing)?.head = next;
    Ok(())
}

/// Attempts to pop one payload slot as consumer.
///
/// Returns the number of valid payload bytes written to `out`.
///
/// # Errors
///
/// Returns [`IpcError::QueueEmpty`] when no slot is available.
pub fn try_pop(
    backing: &mut [u8],
    name: &str,
    slot_count: u32,
    slot_size: u32,
    out: &mut [u8],
) -> Result<usize, IpcError> {
    validate_ring(backing, name, slot_count, slot_size)?;
    let (head, tail, slot_size, slot_count) = {
        let header = header_ref(backing)?;
        (header.head, header.tail, header.slot_size, header.slot_count)
    };
    if head == tail {
        return Err(IpcError::QueueEmpty);
    }
    let slot_offset = slot_byte_offset(tail, slot_size)?;
    let slot = slot_ref(backing, slot_offset, slot_size as usize)?;
    let len = core::cmp::min(out.len(), slot.len());
    out[..len].copy_from_slice(&slot[..len]);
    header_mut(backing)?.tail = (tail + 1) % slot_count;
    Ok(len)
}

/// Reads the next consumer slot without advancing `tail`.
///
/// # Errors
///
/// Returns [`IpcError::QueueEmpty`] when no slot is available.
pub fn peek_front_payload(
    backing: &[u8],
    name: &str,
    slot_count: u32,
    slot_size: u32,
    out: &mut [u8],
) -> Result<usize, IpcError> {
    validate_ring(backing, name, slot_count, slot_size)?;
    let (head, tail, slot_size, _slot_count) = {
        let header = header_ref(backing)?;
        (header.head, header.tail, header.slot_size, header.slot_count)
    };
    if head == tail {
        return Err(IpcError::QueueEmpty);
    }
    let slot_offset = slot_byte_offset(tail, slot_size)?;
    let slot = slot_ref(backing, slot_offset, slot_size as usize)?;
    let len = core::cmp::min(out.len(), slot.len());
    out[..len].copy_from_slice(&slot[..len]);
    Ok(len)
}

/// Returns true when the ring has at least one unconsumed slot.
///
/// # Errors
///
/// Returns [`IpcError`] when header validation fails.
pub fn ring_has_pending_frame(
    backing: &[u8],
    name: &str,
    slot_count: u32,
    slot_size: u32,
) -> Result<bool, IpcError> {
    let header = validate_ring(backing, name, slot_count, slot_size)?;
    Ok(header.head != header.tail)
}

fn header_ref(backing: &[u8]) -> Result<&IpcRingHeader, IpcError> {
    if backing.len() < IPC_RING_HEADER_BYTES {
        return Err(IpcError::BufferTooSmall);
    }
    // SAFETY: header is repr(C) at offset 0 with sufficient size.
    Ok(unsafe { &*(backing.as_ptr() as *const IpcRingHeader) })
}

fn header_mut(backing: &mut [u8]) -> Result<&mut IpcRingHeader, IpcError> {
    if backing.len() < IPC_RING_HEADER_BYTES {
        return Err(IpcError::BufferTooSmall);
    }
    // SAFETY: header is repr(C) at offset 0 with sufficient size.
    Ok(unsafe { &mut *(backing.as_mut_ptr() as *mut IpcRingHeader) })
}

fn slot_byte_offset(index: u32, slot_size: u32) -> Result<usize, IpcError> {
    let index_u64 = u64::from(index);
    let slot_u64 = u64::from(slot_size);
    let payload_offset = index_u64.checked_mul(slot_u64).ok_or(IpcError::Overflow)?;
    checked_add_u64(IPC_RING_HEADER_BYTES as u64, payload_offset)
        .map(|value| value as usize)
        .map_err(|_| IpcError::Overflow)
}

fn slot_ref(backing: &[u8], offset: usize, slot_size: usize) -> Result<&[u8], IpcError> {
    let end = offset.checked_add(slot_size).ok_or(IpcError::Overflow)?;
    if end > backing.len() {
        return Err(IpcError::BufferTooSmall);
    }
    Ok(&backing[offset..end])
}

fn slot_mut(backing: &mut [u8], offset: usize, slot_size: usize) -> Result<&mut [u8], IpcError> {
    let end = offset.checked_add(slot_size).ok_or(IpcError::Overflow)?;
    if end > backing.len() {
        return Err(IpcError::BufferTooSmall);
    }
    Ok(&mut backing[offset..end])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    extern crate alloc;

    use alloc::vec;

    use super::*;

    #[test]
    fn ring_header_layout() {
        assert_eq!(size_of::<IpcRingHeader>(), 40);
        assert_eq!(align_of::<IpcRingHeader>(), 8);
    }

    #[test]
    fn shared_bytes_matches_config_intent() {
        let bytes = compute_shared_bytes(256, 2048).expect("shared bytes");
        assert_eq!(bytes, IPC_RING_HEADER_BYTES as u64 + 256 * 2048);
    }

    #[test]
    fn ipc_mapping_bytes_page_aligns_ring_header() {
        let bytes = compute_shared_bytes(256, 2048).expect("shared bytes");
        assert_eq!(bytes % IPC_MAPPING_PAGE_SIZE, 40);
        let mapped = ipc_mapping_bytes(bytes).expect("mapping bytes");
        assert_eq!(mapped % IPC_MAPPING_PAGE_SIZE, 0);
        assert!(mapped >= bytes);
    }

    #[test]
    fn peek_front_payload_does_not_advance_tail() {
        let mut backing = vec![0u8; compute_shared_bytes(4, 16).expect("bytes") as usize];
        init_ring(&mut backing, "mid_to_out", 4, 16).expect("init");
        try_push(&mut backing, "mid_to_out", 4, 16, b"peek-me").expect("push");
        let mut out = [0u8; 16];
        let len = peek_front_payload(&backing, "mid_to_out", 4, 16, &mut out).expect("peek");
        assert_eq!(len, 16);
        assert_eq!(&out[..7], b"peek-me");
        let popped = try_pop(&mut backing, "mid_to_out", 4, 16, &mut out).expect("pop");
        assert_eq!(popped, 16);
        assert_eq!(&out[..7], b"peek-me");
    }

    #[test]
    fn push_pop_round_trip() {
        let mut backing = vec![0u8; compute_shared_bytes(4, 16).expect("bytes") as usize];
        init_ring(&mut backing, "in_to_mid", 4, 16).expect("init");
        try_push(&mut backing, "in_to_mid", 4, 16, b"hello").expect("push");
        let mut out = [0u8; 16];
        let len = try_pop(&mut backing, "in_to_mid", 4, 16, &mut out).expect("pop");
        assert_eq!(len, 16);
        assert_eq!(&out[..5], b"hello");
    }

    #[test]
    fn corruption_detected_on_bad_head() {
        let mut backing = vec![0u8; compute_shared_bytes(4, 16).expect("bytes") as usize];
        let header = init_ring(&mut backing, "in_to_mid", 4, 16).expect("init");
        header.head = 99;
        assert!(matches!(
            validate_ring(&backing, "in_to_mid", 4, 16),
            Err(IpcError::CorruptionDetected)
        ));
    }

    #[test]
    fn corruption_detected_on_magic_mismatch() {
        let mut backing = vec![0u8; compute_shared_bytes(4, 16).expect("bytes") as usize];
        init_ring(&mut backing, "in_to_mid", 4, 16).expect("init");
        backing[0] = 0;
        assert!(matches!(
            validate_ring(&backing, "in_to_mid", 4, 16),
            Err(IpcError::InvalidHeader)
        ));
    }

    #[test]
    fn queue_full_when_one_slot_free_reserved() {
        let mut backing = vec![0u8; compute_shared_bytes(2, 8).expect("bytes") as usize];
        init_ring(&mut backing, "mid_to_out", 2, 8).expect("init");
        try_push(&mut backing, "mid_to_out", 2, 8, b"a").expect("push1");
        assert!(matches!(
            try_push(&mut backing, "mid_to_out", 2, 8, b"b"),
            Err(IpcError::QueueFull)
        ));
    }
}
