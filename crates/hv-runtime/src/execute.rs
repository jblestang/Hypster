//! Guest partition VMLAUNCH execution.

use hv_ept::EptMemoryType;
use hv_types::{GuestPhysAddr, HostPhysAddr, VmId};
use hv_vmx::guest::{
    PROC_BASED2_ENABLE_EPT, PROC_BASED_ACTIVATE_SECONDARY, PROC_BASED_HLT_EXITING,
};
use hv_vmx::vmcs::{
    EXIT_QUALIFICATION, EPT_POINTER, GUEST_ACTIVITY_STATE, GUEST_CR0, GUEST_CR3, GUEST_CR4,
    GUEST_CS_AR_BYTES, GUEST_CS_BASE, GUEST_CS_LIMIT, GUEST_CS_SELECTOR, GUEST_DR7,
    GUEST_DS_AR_BYTES, GUEST_DS_BASE, GUEST_DS_LIMIT, GUEST_DS_SELECTOR, GUEST_ES_AR_BYTES,
    GUEST_ES_BASE, GUEST_ES_LIMIT, GUEST_ES_SELECTOR, GUEST_FS_AR_BYTES, GUEST_FS_BASE,
    GUEST_FS_LIMIT, GUEST_FS_SELECTOR, GUEST_GDTR_BASE, GUEST_GDTR_LIMIT, GUEST_GS_AR_BYTES,
    GUEST_GS_BASE, GUEST_GS_LIMIT, GUEST_GS_SELECTOR, GUEST_IA32_DEBUGCTL, GUEST_IA32_EFER,
    GUEST_IDTR_BASE, GUEST_IDTR_LIMIT, GUEST_INTERRUPTIBILITY_STATE, GUEST_LDTR_AR_BYTES,
    GUEST_LDTR_BASE, GUEST_LDTR_LIMIT, GUEST_LDTR_SELECTOR, GUEST_PENDING_DBG_EXCEPTIONS,
    GUEST_PHYSICAL_ADDRESS, GUEST_RFLAGS, GUEST_RIP, GUEST_RSP, GUEST_SS_AR_BYTES, GUEST_SS_BASE,
    GUEST_SS_LIMIT, GUEST_SS_SELECTOR, GUEST_SYSENTER_CS, GUEST_SYSENTER_EIP, GUEST_SYSENTER_ESP,
    GUEST_TR_AR_BYTES, GUEST_TR_BASE, GUEST_TR_LIMIT, GUEST_TR_SELECTOR, HOST_CR0, HOST_CR3,
    HOST_CR4, HOST_CS_SELECTOR, HOST_DS_SELECTOR, HOST_ES_SELECTOR, HOST_FS_BASE, HOST_FS_SELECTOR,
    HOST_GDTR_BASE, HOST_GS_BASE, HOST_GS_SELECTOR, HOST_IA32_EFER, HOST_IDTR_BASE, HOST_RIP,
    HOST_RSP, HOST_SS_SELECTOR, HOST_SYSENTER_CS, HOST_SYSENTER_EIP, HOST_SYSENTER_ESP,
    HOST_TR_BASE, HOST_TR_SELECTOR, MSR_BITMAP, PIN_BASED_VM_EXEC_CONTROL,
    PROC_BASED_VM_EXEC_CONTROL, SECONDARY_PROC_BASED_VM_EXEC_CONTROL, VMCS_LINK_POINTER,
    VM_ENTRY_CONTROLS, VM_EXIT_CONTROLS, VM_EXIT_INSTRUCTION_LEN, VM_EXIT_REASON,
    VM_INSTRUCTION_ERROR,
};
use hv_vmx::{
    vmclear, vmlaunch, vmptrld, vmread, vmresume, vmwrite, GuestLaunchPlan, VmcsRegion,
    VmxCapabilities, VmxError,
};
use hv_x86::{
    IA32_EFER, IA32_FS_BASE, IA32_GS_BASE, IA32_SYSENTER_CS, IA32_SYSENTER_EIP, IA32_SYSENTER_ESP,
    IA32_VMX_BASIC, IA32_VMX_ENTRY_CTLS, IA32_VMX_EXIT_CTLS, IA32_VMX_PINBASED_CTLS,
    IA32_VMX_PROCBASED_CTLS, IA32_VMX_PROCBASED_CTLS2, IA32_VMX_TRUE_ENTRY_CTLS,
    IA32_VMX_TRUE_EXIT_CTLS, IA32_VMX_TRUE_PINBASED_CTLS, IA32_VMX_TRUE_PROCBASED_CTLS,
};

use crate::error::RuntimeError;
use crate::guest_paging::{self, GUEST_CR3_GPA, GUEST_GDT_GPA};
use crate::launch::{plan_partition_launches, DEFAULT_GUEST_BOOT_INFO_GPA};
use crate::load::load_elf_into_guest_ram;
use crate::mmio::MmioDispatch;
use crate::partition::GateDPlans;
use crate::vmexit::{handle_mmio_exit, reason, MmioExitInfo};

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
    /// Host CS selector.
    pub host_cs: u16,
    /// Host SS selector.
    pub host_ss: u16,
    /// Host DS selector.
    pub host_ds: u16,
    /// Host ES selector.
    pub host_es: u16,
    /// Host FS selector.
    pub host_fs: u16,
    /// Host GS selector.
    pub host_gs: u16,
    /// Host TR selector (non-null after TSS ensure).
    pub host_tr: u16,
    /// Host TR base.
    pub host_tr_base: u64,
    /// Host GDTR base.
    pub host_gdtr_base: u64,
    /// Host IDTR base.
    pub host_idtr_base: u64,
    /// Host FS base.
    pub host_fs_base: u64,
    /// Host GS base.
    pub host_gs_base: u64,
    /// Host IA32_EFER.
    pub host_efer: u64,
    /// Host IA32_SYSENTER_CS.
    pub host_sysenter_cs: u64,
    /// Host IA32_SYSENTER_ESP.
    pub host_sysenter_esp: u64,
    /// Host IA32_SYSENTER_EIP.
    pub host_sysenter_eip: u64,
}

#[repr(C, packed)]
struct DescriptorTableRegister {
    limit: u16,
    base: u64,
}

#[repr(C, align(4096))]
struct MsrBitmapPage {
    bytes: [u8; 4096],
}

#[repr(C, align(16))]
struct HostTss {
    data: [u8; 104],
}

static mut MSR_BITMAP_PAGE: MsrBitmapPage = MsrBitmapPage { bytes: [0; 4096] };
static mut HOST_TSS: HostTss = HostTss { data: [0; 104] };
static mut HOST_GDT: [u64; 16] = [0; 16];

/// Loads a guest ELF image into the partition guest-RAM backing and writes a boot-info header.
///
/// Also installs a long-mode identity map covering low RAM, IPC GPAs, and MMIO BARs.
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
    let loaded =
        load_elf_into_guest_ram(image, ram).map_err(|_| RuntimeError::TableRegionUnavailable)?;
    let boot_gpa = DEFAULT_GUEST_BOOT_INFO_GPA as usize;
    if boot_info.len() + boot_gpa > ram.len() {
        return Err(RuntimeError::TableRegionUnavailable);
    }
    ram[boot_gpa..boot_gpa + boot_info.len()].copy_from_slice(boot_info);

    let mut high_gpas = alloc::vec::Vec::new();
    if let Some(partition) = plans.gate_c.ept.partitions.iter().find(|part| part.vm_id == vm_id) {
        for mapping in &partition.mappings {
            let gpa = mapping.guest_phys.raw();
            if gpa != 0 {
                high_gpas.push(gpa);
            }
        }
    }
    if !guest_paging::install_identity_map(ram, &high_gpas) {
        return Err(RuntimeError::TableRegionUnavailable);
    }
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

/// Reads the VM-instruction error field after a VMfailValid failure.
///
/// # Safety
///
/// Caller must be in VMX root operation with a valid VMCS loaded.
pub unsafe fn read_vm_instruction_error() -> Result<u32, RuntimeError> {
    let code = vmread(VM_INSTRUCTION_ERROR).map_err(RuntimeError::Vmx)? as u32;
    Ok(code)
}

/// Resumes the current guest after handling a VM-exit.
///
/// # Safety
///
/// Caller must have a valid guest VMCS loaded.
pub unsafe fn resume_guest() -> Result<(), RuntimeError> {
    vmresume().map_err(RuntimeError::Vmx)
}

/// Advances the guest instruction pointer after emulating an exit.
///
/// # Safety
///
/// Caller must be in VMX root operation with a valid guest VMCS loaded.
pub unsafe fn advance_guest_rip(instruction_len: u8) -> Result<(), RuntimeError> {
    let rip = vmread(GUEST_RIP).map_err(RuntimeError::Vmx)?;
    vmwrite(GUEST_RIP, rip + instruction_len as u64).map_err(RuntimeError::Vmx)
}

/// Reads the VM-exit instruction length from the active VMCS.
///
/// # Safety
///
/// Caller must be in VMX root operation with a valid guest VMCS loaded.
pub unsafe fn read_vmexit_instruction_len() -> Result<u8, RuntimeError> {
    let len = vmread(VM_EXIT_INSTRUCTION_LEN).map_err(RuntimeError::Vmx)? as u32;
    Ok(len as u8)
}

/// Reads the guest-physical address associated with the current VM-exit.
///
/// # Safety
///
/// Caller must be in VMX root operation with a valid guest VMCS loaded.
pub unsafe fn read_vmexit_guest_phys() -> Result<GuestPhysAddr, RuntimeError> {
    let gpa = vmread(GUEST_PHYSICAL_ADDRESS).map_err(RuntimeError::Vmx)?;
    Ok(GuestPhysAddr::new(gpa))
}

/// Handles one EPT-violation MMIO exit against the partition dispatcher.
///
/// # Safety
///
/// Caller must be in VMX root operation with a valid guest VMCS loaded.
pub unsafe fn handle_ept_mmio_exit(
    dispatch: &mut MmioDispatch,
    write_value: u32,
) -> Result<(), RuntimeError> {
    let guest_phys = read_vmexit_guest_phys()?;
    let qual = vmread(EXIT_QUALIFICATION).map_err(RuntimeError::Vmx)?;
    let is_write = (qual & (1 << 1)) != 0;
    let info = MmioExitInfo { guest_phys, access_size: 4, is_write };
    let action = handle_mmio_exit(dispatch, info, write_value)?;
    let advance = match action {
        crate::vmexit::MmioExitAction::Read { advance, .. } => advance,
        crate::vmexit::MmioExitAction::Write { advance } => advance,
        crate::vmexit::MmioExitAction::Unhandled { .. } => {
            return Err(RuntimeError::TableRegionUnavailable);
        }
    };
    advance_guest_rip(advance)
}

/// Dispatches one VM-exit for the IN partition, returning true when the guest halted.
///
/// # Safety
///
/// Caller must be in VMX root operation with a valid guest VMCS loaded.
pub unsafe fn dispatch_in_guest_vmexit(dispatch: &mut MmioDispatch) -> Result<bool, RuntimeError> {
    let exit_reason = read_vmexit_reason()?;
    match exit_reason {
        12 => Ok(true),
        reason::EPT_VIOLATION => {
            handle_ept_mmio_exit(dispatch, 0)?;
            Ok(false)
        }
        _ => Err(RuntimeError::TableRegionUnavailable),
    }
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
    match vmlaunch() {
        Ok(()) => Ok(()),
        Err(VmxError::VmlaunchFailed) => Err(RuntimeError::Vmx(VmxError::VmlaunchFailed)),
        Err(err) => Err(RuntimeError::Vmx(err)),
    }
}

/// Returns the launch plan for one VM, if present.
#[must_use]
pub fn launch_plan_for_vm(plans: &GateDPlans, vm_id: VmId) -> Option<GuestLaunchPlan> {
    plan_partition_launches(plans).into_iter().find(|plan| plan.vm_id == vm_id)
}

/// Captures host control registers and segment state for VMCS host-state fields.
#[must_use]
pub fn capture_host_launch_context(host_rip: u64) -> HostLaunchContext {
    let host_cr0 = hv_x86::read_cr0().unwrap_or(0x8001_0033);
    let (host_rsp, host_cr3) = read_host_rsp_cr3();
    let host_cr4 = hv_x86::read_cr4().unwrap_or(0x2000);

    let hs_cs: u16;
    let hs_ss: u16;
    let hs_ds: u16;
    let hs_es: u16;
    let hs_fs: u16;
    let hs_gs: u16;
    let hs_tr: u16;
    // SAFETY: segment register reads are defined on x86_64.
    unsafe {
        core::arch::asm!("mov {0:x}, cs", out(reg) hs_cs, options(nostack, nomem, preserves_flags));
        core::arch::asm!("mov {0:x}, ss", out(reg) hs_ss, options(nostack, nomem, preserves_flags));
        core::arch::asm!("mov {0:x}, ds", out(reg) hs_ds, options(nostack, nomem, preserves_flags));
        core::arch::asm!("mov {0:x}, es", out(reg) hs_es, options(nostack, nomem, preserves_flags));
        core::arch::asm!("mov {0:x}, fs", out(reg) hs_fs, options(nostack, nomem, preserves_flags));
        core::arch::asm!("mov {0:x}, gs", out(reg) hs_gs, options(nostack, nomem, preserves_flags));
        core::arch::asm!("str {0:x}", out(reg) hs_tr, options(nostack, nomem, preserves_flags));
    }

    let mut gdtr = DescriptorTableRegister { limit: 0, base: 0 };
    let mut idtr = DescriptorTableRegister { limit: 0, base: 0 };
    // SAFETY: SGDT/SIDT write the packed descriptor to the provided buffer.
    unsafe {
        core::arch::asm!("sgdt [{}]", in(reg) &mut gdtr, options(nostack));
        core::arch::asm!("sidt [{}]", in(reg) &mut idtr, options(nostack));
    }

    let mut tr_sel = hs_tr & 0xFFF8;
    let tr_base = if tr_sel == 0 {
        // SAFETY: installs a private TSS when firmware left TR null.
        tr_sel = unsafe { ensure_host_tss(host_rsp) };
        unsafe {
            core::arch::asm!("sgdt [{}]", in(reg) &mut gdtr, options(nostack));
        }
        unsafe { read_segment_base(gdtr.base, tr_sel) }
    } else {
        unsafe { read_segment_base(gdtr.base, tr_sel) }
    };

    let host_fs_base = unsafe { hv_x86::read_msr(IA32_FS_BASE) }.map(|m| m.raw()).unwrap_or(0);
    let host_gs_base = unsafe { hv_x86::read_msr(IA32_GS_BASE) }.map(|m| m.raw()).unwrap_or(0);
    let host_efer = unsafe { hv_x86::read_msr(IA32_EFER) }.map(|m| m.raw()).unwrap_or(0x500);
    let host_sysenter_cs =
        unsafe { hv_x86::read_msr(IA32_SYSENTER_CS) }.map(|m| m.raw()).unwrap_or(0);
    let host_sysenter_esp =
        unsafe { hv_x86::read_msr(IA32_SYSENTER_ESP) }.map(|m| m.raw()).unwrap_or(0);
    let host_sysenter_eip =
        unsafe { hv_x86::read_msr(IA32_SYSENTER_EIP) }.map(|m| m.raw()).unwrap_or(0);

    HostLaunchContext {
        host_rip,
        host_rsp,
        host_cr0,
        host_cr3,
        host_cr4,
        host_cs: hs_cs & 0xFFF8,
        host_ss: hs_ss & 0xFFF8,
        host_ds: hs_ds & 0xFFF8,
        host_es: hs_es & 0xFFF8,
        host_fs: hs_fs & 0xFFF8,
        host_gs: hs_gs & 0xFFF8,
        host_tr: tr_sel,
        host_tr_base: tr_base,
        host_gdtr_base: gdtr.base,
        host_idtr_base: idtr.base,
        host_fs_base,
        host_gs_base,
        host_efer,
        host_sysenter_cs,
        host_sysenter_esp,
        host_sysenter_eip,
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
        .find(|mapping| {
            mapping.memory_type == EptMemoryType::WriteBack && mapping.guest_phys.raw() == 0
        })
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
    let basic = unsafe { hv_x86::read_msr(IA32_VMX_BASIC) }.map(|m| m.raw()).unwrap_or(0);
    let use_true = (basic & (1u64 << 55)) != 0;
    let pin_msr = read_ctl_msr(use_true, IA32_VMX_TRUE_PINBASED_CTLS, IA32_VMX_PINBASED_CTLS);
    let proc_msr = read_ctl_msr(use_true, IA32_VMX_TRUE_PROCBASED_CTLS, IA32_VMX_PROCBASED_CTLS);
    let exit_msr = read_ctl_msr(use_true, IA32_VMX_TRUE_EXIT_CTLS, IA32_VMX_EXIT_CTLS);
    let entry_msr = read_ctl_msr(use_true, IA32_VMX_TRUE_ENTRY_CTLS, IA32_VMX_ENTRY_CTLS);
    let proc2_msr =
        unsafe { hv_x86::read_msr(IA32_VMX_PROCBASED_CTLS2) }.map(|m| m.raw()).unwrap_or(0);

    let pin_ctls = adjust_vmx_ctl(0, pin_msr);
    let proc_ctls =
        adjust_vmx_ctl(PROC_BASED_ACTIVATE_SECONDARY | PROC_BASED_HLT_EXITING, proc_msr);
    let proc2_ctls = adjust_vmx_ctl(PROC_BASED2_ENABLE_EPT, proc2_msr);
    let exit_ctls = adjust_vmx_ctl((1 << 9) | (1 << 20) | (1 << 21), exit_msr);
    let entry_ctls = adjust_vmx_ctl((1 << 2) | (1 << 9) | (1 << 15), entry_msr);

    let host_cr0 = (host.host_cr0 | caps.cr0_fixed0) & caps.cr0_fixed1;
    let host_cr4 = (host.host_cr4 | caps.cr4_fixed0) & caps.cr4_fixed1;
    let guest_cr0 = host_cr0 | 0x8000_0031;
    let guest_cr4 = host_cr4 | 0x20;
    let ept_ptr = launch.ept_root_hpa.raw() | (3 << 3) | 6;

    let writes = [
        (VMCS_LINK_POINTER, u64::MAX),
        (PIN_BASED_VM_EXEC_CONTROL, u64::from(pin_ctls)),
        (PROC_BASED_VM_EXEC_CONTROL, u64::from(proc_ctls)),
        (SECONDARY_PROC_BASED_VM_EXEC_CONTROL, u64::from(proc2_ctls)),
        (VM_EXIT_CONTROLS, u64::from(exit_ctls)),
        (VM_ENTRY_CONTROLS, u64::from(entry_ctls)),
        (EPT_POINTER, ept_ptr),
        (GUEST_CR0, guest_cr0),
        (GUEST_CR3, GUEST_CR3_GPA),
        (GUEST_CR4, guest_cr4),
        (GUEST_IA32_EFER, 0x500),
        (GUEST_RIP, launch.guest_entry.raw()),
        (GUEST_RSP, launch.guest_stack.raw()),
        (GUEST_RFLAGS, 0x2),
        (GUEST_CS_SELECTOR, 0x08),
        (GUEST_SS_SELECTOR, 0x10),
        (GUEST_DS_SELECTOR, 0x10),
        (GUEST_ES_SELECTOR, 0x10),
        (GUEST_FS_SELECTOR, 0),
        (GUEST_GS_SELECTOR, 0),
        (GUEST_TR_SELECTOR, 0x18),
        (GUEST_LDTR_SELECTOR, 0),
        (GUEST_CS_BASE, 0),
        (GUEST_SS_BASE, 0),
        (GUEST_DS_BASE, 0),
        (GUEST_ES_BASE, 0),
        (GUEST_FS_BASE, 0),
        (GUEST_GS_BASE, 0),
        (GUEST_TR_BASE, 0),
        (GUEST_LDTR_BASE, 0),
        (GUEST_CS_LIMIT, 0xFFFF_FFFF),
        (GUEST_SS_LIMIT, 0xFFFF_FFFF),
        (GUEST_DS_LIMIT, 0xFFFF_FFFF),
        (GUEST_ES_LIMIT, 0xFFFF_FFFF),
        (GUEST_FS_LIMIT, 0),
        (GUEST_GS_LIMIT, 0),
        (GUEST_TR_LIMIT, 0x67),
        (GUEST_LDTR_LIMIT, 0),
        (GUEST_CS_AR_BYTES, 0xA09B),
        (GUEST_SS_AR_BYTES, 0xC093),
        (GUEST_DS_AR_BYTES, 0xC093),
        (GUEST_ES_AR_BYTES, 0xC093),
        (GUEST_FS_AR_BYTES, 0x1_0000),
        (GUEST_GS_AR_BYTES, 0x1_0000),
        (GUEST_TR_AR_BYTES, 0x8B),
        (GUEST_LDTR_AR_BYTES, 0x1_0000),
        (GUEST_GDTR_BASE, GUEST_GDT_GPA),
        (GUEST_IDTR_BASE, 0),
        (GUEST_GDTR_LIMIT, 0x2F),
        (GUEST_IDTR_LIMIT, 0),
        (GUEST_DR7, 0x400),
        (GUEST_IA32_DEBUGCTL, 0),
        (GUEST_INTERRUPTIBILITY_STATE, 0),
        (GUEST_ACTIVITY_STATE, 0),
        (GUEST_PENDING_DBG_EXCEPTIONS, 0),
        (GUEST_SYSENTER_CS, 0),
        (GUEST_SYSENTER_ESP, 0),
        (GUEST_SYSENTER_EIP, 0),
        (HOST_CR0, host_cr0),
        (HOST_CR3, host.host_cr3),
        (HOST_CR4, host_cr4),
        (HOST_CS_SELECTOR, u64::from(host.host_cs)),
        (HOST_SS_SELECTOR, u64::from(host.host_ss)),
        (HOST_DS_SELECTOR, u64::from(host.host_ds)),
        (HOST_ES_SELECTOR, u64::from(host.host_es)),
        (HOST_FS_SELECTOR, u64::from(host.host_fs)),
        (HOST_GS_SELECTOR, u64::from(host.host_gs)),
        (HOST_TR_SELECTOR, u64::from(host.host_tr)),
        (HOST_FS_BASE, host.host_fs_base),
        (HOST_GS_BASE, host.host_gs_base),
        (HOST_TR_BASE, host.host_tr_base),
        (HOST_GDTR_BASE, host.host_gdtr_base),
        (HOST_IDTR_BASE, host.host_idtr_base),
        (HOST_IA32_EFER, host.host_efer),
        (HOST_SYSENTER_CS, host.host_sysenter_cs),
        (HOST_SYSENTER_ESP, host.host_sysenter_esp),
        (HOST_SYSENTER_EIP, host.host_sysenter_eip),
        (HOST_RSP, host.host_rsp),
        (HOST_RIP, host.host_rip),
    ];

    for (field, value) in writes {
        unsafe {
            vmwrite(field, value).map_err(RuntimeError::Vmx)?;
        }
    }

    if (proc_ctls & (1 << 26)) != 0 {
        let bitmap = core::ptr::addr_of_mut!(MSR_BITMAP_PAGE) as u64;
        unsafe {
            vmwrite(MSR_BITMAP, bitmap).map_err(RuntimeError::Vmx)?;
        }
    }
    Ok(())
}

fn read_ctl_msr(use_true: bool, true_index: u32, legacy_index: u32) -> u64 {
    let index = if use_true { true_index } else { legacy_index };
    unsafe { hv_x86::read_msr(index) }.map(|m| m.raw()).unwrap_or(0)
}

fn adjust_vmx_ctl(desired: u64, msr: u64) -> u32 {
    let allowed0 = msr as u32;
    let allowed1 = (msr >> 32) as u32;
    (desired as u32 | allowed0) & allowed1
}

unsafe fn read_segment_base(gdt_base: u64, selector: u16) -> u64 {
    if selector == 0 {
        return 0;
    }
    let index = (selector >> 3) as u64;
    let desc = *((gdt_base + index * 8) as *const u64);
    let base_0_23 = (desc >> 16) & 0xFF_FFFF;
    let base_24_31 = (desc >> 56) & 0xFF;
    let mut base = base_0_23 | (base_24_31 << 24);
    let typ = ((desc >> 40) & 0xF) as u8;
    let s_bit = ((desc >> 44) & 1) as u8;
    if s_bit == 0 && (typ == 0x9 || typ == 0xB) {
        let upper = *((gdt_base + index * 8 + 8) as *const u64);
        base |= (upper & 0xFFFF_FFFF) << 32;
    }
    base
}

unsafe fn ensure_host_tss(stack_top: u64) -> u16 {
    let gdt = &mut *core::ptr::addr_of_mut!(HOST_GDT);
    let tss = &mut *core::ptr::addr_of_mut!(HOST_TSS);
    tss.data.fill(0);
    tss.data[4..12].copy_from_slice(&stack_top.to_le_bytes());

    let mut gdtr = DescriptorTableRegister { limit: 0, base: 0 };
    core::arch::asm!("sgdt [{}]", in(reg) &mut gdtr, options(nostack));

    let copy_len = ((gdtr.limit as usize) + 1).min(core::mem::size_of_val(gdt));
    if gdtr.base != 0 && copy_len > 0 {
        core::ptr::copy_nonoverlapping(
            gdtr.base as *const u8,
            gdt.as_mut_ptr() as *mut u8,
            copy_len,
        );
    }

    let tss_addr = tss as *mut HostTss as u64;
    // 64-bit TSS descriptor: limit=0x67, type=0x9 (available 64-bit TSS).
    let low = 0x67u64
        | ((tss_addr & 0xFFFF) << 16)
        | (((tss_addr >> 16) & 0xFF) << 32)
        | (0x89u64 << 40)
        | (((tss_addr >> 24) & 0xFF) << 56);
    let high = tss_addr >> 32;
    gdt[8] = low;
    gdt[9] = high;

    let new_gdtr = DescriptorTableRegister {
        limit: (core::mem::size_of_val(gdt) - 1) as u16,
        base: gdt.as_ptr() as u64,
    };
    core::arch::asm!("lgdt [{}]", in(reg) &new_gdtr, options(nostack));
    let selector: u16 = 0x40;
    core::arch::asm!("ltr {0:x}", in(reg) selector, options(nostack, preserves_flags));
    selector
}
