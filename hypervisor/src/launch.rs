//! Guest VMLAUNCH under VMX (requires `hardware` feature).

use hv_runtime::{
    capture_host_launch_context, launch_plan_for_vm, read_vmexit_reason, stage_guest_image,
    vmlaunch_guest, GateDPlans,
};
use hv_types::VmId;
use hv_vmx::VmxCapabilities;

use crate::serial;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/guest_in_image.rs"));
}

use embedded::GUEST_IN_IMAGE;

const VMX_EXIT_REASON_HLT: u32 = 12;

/// Stages the IN guest and attempts VMLAUNCH when VMX is available.
///
/// # Errors
///
/// Returns [`hv_runtime::RuntimeError`] when staging or launch preparation fails.
///
/// # Safety
///
/// Caller must have completed Gate D init with VMX enabled.
pub unsafe fn try_launch_in_guest(plans: &GateDPlans) -> Result<(), hv_runtime::RuntimeError> {
    if GUEST_IN_IMAGE.is_empty() {
        return Err(hv_runtime::RuntimeError::TableRegionUnavailable);
    }
    let vm_id = VmId::new(0);
    let boot_info = minimal_boot_info_bytes();
    let mut launch = launch_plan_for_vm(plans, vm_id).ok_or(hv_runtime::RuntimeError::TableRegionUnavailable)?;
    launch.guest_entry = stage_guest_image(plans, vm_id, GUEST_IN_IMAGE, &boot_info)?;

    let caps = VmxCapabilities::from_hardware().unwrap_or(VmxCapabilities::from_assumed_qemu());
    let host = capture_host_launch_context(vmexit_entry as u64);
    vmlaunch_guest(&launch, host, caps)
}

extern "C" {
    fn vmexit_entry() -> !;
}

/// VM-exit dispatch routine reached from `vmexit_entry`.
#[no_mangle]
extern "C" fn vmexit_dispatch() -> ! {
    match unsafe { read_vmexit_reason() } {
        Ok(VMX_EXIT_REASON_HLT) => serial::write_str("hypster: vmlaunch ok\n"),
        Ok(reason) => {
            serial::write_str("hypster: vmexit reason=");
            write_hex_u32(reason);
            serial::write_str("\n");
        }
        Err(_) => serial::write_str("hypster: vmexit read fail\n"),
    }
    halt_forever()
}

fn minimal_boot_info_bytes() -> [u8; core::mem::size_of::<hv_guest_abi::GuestBootInfo>()] {
    let info = hv_guest_abi::GuestBootInfo {
        header: hv_guest_abi::GuestBootInfoHeader {
            magic: hv_guest_abi::GUEST_BOOT_INFO_MAGIC,
            version_major: hv_guest_abi::GUEST_ABI_VERSION_MAJOR,
            version_minor: hv_guest_abi::GUEST_ABI_VERSION_MINOR,
            total_size: core::mem::size_of::<hv_guest_abi::GuestBootInfo>() as u32,
            vm_id: 0,
            vcpu_id: 0,
        },
        memory_region_count: 0,
        ipc_region_count: 0,
        mmio_region_count: 0,
        reserved: 0,
    };
    let mut bytes = [0u8; core::mem::size_of::<hv_guest_abi::GuestBootInfo>()];
    unsafe {
        core::ptr::write(bytes.as_mut_ptr() as *mut hv_guest_abi::GuestBootInfo, info);
    }
    bytes
}

fn write_hex_u32(value: u32) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0xF) as usize;
        serial::write_byte(HEX[nibble]);
    }
}

fn halt_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
