//! Identity-mapped physical memory reads (Gate C MVP).

use hv_types::HostPhysAddr;

use crate::error::X86Error;

/// Maximum physical address readable via identity map in Gate C MVP.
const IDENTITY_MAP_LIMIT: u64 = 0x0000_0001_0000_0000;

/// Reads one byte from a physical address using the Gate C identity map.
///
/// # Safety
///
/// The address must refer to valid, mapped physical memory.
pub unsafe fn read_phys_u8(addr: HostPhysAddr) -> Result<u8, X86Error> {
    let mut buf = [0u8; 1];
    read_phys_bytes(addr, &mut buf)?;
    Ok(buf[0])
}

/// Reads bytes from a physical address using the Gate C identity map.
///
/// # Safety
///
/// The address range must refer to valid, mapped physical memory.
pub unsafe fn read_phys_bytes(addr: HostPhysAddr, out: &mut [u8]) -> Result<(), X86Error> {
    if out.is_empty() {
        return Ok(());
    }
    let base = addr.raw();
    let end = base.checked_add(out.len() as u64).ok_or(X86Error::Overflow)?;
    if end > IDENTITY_MAP_LIMIT {
        return Err(X86Error::NotMapped);
    }
    let src = base as *const u8;
    for (idx, byte) in out.iter_mut().enumerate() {
        *byte = unsafe { src.add(idx).read_volatile() };
    }
    Ok(())
}
