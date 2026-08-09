//! e1000 register offsets.

/// Mapped MMIO BAR size for Gate D MVP.
pub const E1000_MMIO_SIZE: u32 = 128 * 1024;

/// Device control register.
pub const REG_CTRL: u32 = 0x0000;
/// Device status register.
pub const REG_STATUS: u32 = 0x0008;
/// Transmit descriptor head.
pub const REG_TDH: u32 = 0x0380;
/// Transmit descriptor tail.
pub const REG_TDT: u32 = 0x0388;
