//! Gate D embedded platform plans.

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/gate_d_plans.rs"));
}

use hv_boot_abi::BootInfo;
use hv_runtime::{GateDInitReport, RuntimeError};

pub use embedded::embedded_gate_d_plans;

/// Runs Gate D runtime init using build-time embedded plans.
pub fn run_gate_d_init(boot_info: &BootInfo) -> Result<GateDInitReport, RuntimeError> {
    let plans = embedded_gate_d_plans();
    hv_runtime::initialize_gate_d(boot_info, &plans)
}
