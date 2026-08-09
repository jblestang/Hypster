//! VT-d root and context table installation into a caller-provided buffer.

use hv_types::{HostPhysAddr, PciBdf};

use crate::error::VtdPlanError;
use crate::model::VtdDomainPlan;
use crate::table::{
    context_entry, context_index, root_entry, CONTEXT_ENTRY_SIZE, CONTEXT_TABLE_SIZE,
    ROOT_ENTRY_SIZE, ROOT_TABLE_SIZE,
};

/// Result of installing VT-d domain tables into a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VtdInstallResult {
    /// HPA of the root table.
    pub root_table_hpa: HostPhysAddr,
    /// HPA of the primary context table.
    pub context_table_hpa: HostPhysAddr,
    /// Bytes consumed from the install buffer.
    pub bytes_used: usize,
}

/// VT-d tables (root + context) built in a caller-provided buffer.
pub struct VtdContextTables<'a> {
    buffer: &'a mut [u8],
    buffer_hpa: HostPhysAddr,
    root_offset: usize,
    context_offset: usize,
}

impl<'a> VtdContextTables<'a> {
    fn new(buffer: &'a mut [u8], buffer_hpa: HostPhysAddr) -> Result<Self, VtdPlanError> {
        if buffer.len() < ROOT_TABLE_SIZE + CONTEXT_TABLE_SIZE {
            return Err(VtdPlanError::BufferTooSmall);
        }
        buffer[..ROOT_TABLE_SIZE + CONTEXT_TABLE_SIZE].fill(0);
        Ok(Self {
            buffer,
            buffer_hpa,
            root_offset: 0,
            context_offset: ROOT_TABLE_SIZE,
        })
    }

    fn root_table_hpa(&self) -> HostPhysAddr {
        HostPhysAddr::new(self.buffer_hpa.raw() + self.root_offset as u64)
    }

    fn context_table_hpa(&self) -> HostPhysAddr {
        HostPhysAddr::new(self.buffer_hpa.raw() + self.context_offset as u64)
    }

    fn install_root_pointer(&mut self) {
        let entry = root_entry(self.context_table_hpa().raw());
        self.buffer[self.root_offset..self.root_offset + ROOT_ENTRY_SIZE]
            .copy_from_slice(&entry);
    }

    fn write_context_entry(
        &mut self,
        bus: u8,
        device: u8,
        function: u8,
        domain_id: u16,
    ) -> Result<(), VtdPlanError> {
        if bus as usize >= ROOT_TABLE_SIZE / ROOT_ENTRY_SIZE {
            return Err(VtdPlanError::Overflow);
        }
        let root_entry_offset = self.root_offset + bus as usize * ROOT_ENTRY_SIZE;
        if self.buffer[root_entry_offset..root_entry_offset + ROOT_ENTRY_SIZE]
            .iter()
            .all(|byte| *byte == 0)
        {
            let entry = root_entry(self.context_table_hpa().raw());
            self.buffer[root_entry_offset..root_entry_offset + ROOT_ENTRY_SIZE]
                .copy_from_slice(&entry);
        }

        let index = context_index(device, function);
        let entry_offset = self.context_offset + index * CONTEXT_ENTRY_SIZE;
        if entry_offset + CONTEXT_ENTRY_SIZE > self.buffer.len() {
            return Err(VtdPlanError::BufferTooSmall);
        }
        let entry = context_entry(domain_id);
        self.buffer[entry_offset..entry_offset + CONTEXT_ENTRY_SIZE].copy_from_slice(&entry);
        Ok(())
    }
}

/// Descriptor for programming VT-d MMIO registers after table install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VtdHardwareState {
    /// Root table physical address.
    pub root_table: HostPhysAddr,
    /// Context table physical address.
    pub context_table: HostPhysAddr,
    /// DRHD register base to program.
    pub drhd_base: HostPhysAddr,
}

impl VtdHardwareState {
    /// Builds MMIO programming state from install output and DRHD base.
    #[must_use]
    pub const fn from_install(result: VtdInstallResult, drhd_base: HostPhysAddr) -> Self {
        Self {
            root_table: result.root_table_hpa,
            context_table: result.context_table_hpa,
            drhd_base,
        }
    }
}

/// Installs VT-d domain tables for `domains` into `buffer`.
///
/// # Errors
///
/// Returns [`VtdPlanError`] when the buffer is too small or a device BDF is invalid.
pub fn install_vtd_domains(
    buffer: &mut [u8],
    buffer_hpa: HostPhysAddr,
    domains: &[VtdDomainPlan],
    drhd_register_base: HostPhysAddr,
) -> Result<VtdInstallResult, VtdPlanError> {
    let _ = drhd_register_base;
    if domains.is_empty() {
        return Err(VtdPlanError::EmptyDomains);
    }

    let mut tables = VtdContextTables::new(buffer, buffer_hpa)?;
    tables.install_root_pointer();

    for domain in domains {
        for device_bdf in &domain.devices {
            let parsed = PciBdf::parse(device_bdf).map_err(|_| VtdPlanError::InvalidPciBdf)?;
            tables.write_context_entry(
                parsed.bus.raw(),
                parsed.device.raw(),
                parsed.function.raw(),
                domain.domain_id.raw(),
            )?;
        }
    }

    Ok(VtdInstallResult {
        root_table_hpa: tables.root_table_hpa(),
        context_table_hpa: tables.context_table_hpa(),
        bytes_used: ROOT_TABLE_SIZE + CONTEXT_TABLE_SIZE,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{install_vtd_domains, VtdHardwareState};
    use crate::model::VtdDomainPlan;
    use crate::table::{CONTEXT_TABLE_SIZE, ROOT_TABLE_SIZE};
    use alloc::string::ToString;
    use alloc::vec;
    use hv_types::{HostPhysAddr, IommuDomainId, VmId};

    #[test]
    fn three_domains_from_qemu_plan_layout() {
        let domains = vec![
            VtdDomainPlan {
                domain_id: IommuDomainId::new(1),
                vm_id: VmId::new(0),
                devices: vec!["0000:00:03.0".to_string()],
                mappings: vec![],
            },
            VtdDomainPlan {
                domain_id: IommuDomainId::new(2),
                vm_id: VmId::new(1),
                devices: vec![],
                mappings: vec![],
            },
            VtdDomainPlan {
                domain_id: IommuDomainId::new(3),
                vm_id: VmId::new(2),
                devices: vec!["0000:00:04.0".to_string()],
                mappings: vec![],
            },
        ];

        let mut buffer = [0u8; ROOT_TABLE_SIZE + CONTEXT_TABLE_SIZE];
        let drhd = HostPhysAddr::new(0xFED9_0000);
        let result = install_vtd_domains(
            &mut buffer,
            HostPhysAddr::new(0x20_0000),
            &domains,
            drhd,
        )
        .expect("install VT-d");

        assert_eq!(result.root_table_hpa, HostPhysAddr::new(0x20_0000));
        assert_eq!(
            result.context_table_hpa,
            HostPhysAddr::new(0x20_0000 + ROOT_TABLE_SIZE as u64)
        );
        assert_eq!(result.bytes_used, ROOT_TABLE_SIZE + CONTEXT_TABLE_SIZE);

        let hw = VtdHardwareState::from_install(result, drhd);
        assert_eq!(hw.drhd_base, drhd);
        assert_ne!(buffer[0], 0);
    }
}
