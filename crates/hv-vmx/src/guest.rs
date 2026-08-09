//! Guest VMCS launch planning and field programming model.

use hv_types::{GuestPhysAddr, HostPhysAddr, VmId};

use crate::error::VmxError;
use crate::vmcs::{
    EPT_POINTER, GUEST_CR0, GUEST_CR3, GUEST_CR4, GUEST_RFLAGS, GUEST_RIP, GUEST_RSP,
    PIN_BASED_VM_EXEC_CONTROL, PROC_BASED_VM_EXEC_CONTROL, SECONDARY_PROC_BASED_VM_EXEC_CONTROL,
    VMCS_LINK_POINTER, VM_ENTRY_CONTROLS, VM_EXIT_CONTROLS,
};

/// Secondary processor-based control: enable EPT.
pub const PROC_BASED2_ENABLE_EPT: u64 = 1 << 1;
/// Processor-based control: use secondary controls.
pub const PROC_BASED_ACTIVATE_SECONDARY: u64 = 1 << 31;

/// One partition guest launch descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestLaunchPlan {
    /// VM identifier.
    pub vm_id: VmId,
    /// VMCS region host physical address.
    pub vmcs_hpa: HostPhysAddr,
    /// Guest entry point (RIP).
    pub guest_entry: GuestPhysAddr,
    /// Initial guest stack pointer.
    pub guest_stack: GuestPhysAddr,
    /// EPT root host physical address.
    pub ept_root_hpa: HostPhysAddr,
    /// Guest boot info guest physical address.
    pub guest_boot_info_gpa: GuestPhysAddr,
}

/// Encoded VMCS field assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VmcsFieldWrite {
    /// VMCS field encoding.
    pub field: u32,
    /// Value to write.
    pub value: u64,
}

/// Builds the minimal VMCS field writes required before VMLAUNCH.
///
/// # Errors
///
/// Returns [`VmxError::InvalidGuestLaunchPlan`] when addresses are misaligned.
pub fn build_guest_vmcs_fields(
    plan: &GuestLaunchPlan,
) -> Result<alloc::vec::Vec<VmcsFieldWrite>, VmxError> {
    if plan.vmcs_hpa.raw() & 0xFFF != 0 {
        return Err(VmxError::MisalignedVmcsRegion);
    }
    if plan.ept_root_hpa.raw() & 0xFFF != 0 {
        return Err(VmxError::MisalignedEptRoot);
    }

    let proc_based = PROC_BASED_ACTIVATE_SECONDARY;
    let proc_based2 = PROC_BASED2_ENABLE_EPT;
    let ept_ptr = plan.ept_root_hpa.raw() | 0x6;

    Ok(alloc::vec![
        VmcsFieldWrite { field: VMCS_LINK_POINTER, value: u64::MAX },
        VmcsFieldWrite { field: PIN_BASED_VM_EXEC_CONTROL, value: 0 },
        VmcsFieldWrite { field: PROC_BASED_VM_EXEC_CONTROL, value: proc_based },
        VmcsFieldWrite { field: SECONDARY_PROC_BASED_VM_EXEC_CONTROL, value: proc_based2 },
        VmcsFieldWrite { field: VM_EXIT_CONTROLS, value: 0 },
        VmcsFieldWrite { field: VM_ENTRY_CONTROLS, value: 0 },
        VmcsFieldWrite { field: GUEST_CR0, value: 0x80010033 },
        VmcsFieldWrite { field: GUEST_CR3, value: 0 },
        VmcsFieldWrite { field: GUEST_CR4, value: 0x2000 },
        VmcsFieldWrite { field: GUEST_RSP, value: plan.guest_stack.raw() },
        VmcsFieldWrite { field: GUEST_RIP, value: plan.guest_entry.raw() },
        VmcsFieldWrite { field: GUEST_RFLAGS, value: 0x2 },
        VmcsFieldWrite { field: EPT_POINTER, value: ept_ptr },
    ])
}

/// Returns true when all required launch plan invariants hold.
#[must_use]
pub fn validate_guest_launch_plan(plan: &GuestLaunchPlan) -> bool {
    build_guest_vmcs_fields(plan).is_ok()
        && plan.guest_entry.raw() < plan.guest_stack.raw()
        && plan.guest_boot_info_gpa.raw() != 0
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn guest_vmcs_fields_include_ept_pointer() {
        let plan = GuestLaunchPlan {
            vm_id: VmId::new(0),
            vmcs_hpa: HostPhysAddr::new(0x2400_0000),
            guest_entry: GuestPhysAddr::new(0x1000),
            guest_stack: GuestPhysAddr::new(0x8000),
            ept_root_hpa: HostPhysAddr::new(0x2000_1000),
            guest_boot_info_gpa: GuestPhysAddr::new(0x9000),
        };
        let fields = build_guest_vmcs_fields(&plan).expect("fields");
        assert!(fields.iter().any(|field| field.field == EPT_POINTER));
        assert!(validate_guest_launch_plan(&plan));
    }

    #[test]
    fn misaligned_vmcs_is_rejected() {
        let plan = GuestLaunchPlan {
            vm_id: VmId::new(0),
            vmcs_hpa: HostPhysAddr::new(0x2400_0001),
            guest_entry: GuestPhysAddr::new(0x1000),
            guest_stack: GuestPhysAddr::new(0x8000),
            ept_root_hpa: HostPhysAddr::new(0x2000_1000),
            guest_boot_info_gpa: GuestPhysAddr::new(0x9000),
        };
        assert!(matches!(build_guest_vmcs_fields(&plan), Err(VmxError::MisalignedVmcsRegion)));
    }
}
