//! Gate D embedded platform plans.

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/gate_d_plans.rs"));
}

use hv_boot_abi::BootInfo;
use hv_runtime::{GateDInitReport, GateDPlans, RuntimeError};

pub use embedded::embedded_gate_d_plans;

/// Runs Gate D runtime init using build-time embedded plans.
pub fn run_gate_d_init(
    boot_info: &BootInfo,
) -> Result<(GateDPlans, GateDInitReport), RuntimeError> {
    let mut plans = embedded_gate_d_plans();
    crate::datapath::prepare_local_ipc_backing(&mut plans)?;
    let report = hv_runtime::initialize_gate_d(boot_info, &mut plans)?;
    Ok((plans, report))
}
