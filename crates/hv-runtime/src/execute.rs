//! Guest partition VMLAUNCH execution.

use hv_ept::EptMemoryType;
use hv_types::{GuestPhysAddr, HostPhysAddr, VmId};
use hv_vmx::{
    build_guest_vmcs_fields, vmclear, vmlaunch, vmptrld, vmread, vmresume, vmwrite, GuestLaunchPlan,
    VmcsFieldWrite, VmcsRegion, VmxCapabilities,
};
use hv_vmx::vmcs::{HOST_CR0, HOST_CR3, HOST_CR4, HOST_RIP, HOST_RSP, VM_EXIT_REASON};

use crate::error::RuntimeError;
use crate::launch::{plan_partition_launches, DEFAULT_GUEST_BOOT_INFO_GPA};
use crate::load::load_elf_into_guest_ram;
use crate::partition::GateDPlans;

/// Host state captured before VMLAUNCH.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostLaunchContext {
    /// Host RIP for VM-exit resume.
    pub host_rip: u64,
    /// Host RSP for VM-exit resume.
    pub host_rsp: u64,
    /// Host CR0.
    pub host_cr0: u64,
    /// Host CR3.
    pub host_cr3: u64,
    /// Host CR4.
    pub host_cr4: u64,
}

/// Loads a guest ELF image into the partition guest-RAM backing and writes a boot-info header.
///
/// # Errors
///
/// Returns [`RuntimeError::TableRegionUnavailable`] when guest RAM backing cannot be located.
pub fn stage_guest_image(
    plans: &GateDPlans,
    vm_id: VmId,
    image: &[u8],
    boot_info: &[u8],
) -> Result<GuestPhysAddr, RuntimeError> {
    let ram = guest_ram_slice(plans, vm_id)?;
    let loaded = load_elf_into_guest_ram(image, ram).map_err(|_| RuntimeError::TableRegionUnavailable)?;
    let boot_gpa = DEFAULT_GUEST_BOOT_INFO_GPA as usize;
    if boot_info.len() + boot_gpa > ram.len() {
        return Err(RuntimeError::TableRegionUnavailable);
    }
    ram[boot_gpa..boot_gpa + boot_info.len()].copy_from_slice(boot_info);
    Ok(GuestPhysAddr::new(loaded.entry_point))
}

/// Reads the current VM-exit reason from the active VMCS.
///
/// # Safety
///
/// Caller must be in VMX root operation with a valid guest VMCS loaded.
pub unsafe fn read_vmexit_reason() -> Result<u32, RuntimeError> {
    let reason = vmread(VM_EXIT_REASON).map_err(RuntimeError::Vmx)? as u32;
    Ok(reason & 0xFFFF)
}

/// Resumes the current guest after handling a VM-exit.
///
/// # Safety
///
/// Caller must have a valid guest VMCS loaded.
pub unsafe fn resume_guest() -> Result<(), RuntimeError> {
    vmresume().map_err(RuntimeError::Vmx)
}

/// Programs VMCS fields and executes VMLAUNCH for one guest.
///
/// Does not return when launch succeeds; the CPU resumes at `host.host_rip` on VM-exit.
///
/// # Safety
///
/// Caller must have completed Gate C init with VMX enabled and valid table regions.
pub unsafe fn vmlaunch_guest(
    launch: &GuestLaunchPlan,
    host: HostLaunchContext,
    caps: VmxCapabilities,
) -> Result<(), RuntimeError> {
    init_vmcs_region(launch.vmcs_hpa, caps.revision_id)?;
    program_vmcs(launch, host, caps)?;
    vmlaunch().map_err(RuntimeError::Vmx)
}

/// Returns the launch plan for one VM, if present.
#[must_use]
pub fn launch_plan_for_vm(plans: &GateDPlans, vm_id: VmId) -> Option<GuestLaunchPlan> {
    plan_partition_launches(plans).into_iter().find(|plan| plan.vm_id == vm_id)
}

/// Captures host control registers for VMCS host-state fields.
#[must_use]
pub fn capture_host_launch_context(host_rip: u64) -> HostLaunchContext {
    let (host_rsp, host_cr3) = read_host_rsp_cr3();
    let host_cr4 = hv_x86::read_cr4().unwrap_or(0x2000);
    HostLaunchContext {
        host_rip,
        host_rsp,
        host_cr0: 0x8001_0033,
        host_cr3,
        host_cr4,
    }
}

fn read_host_rsp_cr3() -> (u64, u64) {
    let host_rsp: u64;
    let host_cr3: u64;
    // SAFETY: reads current host stack pointer and page table base.
    unsafe {
        core::arch::asm!(
            "mov {rsp}, rsp",
            "mov {cr3}, cr3",
            rsp = out(reg) host_rsp,
            cr3 = out(reg) host_cr3,
            options(nomem, nostack, preserves_flags),
        );
    }
    (host_rsp, host_cr3)
}

fn guest_ram_slice(plans: &GateDPlans, vm_id: VmId) -> Result<&mut [u8], RuntimeError> {
    let partition = plans
        .gate_c
        .ept
        .partitions
        .iter()
        .find(|part| part.vm_id == vm_id)
        .ok_or(RuntimeError::TableRegionUnavailable)?;
    let mapping = partition
        .mappings
        .iter()
        .find(|mapping| mapping.memory_type == EptMemoryType::WriteBack && mapping.guest_phys.raw() == 0)
        .ok_or(RuntimeError::TableRegionUnavailable)?;
    let ptr = mapping.host_phys.raw() as *mut u8;
    let len = mapping.size as usize;
    // SAFETY: EPT plan maps guest RAM at a contiguous host backing region.
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr, len) })
}

fn init_vmcs_region(vmcs_hpa: HostPhysAddr, revision: u32) -> Result<(), RuntimeError> {
    let region = VmcsRegion::new(revision);
    let ptr = vmcs_hpa.raw() as *mut u8;
    // SAFETY: VMCS region is reserved in the platform plan at `vmcs_hpa`.
    unsafe {
        core::ptr::copy_nonoverlapping(region.as_bytes().as_ptr(), ptr, region.as_bytes().len());
        vmclear(vmcs_hpa.raw()).map_err(RuntimeError::Vmx)?;
        vmptrld(vmcs_hpa.raw()).map_err(RuntimeError::Vmx)?;
    }
    Ok(())
}

fn program_vmcs(
    launch: &GuestLaunchPlan,
    host: HostLaunchContext,
    caps: VmxCapabilities,
) -> Result<(), RuntimeError> {
    let mut fields = build_guest_vmcs_fields(launch).map_err(RuntimeError::Vmx)?;
    append_host_fields(&mut fields, host, caps);
    for field in fields {
        // SAFETY: VMCS is loaded and fields are valid for the current CPU.
        unsafe {
            vmwrite(field.field, field.value).map_err(RuntimeError::Vmx)?;
        }
    }
    Ok(())
}

fn append_host_fields(
    fields: &mut alloc::vec::Vec<VmcsFieldWrite>,
    host: HostLaunchContext,
    caps: VmxCapabilities,
) {
    let cr0 = (host.host_cr0 | caps.cr0_fixed0) & caps.cr0_fixed1;
    let cr4 = (host.host_cr4 | caps.cr4_fixed0) & caps.cr4_fixed1;
    fields.push(VmcsFieldWrite { field: HOST_CR0, value: cr0 });
    fields.push(VmcsFieldWrite { field: HOST_CR3, value: host.host_cr3 });
    fields.push(VmcsFieldWrite { field: HOST_CR4, value: cr4 });
    fields.push(VmcsFieldWrite { field: HOST_RSP, value: host.host_rsp });
    fields.push(VmcsFieldWrite { field: HOST_RIP, value: host.host_rip });
}
