//! Intel e1000 NIC MMIO device model (Gate D skeleton).
//!
//! Provides deterministic MMIO decode for guest device BAR accesses. DMA and
//! interrupt delivery are handled through VT-d and future interrupt modules.
//!
//! # Proof levels
//!
//! | Property | Levels |
//! |----------|--------|
//! | MMIO register decode | UNIT |
//! | Unmapped offset rejection | UNIT + malicious tests |

#![no_std]
#![warn(missing_docs)]

mod error;
mod mmio;
mod regs;

pub use error::E1000Error;
pub use mmio::{mmio_read, mmio_write, E1000DeviceState};
pub use regs::{E1000_MMIO_SIZE, REG_CTRL, REG_STATUS, REG_TDH, REG_TDT};
