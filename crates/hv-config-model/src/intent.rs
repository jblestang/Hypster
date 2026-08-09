//! Static intent intermediate representation.

use alloc::string::String;
use alloc::vec::Vec;

use hv_types::{IommuDomainId, VmId};

use crate::hash::ConfigHash;
use crate::normalize::{
    DeviceKind, NormalizedConfig, NormalizedIpc, NormalizedPartition, NormalizedQemu,
};
use crate::requirements::PlatformRequirements;

/// Static planning IR consumed by later allocation/resolution stages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticIntentIR {
    /// Configuration schema version.
    pub version: u32,
    /// Configuration name.
    pub name: String,
    /// SHA-256 hash of canonical configuration bytes.
    pub config_hash: ConfigHash,
    /// Platform requirements contract.
    pub platform_requirements: PlatformRequirements,
    /// Partition intents in deterministic order.
    pub partitions: Vec<PartitionIntent>,
    /// IPC intents.
    pub ipc: Vec<IpcIntent>,
    /// Boot artifact paths.
    pub boot: BootIntent,
    /// QEMU launch intent.
    pub qemu: QemuIntent,
    /// Benchmark parameters.
    pub benchmark: BenchmarkIntent,
}

/// Partition planning intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartitionIntent {
    /// VM identifier.
    pub vm_id: VmId,
    /// Partition name.
    pub name: String,
    /// Number of vCPUs.
    pub vcpus: u32,
    /// Private RAM bytes.
    pub memory_bytes: u64,
    /// Guest image path.
    pub image: String,
    /// Assigned devices.
    pub devices: Vec<DeviceIntent>,
    /// Dedicated IOMMU domain ID.
    pub iommu_domain: IommuDomainId,
}

/// Device assignment intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceIntent {
    /// Device kind.
    pub kind: DeviceKind,
    /// Canonical BDF string.
    pub bdf: String,
}

/// IPC planning intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpcIntent {
    /// Channel name.
    pub name: String,
    /// Producer VM ID.
    pub producer: VmId,
    /// Consumer VM ID.
    pub consumer: VmId,
    /// Slot count.
    pub slot_count: u32,
    /// Slot size in bytes.
    pub slot_size: u32,
    /// Total shared memory bytes for the channel ring.
    pub shared_bytes: u64,
}

/// Boot artifact intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootIntent {
    /// Loader path.
    pub loader: String,
    /// Hypervisor path.
    pub hypervisor: String,
    /// Whether ExitBootServices is required.
    pub exit_boot_services: bool,
}

/// QEMU launch intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QemuIntent {
    /// Underlying normalized QEMU plan.
    pub plan: NormalizedQemu,
}

/// Benchmark intent copied from normalized configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkIntent {
    /// Protocol identifier.
    pub protocol: String,
    /// Frame size in bytes.
    pub frame_size: u32,
    /// Payload size in bytes.
    pub payload_size: u32,
    /// Warmup duration in seconds.
    pub warmup_seconds: u32,
    /// Measurement duration in seconds.
    pub measurement_seconds: u32,
    /// Number of runs.
    pub runs: u32,
    /// Maximum acceptable loss in parts per million.
    pub max_loss_ppm: u32,
}

impl StaticIntentIR {
    /// Builds the static intent IR from normalized configuration and hash.
    pub fn from_normalized(config: &NormalizedConfig, config_hash: ConfigHash) -> Result<Self, crate::error::ConfigError> {
        let platform_requirements = PlatformRequirements::from_normalized(config);
        let partitions = build_partition_intents(&config.partitions)?;
        let ipc = build_ipc_intents(&config.ipc)?;

        Ok(Self {
            version: config.version,
            name: config.name.clone(),
            config_hash,
            platform_requirements,
            partitions,
            ipc,
            boot: BootIntent {
                loader: config.boot.loader.clone(),
                hypervisor: config.boot.hypervisor.clone(),
                exit_boot_services: config.boot.exit_boot_services,
            },
            qemu: QemuIntent {
                plan: config.qemu.clone(),
            },
            benchmark: BenchmarkIntent {
                protocol: config.benchmark.protocol.clone(),
                frame_size: config.benchmark.frame_size,
                payload_size: config.benchmark.payload_size,
                warmup_seconds: config.benchmark.warmup_seconds,
                measurement_seconds: config.benchmark.measurement_seconds,
                runs: config.benchmark.runs,
                max_loss_ppm: config.benchmark.max_loss_ppm,
            },
        })
    }

    /// Returns partition intents in VM ID order.
    pub fn partitions(&self) -> &[PartitionIntent] {
        &self.partitions
    }
}

fn build_partition_intents(partitions: &[NormalizedPartition]) -> Result<Vec<PartitionIntent>, crate::error::ConfigError> {
    let mut out = Vec::with_capacity(partitions.len());
    for partition in partitions {
        let iommu_domain = IommuDomainId::new((partition.vm_id.raw() + 1) as u16);
        let devices = partition
            .devices
            .iter()
            .map(|device| DeviceIntent {
                kind: device.kind,
                bdf: device.bdf.to_string(),
            })
            .collect();

        out.push(PartitionIntent {
            vm_id: partition.vm_id,
            name: partition.name.clone(),
            vcpus: partition.vcpus,
            memory_bytes: partition.memory_bytes,
            image: partition.image.clone(),
            devices,
            iommu_domain,
        });
    }
    Ok(out)
}

fn build_ipc_intents(channels: &[NormalizedIpc]) -> Result<Vec<IpcIntent>, crate::error::ConfigError> {
    let mut out = Vec::with_capacity(channels.len());
    for channel in channels {
        let slot_bytes = u64::from(channel.slot_size);
        let count = u64::from(channel.slot_count);
        let shared_bytes = slot_bytes
            .checked_mul(count)
            .ok_or_else(|| crate::error::ConfigError::invalid("ipc[]", "shared memory size overflow"))?;
        out.push(IpcIntent {
            name: channel.name.clone(),
            producer: channel.producer,
            consumer: channel.consumer,
            slot_count: channel.slot_count,
            slot_size: channel.slot_size,
            shared_bytes,
        });
    }
    Ok(out)
}
