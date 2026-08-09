//! Transfer control to the hypervisor after ExitBootServices.

use hv_boot_abi::BootInfo;

/// Jumps to the hypervisor entry with boot info in the first argument (SysV ABI).
///
/// This function never returns.
pub fn jump_to_hypervisor(entry: u64, boot_info: *const BootInfo) -> ! {
    // SAFETY: inline transfer to the loaded hypervisor entrypoint.
    unsafe {
        core::arch::asm!(
            "mov rdi, {boot_info}",
            "jmp {entry}",
            boot_info = in(reg) boot_info,
            entry = in(reg) entry,
            options(noreturn)
        );
    }
}
