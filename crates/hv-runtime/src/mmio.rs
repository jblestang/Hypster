//! MMIO dispatch for emulated e1000 device BARs.

use hv_e1000::{mmio_read, mmio_write, E1000DeviceState, E1000Error};
use hv_ept::{EptMapping, EptMemoryType, EptPermissions};
use hv_types::GuestPhysAddr;

/// One emulated MMIO device registered with the runtime dispatcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MmioDevice {
    /// Guest physical base of the device BAR.
    pub guest_base: GuestPhysAddr,
    /// BAR size in bytes.
    pub size: u64,
    /// Device model state.
    pub state: E1000DeviceState,
}

/// MMIO dispatcher indexed by guest physical address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MmioDispatch {
    devices: alloc::vec::Vec<MmioDevice>,
}

impl MmioDispatch {
    /// Builds a dispatcher from EPT mappings that describe uncacheable MMIO regions.
    #[must_use]
    pub fn from_ept_mappings(mappings: &[EptMapping]) -> Self {
        let mut devices = alloc::vec::Vec::new();
        for mapping in mappings {
            if mapping.memory_type != EptMemoryType::Uncacheable
                || mapping.permissions != EptPermissions::MMIO
            {
                continue;
            }
            devices.push(MmioDevice {
                guest_base: mapping.guest_phys,
                size: mapping.size,
                state: E1000DeviceState::new_link_up(),
            });
        }
        devices.sort_by_key(|device| device.guest_base.raw());
        Self { devices }
    }

    /// Returns the IN-facing e1000 instance when present.
    pub fn in_nic(&self) -> Option<&E1000DeviceState> {
        self.devices.first().map(|device| &device.state)
    }

    /// Returns the OUT-facing e1000 instance when present.
    pub fn out_nic(&self) -> Option<&E1000DeviceState> {
        self.devices.last().map(|device| &device.state)
    }

    /// Returns the IN-facing e1000 instance when present.
    pub fn in_nic_mut(&mut self) -> Option<&mut E1000DeviceState> {
        self.devices.first_mut().map(|device| &mut device.state)
    }

    /// Returns the OUT-facing e1000 instance when present.
    pub fn out_nic_mut(&mut self) -> Option<&mut E1000DeviceState> {
        self.devices.last_mut().map(|device| &mut device.state)
    }

    /// Reads one 32-bit MMIO register at `guest_phys + offset`.
    ///
    /// # Errors
    ///
    /// Returns [`E1000Error::OffsetOutOfRange`] when no device covers the address.
    pub fn read32(&self, guest_phys: GuestPhysAddr, offset: u32) -> Result<u32, E1000Error> {
        let device = self.find_device(guest_phys)?;
        let local = guest_phys
            .raw()
            .checked_sub(device.guest_base.raw())
            .ok_or(E1000Error::OffsetOutOfRange)?;
        mmio_read(&device.state, local as u32 + offset)
    }

    /// Writes one 32-bit MMIO register at `guest_phys + offset`.
    ///
    /// # Errors
    ///
    /// Returns [`E1000Error::OffsetOutOfRange`] when no device covers the address.
    pub fn write32(
        &mut self,
        guest_phys: GuestPhysAddr,
        offset: u32,
        value: u32,
    ) -> Result<(), E1000Error> {
        let device = self.find_device_mut(guest_phys)?;
        let local = guest_phys
            .raw()
            .checked_sub(device.guest_base.raw())
            .ok_or(E1000Error::OffsetOutOfRange)?;
        mmio_write(&mut device.state, local as u32 + offset, value)
    }

    fn find_device(&self, guest_phys: GuestPhysAddr) -> Result<&MmioDevice, E1000Error> {
        self.devices
            .iter()
            .find(|device| contains_guest_addr(device, guest_phys.raw()))
            .ok_or(E1000Error::OffsetOutOfRange)
    }

    fn find_device_mut(
        &mut self,
        guest_phys: GuestPhysAddr,
    ) -> Result<&mut MmioDevice, E1000Error> {
        self.devices
            .iter_mut()
            .find(|device| contains_guest_addr(device, guest_phys.raw()))
            .ok_or(E1000Error::OffsetOutOfRange)
    }
}

fn contains_guest_addr(device: &MmioDevice, guest_phys: u64) -> bool {
    let base = device.guest_base.raw();
    guest_phys >= base && guest_phys < base.saturating_add(device.size)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use hv_types::HostPhysAddr;

    use super::*;
    #[test]
    fn dispatch_reads_status_from_mmio_mapping() {
        let mappings = &[EptMapping {
            guest_phys: GuestPhysAddr::new(0xFEB0_0000),
            host_phys: HostPhysAddr::new(0x1_1720_0000),
            size: 128 * 1024,
            permissions: EptPermissions::MMIO,
            memory_type: EptMemoryType::Uncacheable,
        }];
        let dispatch = MmioDispatch::from_ept_mappings(mappings);
        let status =
            dispatch.read32(GuestPhysAddr::new(0xFEB0_0000), hv_e1000::REG_STATUS).expect("status");
        assert_ne!(status & 0x80, 0);
    }
}
