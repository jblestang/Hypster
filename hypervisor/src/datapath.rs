//! Gate D steady-state datapath loop for the hypervisor image.

#![allow(static_mut_refs)]

use core::arch::asm;

use hv_ipc::init_ring;
use hv_runtime::{assign_ipc_host_backing, DatapathEngine, GateDInitReport, GateDPlans};

/// Host backing for the two IPC channels in `configs/qemu.yaml`.
const IPC_RING_BYTES: usize = 524_328;

static mut IPC_BACKING: [u8; IPC_RING_BYTES * 2] = [0; IPC_RING_BYTES * 2];

/// Patches embedded plans to use hypervisor-local IPC backing and initializes rings.
///
/// # Errors
///
/// Returns [`hv_runtime::RuntimeError`] when backing assignment or ring init fails.
pub fn prepare_local_ipc_backing(plans: &mut GateDPlans) -> Result<(), hv_runtime::RuntimeError> {
    // SAFETY: called once during single-threaded init before the steady-state loop.
    let backing =
        unsafe { core::slice::from_raw_parts_mut(IPC_BACKING.as_mut_ptr(), IPC_BACKING.len()) };
    assign_ipc_host_backing(plans, backing)?;
    for channel in &plans.ipc_channels {
        let ptr = channel.host_base.raw() as *mut u8;
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, channel.shared_bytes as usize) };
        init_ring(slice, &channel.name, channel.slot_count, channel.slot_size)?;
    }
    Ok(())
}

/// Runs the host-assisted datapath loop after Gate D initialization succeeds.
pub fn run_steady_state_loop(plans: &GateDPlans, _report: GateDInitReport) -> ! {
    // SAFETY: IPC backing is initialized during init and not mutated concurrently.
    let backing =
        unsafe { core::slice::from_raw_parts_mut(IPC_BACKING.as_mut_ptr(), IPC_BACKING.len()) };
    let mut engine = match DatapathEngine::from_plans(plans, backing) {
        Ok(engine) => engine,
        Err(_) => halt_forever(),
    };
    loop {
        let _ = engine.run_step();
        // SAFETY: halt until the next host event; no IDT yet in Gate D MVP.
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
