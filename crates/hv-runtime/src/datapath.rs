//! End-to-end IPC datapath relay for the IN → MID → OUT topology.

use alloc::vec;

use hv_ipc::{try_pop, try_push, IpcError};

use crate::error::RuntimeError;
use crate::partition::{GateDPlans, IpcChannelPlan};

/// One step of host-assisted datapath relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatapathStepReport {
    /// Frames relayed from IN to MID.
    pub in_to_mid_frames: u32,
    /// Frames relayed from MID to OUT.
    pub mid_to_out_frames: u32,
    /// Payload bytes moved toward OUT.
    pub payload_bytes: u64,
}

/// Channel pair for the validated three-partition topology.
pub struct DatapathChannels<'a> {
    /// Producer/consumer backing for `in_to_mid`.
    pub in_to_mid: ChannelBacking<'a>,
    /// Producer/consumer backing for `mid_to_out`.
    pub mid_to_out: ChannelBacking<'a>,
}

/// Mutable IPC channel backing slice and parameters.
pub struct ChannelBacking<'a> {
    /// Channel name.
    pub name: &'a str,
    /// Shared backing memory.
    pub backing: &'a mut [u8],
    /// Ring slot count.
    pub slot_count: u32,
    /// Ring slot size.
    pub slot_size: u32,
}

impl ChannelBacking<'_> {
    fn relay(&mut self, dst: &mut ChannelBacking<'_>) -> Result<(u32, u64), IpcError> {
        let mut frames = 0u32;
        let mut bytes = 0u64;
        let mut slot = vec![0u8; self.slot_size as usize];
        loop {
            let len = match try_pop(
                self.backing,
                self.name,
                self.slot_count,
                self.slot_size,
                &mut slot,
            ) {
                Ok(len) => len,
                Err(IpcError::QueueEmpty) => break,
                Err(err) => return Err(err),
            };
            let payload_len = len.min(self.slot_size as usize);
            try_push(dst.backing, dst.name, dst.slot_count, dst.slot_size, &slot[..payload_len])?;
            frames += 1;
            bytes += payload_len as u64;
        }
        Ok((frames, bytes))
    }
}

/// Executes one relay pass: drain `in_to_mid` into `mid_to_out`.
///
/// # Errors
///
/// Returns [`IpcError`] when ring invariants fail.
pub fn run_datapath_step(
    channels: &mut DatapathChannels<'_>,
) -> Result<DatapathStepReport, IpcError> {
    let (in_to_mid_frames, in_bytes) = channels.in_to_mid.relay(&mut channels.mid_to_out)?;
    Ok(DatapathStepReport {
        in_to_mid_frames,
        mid_to_out_frames: in_to_mid_frames,
        payload_bytes: in_bytes,
    })
}

/// Injects one UDP payload frame into the IN producer channel.
///
/// # Errors
///
/// Returns [`IpcError::QueueFull`] when the ring has no free slots.
pub fn inject_inbound_payload(
    channel: &mut ChannelBacking<'_>,
    payload: &[u8],
) -> Result<(), IpcError> {
    try_push(channel.backing, channel.name, channel.slot_count, channel.slot_size, payload)?;
    Ok(())
}

/// Drains one frame from the OUT consumer channel.
///
/// # Errors
///
/// Returns [`IpcError::QueueEmpty`] when no frame is available.
pub fn drain_outbound_payload(
    channel: &mut ChannelBacking<'_>,
    out: &mut [u8],
) -> Result<usize, IpcError> {
    try_pop(channel.backing, channel.name, channel.slot_count, channel.slot_size, out)
}

/// Verifies the MID guest relay left `expected` in `mid_to_out`, draining one frame.
///
/// # Errors
///
/// Returns [`RuntimeError::Ipc`] when the ring is empty or the payload prefix mismatches.
pub fn verify_mid_launch_relay(plans: &GateDPlans, expected: &[u8]) -> Result<usize, RuntimeError> {
    let mut channel = mid_to_out_channel(plans)?;
    let mut slot = vec![0u8; channel.slot_size as usize];
    let len = drain_outbound_payload(&mut channel, &mut slot)?;
    if len < expected.len() || &slot[..expected.len()] != expected {
        return Err(RuntimeError::Ipc(IpcError::ParameterMismatch));
    }
    Ok(len)
}

fn mid_to_out_channel(plans: &GateDPlans) -> Result<ChannelBacking<'_>, RuntimeError> {
    let channel = plans
        .ipc_channels
        .iter()
        .find(|ch| ch.name == "mid_to_out")
        .ok_or(RuntimeError::TableRegionUnavailable)?;
    let size = channel.shared_bytes as usize;
    let ptr = channel.host_base.raw() as *mut u8;
    // SAFETY: Gate D init assigned host backing for IPC rings before launch.
    let backing = unsafe { core::slice::from_raw_parts_mut(ptr, size) };
    Ok(ChannelBacking {
        name: "mid_to_out",
        backing,
        slot_count: channel.slot_count,
        slot_size: channel.slot_size,
    })
}

/// Builds channel backings from Gate D IPC plans and a contiguous host buffer.
pub fn channel_backings_from_plans<'a>(
    plans: &'a [IpcChannelPlan],
    backing: &'a mut [u8],
) -> Result<alloc::vec::Vec<ChannelBacking<'a>>, IpcError> {
    let mut out = alloc::vec::Vec::with_capacity(plans.len());
    let mut offset = 0usize;
    let base = backing.as_mut_ptr();
    let total = backing.len();
    for plan in plans {
        let size = plan.shared_bytes as usize;
        let end = offset.checked_add(size).ok_or(IpcError::Overflow)?;
        if end > total {
            return Err(IpcError::BufferTooSmall);
        }
        // SAFETY: each slice covers a disjoint subrange of `backing`.
        let slice = unsafe { core::slice::from_raw_parts_mut(base.add(offset), size) };
        out.push(ChannelBacking {
            name: plan.name.as_str(),
            backing: slice,
            slot_count: plan.slot_count,
            slot_size: plan.slot_size,
        });
        offset = end;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use alloc::vec;

    use hv_ipc::init_ring;

    use super::*;

    const SLOT_COUNT: u32 = 8;
    const SLOT_SIZE: u32 = 2048;

    fn init_channel(backing: &mut [u8], name: &str) {
        init_ring(backing, name, SLOT_COUNT, SLOT_SIZE).expect("init");
    }

    #[test]
    fn e2e_relay_moves_payload_in_to_mid_to_out() {
        let mut in_to_mid = vec![0u8; 16424];
        let mut mid_to_out = vec![0u8; 16424];
        init_channel(&mut in_to_mid, "in_to_mid");
        init_channel(&mut mid_to_out, "mid_to_out");

        let mut in_channel = ChannelBacking {
            name: "in_to_mid",
            backing: &mut in_to_mid,
            slot_count: SLOT_COUNT,
            slot_size: SLOT_SIZE,
        };
        inject_inbound_payload(&mut in_channel, b"udp-payload-bytes").expect("inject");

        let mut channels = DatapathChannels {
            in_to_mid: ChannelBacking {
                name: "in_to_mid",
                backing: &mut in_to_mid,
                slot_count: SLOT_COUNT,
                slot_size: SLOT_SIZE,
            },
            mid_to_out: ChannelBacking {
                name: "mid_to_out",
                backing: &mut mid_to_out,
                slot_count: SLOT_COUNT,
                slot_size: SLOT_SIZE,
            },
        };
        let step = run_datapath_step(&mut channels).expect("relay");
        assert_eq!(step.in_to_mid_frames, 1);
        assert!(step.payload_bytes >= b"udp-payload-bytes".len() as u64);

        let mut out = [0u8; SLOT_SIZE as usize];
        let mut out_channel = ChannelBacking {
            name: "mid_to_out",
            backing: &mut mid_to_out,
            slot_count: SLOT_COUNT,
            slot_size: SLOT_SIZE,
        };
        let len = drain_outbound_payload(&mut out_channel, &mut out).expect("drain");
        assert!(len >= b"udp-payload-bytes".len());
        assert_eq!(&out[..b"udp-payload-bytes".len()], b"udp-payload-bytes");
    }
}
