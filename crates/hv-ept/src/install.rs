//! EPT page table installation into a caller-provided buffer.

use hv_types::arithmetic::checked_add_u64;
use hv_types::HostPhysAddr;

use crate::error::EptPlanError;
use crate::table::{
    ept_2m_entry, ept_4k_entry, ept_permissions, ept_pointer_entry, EPT_HUGE_2M_SIZE,
    EPT_PAGE_SIZE, EPT_TABLE_SIZE,
};
use crate::types::{EptMapping, EptMemoryType, EptPermissions};

/// Result of installing EPT mappings into a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EptInstallResult {
    /// HPA of the installed EPT PML4 root.
    pub root_hpa: HostPhysAddr,
    /// Bytes consumed from the install buffer.
    pub bytes_used: usize,
}

/// EPT page table hierarchy built in a caller-provided buffer.
pub struct EptPageTable<'a> {
    buffer: &'a mut [u8],
    buffer_hpa: HostPhysAddr,
    cursor: usize,
    root_offset: Option<usize>,
}

impl<'a> EptPageTable<'a> {
    fn new(buffer: &'a mut [u8], buffer_hpa: HostPhysAddr) -> Self {
        Self { buffer, buffer_hpa, cursor: 0, root_offset: None }
    }

    fn alloc_table(&mut self) -> Result<(usize, HostPhysAddr), EptPlanError> {
        if self.cursor + EPT_TABLE_SIZE > self.buffer.len() {
            return Err(EptPlanError::BufferTooSmall);
        }
        let offset = self.cursor;
        self.buffer[offset..offset + EPT_TABLE_SIZE].fill(0);
        self.cursor += EPT_TABLE_SIZE;
        let hpa = HostPhysAddr::new(self.buffer_hpa.raw() + offset as u64);
        Ok((offset, hpa))
    }

    fn root(&self) -> Result<(usize, HostPhysAddr), EptPlanError> {
        let offset = self.root_offset.ok_or(EptPlanError::BufferTooSmall)?;
        Ok((offset, HostPhysAddr::new(self.buffer_hpa.raw() + offset as u64)))
    }

    fn write_entry(&mut self, table_offset: usize, index: usize, value: u64) {
        let entry_offset = table_offset + index * 8;
        self.buffer[entry_offset..entry_offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn read_entry(&self, table_offset: usize, index: usize) -> u64 {
        let entry_offset = table_offset + index * 8;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.buffer[entry_offset..entry_offset + 8]);
        u64::from_le_bytes(bytes)
    }

    fn ensure_root(&mut self) -> Result<(usize, HostPhysAddr), EptPlanError> {
        if self.root_offset.is_some() {
            return self.root();
        }
        let (offset, hpa) = self.alloc_table()?;
        self.root_offset = Some(offset);
        Ok((offset, hpa))
    }

    fn ensure_child(
        &mut self,
        table_offset: usize,
        index: usize,
    ) -> Result<(usize, HostPhysAddr), EptPlanError> {
        let existing = self.read_entry(table_offset, index);
        if existing != 0 {
            let child_hpa = existing & 0x000F_FFFF_FFFF_F000;
            let child_offset = (child_hpa - self.buffer_hpa.raw()) as usize;
            return Ok((child_offset, HostPhysAddr::new(child_hpa)));
        }
        let (child_offset, child_hpa) = self.alloc_table()?;
        self.write_entry(table_offset, index, ept_pointer_entry(child_hpa.raw()));
        Ok((child_offset, child_hpa))
    }

    fn map_2m(&mut self, gpa: u64, hpa: u64, perms: u64, memtype: u64) -> Result<(), EptPlanError> {
        let (root_offset, _) = self.ensure_root()?;
        let pml4_index = ((gpa >> 39) & 0x1FF) as usize;
        let pdpt_index = ((gpa >> 30) & 0x1FF) as usize;
        let pd_index = ((gpa >> 21) & 0x1FF) as usize;

        let (pdpt_offset, _) = self.ensure_child(root_offset, pml4_index)?;
        let (pd_offset, _) = self.ensure_child(pdpt_offset, pdpt_index)?;
        self.write_entry(pd_offset, pd_index, ept_2m_entry(hpa, perms, memtype));
        Ok(())
    }

    fn map_4k(&mut self, gpa: u64, hpa: u64, perms: u64, memtype: u64) -> Result<(), EptPlanError> {
        let (root_offset, _) = self.ensure_root()?;
        let pml4_index = ((gpa >> 39) & 0x1FF) as usize;
        let pdpt_index = ((gpa >> 30) & 0x1FF) as usize;
        let pd_index = ((gpa >> 21) & 0x1FF) as usize;
        let pt_index = ((gpa >> 12) & 0x1FF) as usize;

        let (pdpt_offset, _) = self.ensure_child(root_offset, pml4_index)?;
        let (pd_offset, _) = self.ensure_child(pdpt_offset, pdpt_index)?;
        let (pt_offset, _) = self.ensure_child(pd_offset, pd_index)?;
        self.write_entry(pt_offset, pt_index, ept_4k_entry(hpa, perms, memtype));
        Ok(())
    }

    fn map_range(&mut self, mapping: &EptMapping) -> Result<(), EptPlanError> {
        let perms = mapping_to_permissions(mapping.permissions);
        let memtype = mapping_to_memtype(mapping.memory_type);
        let gpa_base = mapping.guest_phys.raw();
        let hpa_base = mapping.host_phys.raw();
        let mut offset = 0u64;

        while offset < mapping.size {
            let gpa = checked_add_u64(gpa_base, offset).map_err(|_| EptPlanError::Overflow)?;
            let hpa = checked_add_u64(hpa_base, offset).map_err(|_| EptPlanError::Overflow)?;
            let remaining = mapping.size - offset;

            if remaining >= EPT_HUGE_2M_SIZE
                && gpa % EPT_HUGE_2M_SIZE == 0
                && hpa % EPT_HUGE_2M_SIZE == 0
            {
                self.map_2m(gpa, hpa, perms, memtype)?;
                offset += EPT_HUGE_2M_SIZE;
            } else {
                if gpa % EPT_PAGE_SIZE != 0 || hpa % EPT_PAGE_SIZE != 0 {
                    return Err(EptPlanError::MisalignedMapping);
                }
                self.map_4k(gpa, hpa, perms, memtype)?;
                offset += EPT_PAGE_SIZE;
            }
        }
        Ok(())
    }
}

/// Installs EPT mappings into `buffer`, returning the root table HPA.
///
/// # Errors
///
/// Returns [`EptPlanError`] when the buffer is too small or mappings are invalid.
pub fn install_ept_mappings(
    buffer: &mut [u8],
    buffer_hpa: HostPhysAddr,
    mappings: &[EptMapping],
) -> Result<EptInstallResult, EptPlanError> {
    if mappings.is_empty() {
        return Err(EptPlanError::EmptyMappings);
    }

    let mut table = EptPageTable::new(buffer, buffer_hpa);
    for mapping in mappings {
        table.map_range(mapping)?;
    }
    let (_, root_hpa) = table.root()?;
    Ok(EptInstallResult { root_hpa, bytes_used: table.cursor })
}

fn mapping_to_permissions(perms: EptPermissions) -> u64 {
    ept_permissions(perms.read, perms.write, perms.execute)
}

fn mapping_to_memtype(memtype: EptMemoryType) -> u64 {
    match memtype {
        EptMemoryType::WriteBack => crate::table::EPT_MEMTYPE_WB,
        EptMemoryType::Uncacheable => crate::table::EPT_MEMTYPE_UC,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::install_ept_mappings;
    use crate::table::{EPT_HUGE_2M_SIZE, EPT_TABLE_SIZE};
    use crate::types::{EptMapping, EptMemoryType, EptPermissions};
    use hv_types::{GuestPhysAddr, HostPhysAddr};

    const ONE_GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn one_gib_identity_mapping_uses_reasonable_page_count() {
        let mut buffer = [0u8; 64 * 1024];
        let mapping = EptMapping {
            guest_phys: GuestPhysAddr::new(0),
            host_phys: HostPhysAddr::new(0),
            size: ONE_GIB,
            permissions: EptPermissions::GUEST_RAM,
            memory_type: EptMemoryType::WriteBack,
        };
        let result = install_ept_mappings(&mut buffer, HostPhysAddr::new(0x10_0000), &[mapping])
            .expect("install EPT");
        assert_eq!(result.root_hpa, HostPhysAddr::new(0x10_0000));
        // PML4 + PDPT + one PD table with 512 x 2 MiB entries.
        assert_eq!(result.bytes_used, 3 * EPT_TABLE_SIZE);
        let expected_2m_pages = ONE_GIB / EPT_HUGE_2M_SIZE;
        assert_eq!(expected_2m_pages, 512);
    }
}
