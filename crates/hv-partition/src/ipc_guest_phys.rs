//! Deterministic guest-physical IPC mapping offsets.

use hv_config_model::intent::StaticIntentIR;
use hv_types::VmId;

use crate::PartitionError;

/// Guest-physical base for an IPC channel mapped above guest RAM.
///
/// Channels are placed sequentially after the partition private RAM region in
/// ascending channel-name order for determinism.
#[must_use]
pub fn ipc_guest_phys_base(
    intent: &StaticIntentIR,
    vm_id: VmId,
    channel_name: &str,
) -> Option<u64> {
    let partition = intent.partitions.iter().find(|part| part.vm_id == vm_id)?;
    let mut cursor = partition.memory_bytes;
    for channel in intent.ipc.iter() {
        if channel.producer != vm_id && channel.consumer != vm_id {
            continue;
        }
        if channel.name == channel_name {
            return Some(cursor);
        }
        cursor = cursor.checked_add(channel.shared_bytes)?;
    }
    None
}

/// Returns all IPC channels visible to a partition in deterministic order.
pub fn visible_ipc_channels(
    intent: &StaticIntentIR,
    vm_id: VmId,
) -> alloc::vec::Vec<&hv_config_model::intent::IpcIntent> {
    intent
        .ipc
        .iter()
        .filter(|channel| channel.producer == vm_id || channel.consumer == vm_id)
        .collect()
}

/// Validates that IPC guest phys bases fit for a partition.
pub fn validate_ipc_layout(intent: &StaticIntentIR, vm_id: VmId) -> Result<(), PartitionError> {
    let partition = intent
        .partitions
        .iter()
        .find(|part| part.vm_id == vm_id)
        .ok_or(PartitionError::UnknownPartition { vm_id: vm_id.raw() })?;
    let mut cursor = partition.memory_bytes;
    for channel in visible_ipc_channels(intent, vm_id) {
        cursor = cursor.checked_add(channel.shared_bytes).ok_or(PartitionError::Overflow)?;
    }
    let _ = cursor;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use hv_config_model::pipeline::compile_config;
    use hv_config_model::yaml::read_yaml_file;
    use hv_types::VmId;

    use super::*;

    #[test]
    fn ipc_bases_are_deterministic_for_qemu_config() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
        let raw = read_yaml_file(path).expect("read config");
        let compiled = compile_config(raw).expect("compile config");
        let intent = &compiled.intent;
        let vm2 = VmId::new(1);
        let in_to_mid = ipc_guest_phys_base(intent, vm2, "in_to_mid").expect("in_to_mid");
        let mid_to_out = ipc_guest_phys_base(intent, vm2, "mid_to_out").expect("mid_to_out");
        assert!(mid_to_out > in_to_mid);
        validate_ipc_layout(intent, vm2).expect("ipc layout");
    }
}
