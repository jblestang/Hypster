//! Host-assisted end-to-end datapath orchestration (IN → MID → OUT).

use hv_e1000::{E1000DeviceState, MAX_FRAME_BYTES};
use hv_ipc::IpcError;

use crate::datapath::{
    drain_outbound_payload, inject_inbound_payload, run_datapath_step, ChannelBacking,
    DatapathChannels, DatapathStepReport,
};
use crate::mmio::MmioDispatch;
use crate::partition::GateDPlans;

/// Summary of datapath preparation during Gate D init.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatapathPrepReport {
    /// Number of IPC channels in the validated topology.
    pub ipc_channels: usize,
    /// Expected MMIO device count across all partitions.
    pub mmio_devices: usize,
    /// Ring slot size from the first IPC channel.
    pub slot_size: u32,
}

/// Validates datapath prerequisites from Gate D plans.
pub fn prepare_datapath(plans: &GateDPlans) -> Result<DatapathPrepReport, IpcError> {
    let ipc_channels = plans.ipc_channels.len();
    let slot_size = plans.ipc_channels.first().map(|channel| channel.slot_size).unwrap_or(0);
    let mmio_devices = plans
        .gate_c
        .ept
        .partitions
        .iter()
        .map(|partition| {
            partition
                .mappings
                .iter()
                .filter(|mapping| mapping.memory_type == hv_ept::EptMemoryType::Uncacheable)
                .count()
        })
        .sum();
    if ipc_channels < 2 {
        return Err(IpcError::ParameterMismatch);
    }
    Ok(DatapathPrepReport { ipc_channels, mmio_devices, slot_size })
}

/// End-to-end datapath engine combining e1000 bookends and IPC relay.
pub struct DatapathEngine<'a> {
    in_nic: E1000DeviceState,
    out_nic: E1000DeviceState,
    channels: DatapathChannels<'a>,
    relay_slot: alloc::vec::Vec<u8>,
}

/// Summary of one full e2e transfer attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E2eTransferReport {
    /// IPC relay report for the final step.
    pub step: DatapathStepReport,
    /// Payload bytes delivered to the OUT-facing device.
    pub outbound_bytes: usize,
}

impl<'a> DatapathEngine<'a> {
    /// Builds an engine from Gate D IPC plans and contiguous host backing memory.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] when channel layout is invalid or required channels are missing.
    pub fn from_plans(plans: &GateDPlans, ipc_backing: &'a mut [u8]) -> Result<Self, IpcError> {
        let (in_to_mid, mid_to_out, slot_size) = build_topology_channels(plans, ipc_backing)?;
        Ok(Self {
            in_nic: E1000DeviceState::new_link_up(),
            out_nic: E1000DeviceState::new_link_up(),
            channels: DatapathChannels { in_to_mid, mid_to_out },
            relay_slot: alloc::vec![0u8; slot_size as usize],
        })
    }

    /// Builds an engine using MMIO-discovered NIC instances and IPC backing memory.
    pub fn from_plans_and_mmio(
        plans: &GateDPlans,
        ipc_backing: &'a mut [u8],
        mmio: &MmioDispatch,
    ) -> Result<Self, IpcError> {
        let mut engine = Self::from_plans(plans, ipc_backing)?;
        if let Some(in_nic) = mmio.in_nic() {
            engine.in_nic = in_nic.clone();
        }
        if let Some(out_nic) = mmio.out_nic() {
            engine.out_nic = out_nic.clone();
        }
        Ok(engine)
    }

    /// Injects one inbound frame on the IN-facing e1000 RX queue.
    pub fn inject_inbound(&mut self, payload: &[u8]) {
        self.in_nic.inject_rx_frame(payload);
    }

    /// Executes one host-assisted datapath step.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] when ring invariants fail.
    pub fn run_step(&mut self) -> Result<DatapathStepReport, IpcError> {
        self.bridge_in_nic_to_ipc()?;
        let mut report = run_datapath_step(&mut self.channels)?;
        let bridged = self.bridge_ipc_to_out_nic()?;
        report.mid_to_out_frames = report.mid_to_out_frames.max(bridged);
        Ok(report)
    }

    /// Moves one frame through the full IN → MID → OUT path.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::QueueEmpty`] when no outbound frame is produced.
    pub fn run_e2e_once(
        &mut self,
        payload: &[u8],
        out: &mut [u8],
    ) -> Result<E2eTransferReport, IpcError> {
        self.inject_inbound(payload);
        let step = self.run_step()?;
        let outbound_bytes = self.drain_outbound(out)?;
        Ok(E2eTransferReport { step, outbound_bytes })
    }

    /// Takes one transmitted frame from the OUT-facing e1000 TX queue.
    pub fn drain_outbound(&mut self, out: &mut [u8]) -> Result<usize, IpcError> {
        self.out_nic.take_tx_frame(out).ok_or(IpcError::QueueEmpty)
    }

    fn bridge_in_nic_to_ipc(&mut self) -> Result<u32, IpcError> {
        let mut frame = [0u8; MAX_FRAME_BYTES];
        let Some(len) = self.in_nic.take_rx_frame(&mut frame) else {
            return Ok(0);
        };
        inject_inbound_payload(&mut self.channels.in_to_mid, &frame[..len])?;
        Ok(1)
    }

    fn bridge_ipc_to_out_nic(&mut self) -> Result<u32, IpcError> {
        let len = drain_outbound_payload(&mut self.channels.mid_to_out, &mut self.relay_slot)?;
        self.out_nic.enqueue_tx_frame(&self.relay_slot[..len]);
        Ok(1)
    }
}

fn build_topology_channels<'a>(
    plans: &GateDPlans,
    backing: &'a mut [u8],
) -> Result<(ChannelBacking<'a>, ChannelBacking<'a>, u32), IpcError> {
    let mut in_layout = None;
    let mut out_layout = None;
    let mut offset = 0usize;
    let total = backing.len();
    let mut slot_size = 0u32;

    for plan in &plans.ipc_channels {
        let size = plan.shared_bytes as usize;
        let end = offset.checked_add(size).ok_or(IpcError::Overflow)?;
        if end > total {
            return Err(IpcError::BufferTooSmall);
        }
        slot_size = plan.slot_size;
        let layout = (offset, size, plan.slot_count, plan.slot_size);
        match plan.name.as_str() {
            "in_to_mid" => in_layout = Some(layout),
            "mid_to_out" => out_layout = Some(layout),
            _ => {}
        }
        offset = end;
    }

    let (in_offset, in_size, in_slots, in_slot_size) =
        in_layout.ok_or(IpcError::ParameterMismatch)?;
    let (out_offset, out_size, out_slots, out_slot_size) =
        out_layout.ok_or(IpcError::ParameterMismatch)?;
    let base = backing.as_mut_ptr();
    // SAFETY: IN and OUT channel layouts occupy disjoint subranges of `backing`.
    let in_slice = unsafe { core::slice::from_raw_parts_mut(base.add(in_offset), in_size) };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(base.add(out_offset), out_size) };
    Ok((
        ChannelBacking {
            name: "in_to_mid",
            backing: in_slice,
            slot_count: in_slots,
            slot_size: in_slot_size,
        },
        ChannelBacking {
            name: "mid_to_out",
            backing: out_slice,
            slot_count: out_slots,
            slot_size: out_slot_size,
        },
        slot_size,
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use alloc::vec;

    use hv_ipc::init_ring;

    use super::*;

    const SLOT_COUNT: u32 = 8;
    const SLOT_SIZE: u32 = 2048;

    fn sample_plans(in_to_mid: &mut [u8], mid_to_out: &mut [u8]) -> GateDPlans {
        use crate::init::GateCPlans;
        use crate::partition::IpcChannelPlan;
        use hv_config_model::hash::ConfigHash;
        use hv_ept::EptPlan;
        use hv_types::HostPhysAddr;
        use hv_vtd::VtdPlan;

        GateDPlans {
            gate_c: GateCPlans {
                ept: EptPlan { partitions: alloc::vec::Vec::new() },
                vtd: VtdPlan { domains: alloc::vec::Vec::new() },
                ept_table_base: HostPhysAddr::new(0x2000_0000),
                vtd_table_base: HostPhysAddr::new(0x2100_0000),
                vmxon_region_base: HostPhysAddr::new(0x2200_0000),
            },
            ipc_channels: alloc::vec![
                IpcChannelPlan {
                    name: alloc::string::String::from("in_to_mid"),
                    slot_count: SLOT_COUNT,
                    slot_size: SLOT_SIZE,
                    host_base: HostPhysAddr::new(in_to_mid.as_mut_ptr() as u64),
                    shared_bytes: in_to_mid.len() as u64,
                },
                IpcChannelPlan {
                    name: alloc::string::String::from("mid_to_out"),
                    slot_count: SLOT_COUNT,
                    slot_size: SLOT_SIZE,
                    host_base: HostPhysAddr::new(mid_to_out.as_mut_ptr() as u64),
                    shared_bytes: mid_to_out.len() as u64,
                },
            ],
            config_hash: ConfigHash([0; 32]),
            require_config_hash: false,
        }
    }

    #[test]
    fn e2e_once_moves_payload_in_to_out() {
        let mut in_to_mid = vec![0u8; 16424];
        let mut mid_to_out = vec![0u8; 16424];
        init_ring(&mut in_to_mid, "in_to_mid", SLOT_COUNT, SLOT_SIZE).expect("init in");
        init_ring(&mut mid_to_out, "mid_to_out", SLOT_COUNT, SLOT_SIZE).expect("init out");
        let plans = sample_plans(&mut in_to_mid, &mut mid_to_out);
        let mut backing = vec![0u8; in_to_mid.len() + mid_to_out.len()];
        backing[..in_to_mid.len()].copy_from_slice(&in_to_mid);
        backing[in_to_mid.len()..].copy_from_slice(&mid_to_out);
        let mut engine = DatapathEngine::from_plans(&plans, &mut backing).expect("engine");
        let mut out = [0u8; SLOT_SIZE as usize];
        let report = engine.run_e2e_once(b"udp-payload", &mut out).expect("e2e");
        assert_eq!(report.step.in_to_mid_frames, 1);
        assert!(report.outbound_bytes >= b"udp-payload".len());
        assert_eq!(&out[..b"udp-payload".len()], b"udp-payload");
    }
}
