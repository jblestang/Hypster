//! Single-producer single-consumer IPC ring buffers.
//!
//! Rings are backed by preallocated shared memory described in validated
//! configuration. Producers and consumers validate header invariants before
//! every access and treat corruption as a fatal channel fault.
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | Header layout | UNIT |
//! | Corruption detection | UNIT + malicious tests |
//! | SPSC slot accounting | UNIT + PROPERTY |

#![no_std]
#![warn(missing_docs)]
#![allow(unsafe_code)] // Shared memory rings require raw byte views.

mod error;
mod ring;

pub use error::IpcError;
pub use ring::{
    compute_shared_bytes, init_ring, ipc_mapping_bytes, ipc_name_hash, peek_front_payload,
    ring_has_pending_frame, try_pop, try_push, validate_ring, IpcRingHeader, IPC_MAPPING_PAGE_SIZE,
    IPC_RING_HEADER_BYTES, IPC_RING_MAGIC, ROLE_CONSUMER, ROLE_PRODUCER,
};
