//! Gate D steady-state datapath loop for the hypervisor image.

#![allow(static_mut_refs)]

use core::arch::asm;

use hv_runtime::{assign_ipc_host_backing, DatapathEngine, GateDInitReport, GateDPlans};

use crate::serial;

/// Host backing for the two IPC channels in `configs/qemu.yaml`.
/// Page-rounded so consecutive EPT IPC mappings stay aligned.
const IPC_MAPPING_BYTES: usize = 528_384;

#[repr(align(4096))]
struct IpcBacking {
    bytes: [u8; IPC_MAPPING_BYTES * 2],
}

static mut IPC_BACKING: IpcBacking = IpcBacking { bytes: [0; IPC_MAPPING_BYTES * 2] };

/// Patches embedded plans to use hypervisor-local IPC backing and initializes rings.
///
/// # Errors
///
/// Returns [`hv_runtime::RuntimeError`] when backing assignment or ring init fails.
pub fn prepare_local_ipc_backing(plans: &mut GateDPlans) -> Result<(), hv_runtime::RuntimeError> {
    // SAFETY: called once during single-threaded init before the steady-state loop.
    let backing = unsafe {
        core::slice::from_raw_parts_mut(IPC_BACKING.bytes.as_mut_ptr(), IPC_BACKING.bytes.len())
    };
    assign_ipc_host_backing(plans, backing)
}

/// Runs the host-assisted datapath loop after Gate D initialization succeeds.
pub fn run_steady_state_loop(plans: &GateDPlans, _report: GateDInitReport) -> ! {
    // SAFETY: IPC backing is initialized during init and not mutated concurrently.
    let backing = unsafe {
        core::slice::from_raw_parts_mut(IPC_BACKING.bytes.as_mut_ptr(), IPC_BACKING.bytes.len())
    };
    let mut engine = match DatapathEngine::from_plans(plans, backing) {
        Ok(engine) => engine,
        Err(_) => {
            serial::write_str("hypster: datapath engine fail\n");
            halt_forever();
        }
    };

    const E2E_PAYLOAD: &[u8] = b"hypster-qemu-e2e";
    let mut e2e_out = [0u8; 2048];
    match engine.run_e2e_once(E2E_PAYLOAD, &mut e2e_out) {
        Ok(report)
            if report.outbound_bytes >= E2E_PAYLOAD.len()
                && &e2e_out[..E2E_PAYLOAD.len()] == E2E_PAYLOAD =>
        {
            serial::write_str("hypster: e2e ok\n");
        }
        _ => serial::write_str("hypster: e2e fail\n"),
    }

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
