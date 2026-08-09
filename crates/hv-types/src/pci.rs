//! PCI topology newtypes.
//!
//! Hex parsing loops use bounded indices derived from validated input lengths.

use core::fmt;

use crate::arithmetic::ArithmeticError;

/// PCI segment (domain) number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PciSegment(pub u16);

impl PciSegment {
    /// Creates a PCI segment identifier.
    #[must_use]
    pub const fn new(segment: u16) -> Self {
        Self(segment)
    }

    /// Returns the raw segment value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for PciSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PciSegment({})", self.0)
    }
}

/// PCI bus number.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PciBus(u8);

impl PciBus {
    /// Creates a PCI bus number.
    #[must_use]
    pub const fn new(bus: u8) -> Self {
        Self(bus)
    }

    /// Returns the raw bus number.
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

impl fmt::Debug for PciBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PciBus({})", self.0)
    }
}

/// PCI device number (0..=31).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PciDevice(u8);

impl PciDevice {
    /// Creates a PCI device number if valid.
    ///
    /// # Errors
    ///
    /// Returns [`ArithmeticError::InvalidPciDevice`] when `device` exceeds 31.
    pub const fn new(device: u8) -> Result<Self, ArithmeticError> {
        if device > 31 {
            return Err(ArithmeticError::InvalidPciDevice);
        }
        Ok(Self(device))
    }

    /// Returns the raw device number.
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

impl fmt::Debug for PciDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PciDevice({})", self.0)
    }
}

/// PCI function number (0..=7).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PciFunction(u8);

impl PciFunction {
    /// Creates a PCI function number if valid.
    ///
    /// # Errors
    ///
    /// Returns [`ArithmeticError::InvalidPciFunction`] when `function` exceeds 7.
    pub const fn new(function: u8) -> Result<Self, ArithmeticError> {
        if function > 7 {
            return Err(ArithmeticError::InvalidPciFunction);
        }
        Ok(Self(function))
    }

    /// Returns the raw function number.
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

impl fmt::Debug for PciFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PciFunction({})", self.0)
    }
}

/// Fully qualified PCI BDF address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PciBdf {
    /// PCI segment (domain).
    pub segment: PciSegment,
    /// PCI bus.
    pub bus: PciBus,
    /// PCI device slot.
    pub device: PciDevice,
    /// PCI function.
    pub function: PciFunction,
}

impl PciBdf {
    /// Parses a BDF string in `SSSS:BB:DD.F` or `BB:DD.F` form.
    ///
    /// # Errors
    ///
    /// Returns [`ArithmeticError::InvalidPciBdf`] or component-specific PCI errors when
    /// the input is malformed.
    pub fn parse(input: &str) -> Result<Self, ArithmeticError> {
        let trimmed = input.trim();
        let (segment_str, rest) = if let Some((seg, rest)) = trimmed.split_once(':') {
            if rest.contains(':') {
                (Some(seg), rest)
            } else {
                (None, trimmed)
            }
        } else {
            return Err(ArithmeticError::InvalidPciBdf);
        };

        let segment = if let Some(seg) = segment_str {
            let value = parse_u16_hex(seg)?;
            PciSegment::new(value)
        } else {
            PciSegment::new(0)
        };

        let mut parts = rest.split(':');
        let bus_str = parts.next().ok_or(ArithmeticError::InvalidPciBdf)?;
        let dev_fn_str = parts.next().ok_or(ArithmeticError::InvalidPciBdf)?;
        if parts.next().is_some() {
            return Err(ArithmeticError::InvalidPciBdf);
        }

        let bus = PciBus::new(parse_u8_hex(bus_str)?);
        let (dev_str, fn_str) = dev_fn_str
            .split_once('.')
            .ok_or(ArithmeticError::InvalidPciBdf)?;
        let device = PciDevice::new(parse_u8_hex(dev_str)?)?;
        let function = PciFunction::new(parse_u8_hex(fn_str)?)?;

        Ok(Self {
            segment,
            bus,
            device,
            function,
        })
    }

    /// Creates a BDF from literal components.
    ///
    /// # Errors
    ///
    /// Returns component-specific PCI errors when `device` or `function` are out of range.
    pub fn from_components(
        segment: u16,
        bus: u8,
        device: u8,
        function: u8,
    ) -> Result<Self, ArithmeticError> {
        Ok(Self {
            segment: PciSegment::new(segment),
            bus: PciBus::new(bus),
            device: PciDevice::new(device)?,
            function: PciFunction::new(function)?,
        })
    }

    /// Formats the BDF as `SSSS:BB:DD.F`.
    #[must_use]
    pub fn format(self) -> [u8; 12] {
        let mut buf = *b"0000:00:00.0";
        write_hex_u16(self.segment.raw(), &mut buf[0..4]);
        write_hex_u8(self.bus.raw(), &mut buf[5..7]);
        write_hex_u8(self.device.raw(), &mut buf[8..10]);
        buf[11] = b'0' + self.function.raw();
        buf
    }
}

impl fmt::Debug for PciBdf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted = self.format();
        let s = core::str::from_utf8(&formatted).map_err(|_| fmt::Error)?;
        write!(f, "PciBdf({s})")
    }
}

impl fmt::Display for PciBdf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatted = self.format();
        let s = core::str::from_utf8(&formatted).map_err(|_| fmt::Error)?;
        f.write_str(s)
    }
}

fn parse_u8_hex(input: &str) -> Result<u8, ArithmeticError> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || bytes.len() > 2 {
        return Err(ArithmeticError::InvalidPciBdf);
    }
    let mut value = 0u8;
    let mut i = 0;
    while i < bytes.len() {
        let Some(&ch) = bytes.get(i) else {
            return Err(ArithmeticError::InvalidPciBdf);
        };
        let digit = hex_digit(ch).ok_or(ArithmeticError::InvalidPciBdf)?;
        value = value.wrapping_shl(4).wrapping_add(digit);
        i += 1;
    }
    Ok(value)
}

fn parse_u16_hex(input: &str) -> Result<u16, ArithmeticError> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || bytes.len() > 4 {
        return Err(ArithmeticError::InvalidPciBdf);
    }
    let mut value = 0u16;
    let mut i = 0;
    while i < bytes.len() {
        let Some(&ch) = bytes.get(i) else {
            return Err(ArithmeticError::InvalidPciBdf);
        };
        let digit = hex_digit(ch).ok_or(ArithmeticError::InvalidPciBdf)?;
        value = value.wrapping_shl(4).wrapping_add(u16::from(digit));
        i += 1;
    }
    Ok(value)
}

const fn hex_digit(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

fn write_hex_u8(value: u8, out: &mut [u8]) {
    let hi = (value >> 4) & 0xF;
    let lo = value & 0xF;
    if out.len() >= 2 {
        if let Some(slot) = out.get_mut(0) {
            *slot = b"0123456789abcdef"[hi as usize];
        }
        if let Some(slot) = out.get_mut(1) {
            *slot = b"0123456789abcdef"[lo as usize];
        }
    }
}

fn write_hex_u16(value: u16, out: &mut [u8]) {
    let nibbles = [
        ((value >> 12) & 0xF) as u8,
        ((value >> 8) & 0xF) as u8,
        ((value >> 4) & 0xF) as u8,
        (value & 0xF) as u8,
    ];
    let start = out.len().saturating_sub(4);
    let mut i = 0;
    while i < 4 {
        if let Some(slot) = out.get_mut(start + i) {
            *slot = b"0123456789abcdef"[nibbles[i] as usize];
        }
        i += 1;
    }
}
