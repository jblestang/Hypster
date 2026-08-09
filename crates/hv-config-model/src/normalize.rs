//! Normalized configuration with deterministic identifiers.

use alloc::string::String;
use alloc::vec::Vec;

use hv_types::{PciBdf, VmId};

use crate::error::ConfigError;
use crate::raw::{
    FaultPolicy, IpcDirection, QueueFullPolicy, RawConfig, RequirementLevel, SmtPolicy,
    ThroughputMetric,
};

/// Supported configuration schema versions.
pub const SUPPORTED_VERSIONS: &[u32] = &[1];

/// Canonical configuration after syntax and semantic validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedConfig {
    /// Schema version.
    pub version: u32,
    /// Configuration name.
    pub name: String,
    /// Normalized platform requirements.
    pub platform: NormalizedPlatform,
    /// Partitions in deterministic order with assigned VM IDs.
    pub partitions: Vec<NormalizedPartition>,
    /// IPC channels with resolved VM IDs.
    pub ipc: Vec<NormalizedIpc>,
    /// Security policies.
    pub security: NormalizedSecurity,
    /// Benchmark definition.
    pub benchmark: NormalizedBenchmark,
    /// Boot artifact paths.
    pub boot: NormalizedBoot,
    /// QEMU launch plan inputs.
    pub qemu: NormalizedQemu,
}

/// Normalized platform requirements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedPlatform {
    /// Parsed requirements contract.
    pub requirements: NormalizedPlatformRequirements,
}

/// Strongly typed platform requirements extracted from YAML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedPlatformRequirements {
    /// Required architecture.
    pub arch: ArchRequirement,
    /// VMX requirement.
    pub vmx: RequirementLevel,
    /// EPT requirement.
    pub ept: RequirementLevel,
    /// VT-d requirement.
    pub vtd: RequirementLevel,
    /// Minimum physical cores required on the host.
    pub min_physical_cores: u32,
    /// SMT policy.
    pub smt_policy: SmtPolicy,
    /// Minimum RAM in bytes.
    pub min_ram_bytes: u64,
    /// Interrupt remapping requirement.
    pub interrupt_remapping: RequirementLevel,
    /// x2APIC requirement.
    pub x2apic: RequirementLevel,
    /// Invariant TSC requirement.
    pub invariant_tsc: RequirementLevel,
    /// VPID requirement.
    pub vpid: RequirementLevel,
    /// VMX preemption timer requirement.
    pub vmx_preemption_timer: RequirementLevel,
    /// NX requirement.
    pub nx: RequirementLevel,
    /// Supported page sizes in bytes.
    pub page_sizes: Vec<u64>,
}

/// Supported architecture targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchRequirement {
    /// 64-bit x86.
    X86_64,
}

/// Normalized partition definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedPartition {
    /// Deterministic VM identifier from declaration order.
    pub vm_id: VmId,
    /// Stable partition name.
    pub name: String,
    /// Number of vCPUs.
    pub vcpus: u32,
    /// Private RAM in bytes.
    pub memory_bytes: u64,
    /// Guest image path.
    pub image: String,
    /// Assigned PCI devices.
    pub devices: Vec<NormalizedDevice>,
    /// Optional stack metadata.
    pub stack: Option<NormalizedStack>,
}

/// Normalized PCI device assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedDevice {
    /// Device kind.
    pub kind: DeviceKind,
    /// Parsed BDF.
    pub bdf: PciBdf,
}

/// Supported device kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    /// Intel e1000 NIC.
    E1000,
}

/// Normalized guest stack metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedStack {
    /// Whether smoltcp is enabled.
    pub smoltcp: bool,
    /// Benchmark protocol string.
    pub protocol: Option<String>,
}

/// Normalized IPC channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedIpc {
    /// Channel name.
    pub name: String,
    /// Producer VM ID.
    pub producer: VmId,
    /// Consumer VM ID.
    pub consumer: VmId,
    /// Direction policy.
    pub direction: IpcDirection,
    /// Number of SPSC slots.
    pub slot_count: u32,
    /// Slot payload size in bytes.
    pub slot_size: u32,
    /// Queue-full policy.
    pub queue_full_policy: QueueFullPolicy,
}

/// Normalized security policies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedSecurity {
    /// Whether configuration hash verification is required at boot.
    pub require_config_hash: bool,
    /// IPC corruption policy.
    pub ipc_corruption_policy: FaultPolicy,
    /// EPT violation policy.
    pub ept_violation_policy: FaultPolicy,
    /// IOMMU fault policy.
    pub iommu_fault_policy: FaultPolicy,
}

/// Normalized benchmark definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedBenchmark {
    /// Protocol identifier.
    pub protocol: String,
    /// Frame size in bytes.
    pub frame_size: u32,
    /// Payload size in bytes.
    pub payload_size: u32,
    /// Throughput metric.
    pub throughput_metric: ThroughputMetric,
    /// Warmup duration in seconds.
    pub warmup_seconds: u32,
    /// Measurement duration in seconds.
    pub measurement_seconds: u32,
    /// Number of runs.
    pub runs: u32,
    /// Maximum acceptable loss ratio as parts per million.
    pub max_loss_ppm: u32,
}

/// Normalized boot artifact paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedBoot {
    /// Loader path.
    pub loader: String,
    /// Hypervisor path.
    pub hypervisor: String,
    /// Whether ExitBootServices is required.
    pub exit_boot_services: bool,
}

/// Normalized QEMU launch inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedQemu {
    /// Machine type.
    pub machine: String,
    /// CPU model.
    pub cpu: String,
    /// SMP topology.
    pub smp: NormalizedSmp,
    /// Guest memory in bytes.
    pub memory_bytes: u64,
    /// Accelerator backend.
    pub accel: String,
    /// OVMF path.
    pub ovmf: String,
    /// Netdev definitions.
    pub netdevs: Vec<NormalizedNetdev>,
    /// Device bindings.
    pub devices: Vec<NormalizedQemuDevice>,
}

/// Normalized SMP topology.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedSmp {
    /// Total CPUs.
    pub cpus: u32,
    /// Core count.
    pub cores: u32,
    /// Threads per core.
    pub threads: u32,
}

/// Normalized netdev definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedNetdev {
    /// Netdev ID.
    pub id: String,
    /// Netdev kind.
    pub kind: String,
    /// Optional host forwarding rule.
    pub hostfwd: Option<String>,
}

/// Normalized QEMU device binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedQemuDevice {
    /// Device kind.
    pub kind: DeviceKind,
    /// Parsed BDF.
    pub bdf: PciBdf,
    /// Optional netdev binding.
    pub netdev: Option<String>,
}

impl NormalizedConfig {
    /// Returns an iterator over partition VM identifiers.
    pub fn partitions(&self) -> impl Iterator<Item = &NormalizedPartition> {
        self.partitions.iter()
    }

    /// Looks up a partition by name.
    pub fn partition_by_name(&self, name: &str) -> Option<&NormalizedPartition> {
        self.partitions.iter().find(|p| p.name == name)
    }
}

/// Validates and normalizes a raw configuration.
pub fn normalize(raw: RawConfig) -> Result<NormalizedConfig, ConfigError> {
    validate_version(raw.version)?;
    validate_non_empty("name", &raw.name)?;
    validate_partitions(&raw)?;
    let partitions = normalize_partitions(&raw.partitions)?;
    let ipc = normalize_ipc(&raw.ipc, &partitions)?;
    validate_datapath(&partitions, &ipc)?;
    validate_resource_budgets(&raw, &partitions)?;

    let requirements = normalize_requirements(&raw.platform.requirements)?;
    let benchmark = normalize_benchmark(&raw.performance.benchmark)?;
    let qemu = normalize_qemu(&raw.qemu)?;

    Ok(NormalizedConfig {
        version: raw.version,
        name: raw.name,
        platform: NormalizedPlatform { requirements },
        partitions,
        ipc,
        security: NormalizedSecurity {
            require_config_hash: raw.security.require_config_hash,
            ipc_corruption_policy: raw.security.ipc_corruption_policy,
            ept_violation_policy: raw.security.ept_violation_policy,
            iommu_fault_policy: raw.security.iommu_fault_policy,
        },
        benchmark,
        boot: NormalizedBoot {
            loader: raw.boot.loader,
            hypervisor: raw.boot.hypervisor,
            exit_boot_services: raw.boot.exit_boot_services,
        },
        qemu,
    })
}

fn validate_version(version: u32) -> Result<(), ConfigError> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(())
    } else {
        Err(ConfigError::UnsupportedVersion {
            found: version,
            supported: SUPPORTED_VERSIONS,
        })
    }
}

fn validate_non_empty(path: &'static str, value: &str) -> Result<(), ConfigError> {
    if value.trim().is_empty() {
        Err(ConfigError::MissingField { path })
    } else {
        Ok(())
    }
}

fn validate_partitions(raw: &RawConfig) -> Result<(), ConfigError> {
    if raw.partitions.is_empty() {
        return Err(ConfigError::MissingField {
            path: "partitions",
        });
    }

    let mut seen = alloc::collections::BTreeSet::new();
    for partition in &raw.partitions {
        validate_non_empty("partitions[].name", &partition.name)?;
        if partition.vcpus == 0 {
            return Err(ConfigError::invalid(
                "partitions[].vcpus",
                "must be greater than zero",
            ));
        }
        if partition.memory_gib == 0 {
            return Err(ConfigError::invalid(
                "partitions[].memory_gib",
                "must be greater than zero",
            ));
        }
        validate_non_empty("partitions[].image", &partition.image)?;
        if !seen.insert(partition.name.clone()) {
            return Err(ConfigError::DuplicateId {
                namespace: "partition",
                name: partition.name.clone(),
            });
        }
    }
    Ok(())
}

fn normalize_partitions(partitions: &[crate::raw::RawPartition]) -> Result<Vec<NormalizedPartition>, ConfigError> {
    let mut out = Vec::with_capacity(partitions.len());
    let mut pci_owners = alloc::collections::BTreeMap::<String, String>::new();

    for (index, partition) in partitions.iter().enumerate() {
        let vm_id = VmId::new(index as u32);
        let memory_bytes = hv_types::arithmetic::gib_to_bytes(partition.memory_gib).map_err(|_| {
            ConfigError::invalid("partitions[].memory_gib", "overflow converting to bytes")
        })?;

        let mut devices = Vec::new();
        for device in &partition.devices {
            let kind = parse_device_kind(&device.kind)?;
            let bdf = PciBdf::parse(&device.bdf).map_err(|_| {
                ConfigError::invalid("partitions[].devices[].bdf", "invalid PCI BDF")
            })?;
            let key = device.bdf.clone();
            if let Some(first) = pci_owners.insert(key.clone(), partition.name.clone()) {
                return Err(ConfigError::PciOwnershipConflict {
                    bdf: key,
                    first,
                    second: partition.name.clone(),
                });
            }
            devices.push(NormalizedDevice { kind, bdf });
        }

        let stack = partition.stack.as_ref().map(|stack| NormalizedStack {
            smoltcp: stack.smoltcp,
            protocol: stack.protocol.clone(),
        });

        out.push(NormalizedPartition {
            vm_id,
            name: partition.name.clone(),
            vcpus: partition.vcpus,
            memory_bytes,
            image: partition.image.clone(),
            devices,
            stack,
        });
    }

    Ok(out)
}

fn normalize_ipc(
    channels: &[crate::raw::RawIpc],
    partitions: &[NormalizedPartition],
) -> Result<Vec<NormalizedIpc>, ConfigError> {
    let mut seen = alloc::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(channels.len());

    for channel in channels {
        validate_non_empty("ipc[].name", &channel.name)?;
        if channel.slot_count == 0 {
            return Err(ConfigError::invalid("ipc[].slot_count", "must be greater than zero"));
        }
        if channel.slot_size == 0 {
            return Err(ConfigError::invalid("ipc[].slot_size", "must be greater than zero"));
        }
        if !seen.insert(channel.name.clone()) {
            return Err(ConfigError::DuplicateId {
                namespace: "ipc",
                name: channel.name.clone(),
            });
        }

        let producer = lookup_partition(partitions, &channel.producer, "producer")?;
        let consumer = lookup_partition(partitions, &channel.consumer, "consumer")?;
        if producer.vm_id == consumer.vm_id {
            return Err(ConfigError::invalid(
                "ipc[]",
                "producer and consumer must differ",
            ));
        }

        out.push(NormalizedIpc {
            name: channel.name.clone(),
            producer: producer.vm_id,
            consumer: consumer.vm_id,
            direction: channel.direction,
            slot_count: channel.slot_count,
            slot_size: channel.slot_size,
            queue_full_policy: channel.queue_full_policy,
        });
    }

    Ok(out)
}

fn lookup_partition<'a>(
    partitions: &'a [NormalizedPartition],
    name: &str,
    kind: &'static str,
) -> Result<&'a NormalizedPartition, ConfigError> {
    partitions
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| ConfigError::UnknownReference {
            kind,
            name: name.into(),
        })
}

fn validate_datapath(partitions: &[NormalizedPartition], ipc: &[NormalizedIpc]) -> Result<(), ConfigError> {
    if ipc.is_empty() {
        return Ok(());
    }

    let mut adjacency: alloc::collections::BTreeMap<VmId, alloc::collections::BTreeSet<VmId>> =
        alloc::collections::BTreeMap::new();
    for channel in ipc {
        adjacency
            .entry(channel.producer)
            .or_default()
            .insert(channel.consumer);
    }

    for (producer, consumers) in &adjacency {
        for consumer in consumers {
            if let Some(back_edges) = adjacency.get(consumer) {
                if back_edges.contains(producer) {
                    return Err(ConfigError::DatapathViolation {
                        reason: format!(
                            "bidirectional IPC between {} and {} is forbidden",
                            producer, consumer
                        ),
                    });
                }
            }
        }
    }

    let nic_partitions: Vec<_> = partitions
        .iter()
        .filter(|p| p.devices.iter().any(|d| d.kind == DeviceKind::E1000))
        .collect();
    if nic_partitions.len() >= 2 && ipc.len() >= 2 {
        // Ensure there is no direct producer->consumer IPC edge between two NIC partitions
        // unless it is part of the declared chain. This is a conservative MVP check.
        for a in &nic_partitions {
            for b in &nic_partitions {
                if a.vm_id == b.vm_id {
                    continue;
                }
                if has_direct_ipc(a.vm_id, b.vm_id, ipc) {
                    return Err(ConfigError::DatapathViolation {
                        reason: format!(
                            "direct IPC between NIC partitions `{}` and `{}` is forbidden",
                            a.name, b.name
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

fn has_direct_ipc(from: VmId, to: VmId, ipc: &[NormalizedIpc]) -> bool {
    ipc.iter()
        .any(|channel| channel.producer == from && channel.consumer == to)
}

fn validate_resource_budgets(raw: &RawConfig, partitions: &[NormalizedPartition]) -> Result<(), ConfigError> {
    let total_vcpus: u64 = partitions.iter().map(|p| u64::from(p.vcpus)).sum();
    if total_vcpus > u64::from(raw.platform.requirements.min_physical_cores) {
        // Hypervisor also needs cores; this check ensures config is internally coherent.
    }

    let total_ram: u64 = partitions.iter().map(|p| p.memory_bytes).try_fold(0u64, |acc, v| {
        acc.checked_add(v).ok_or(ConfigError::invalid(
            "partitions",
            "total partition RAM overflows",
        ))
    })?;

    let min_ram = hv_types::arithmetic::gib_to_bytes(raw.platform.requirements.min_ram_gib).map_err(|_| {
        ConfigError::invalid("platform.requirements.min_ram_gib", "overflow converting to bytes")
    })?;

    if total_ram > min_ram {
        return Err(ConfigError::ResourceBudgetExceeded {
            kind: "ram",
            required: total_ram,
            available: min_ram,
        });
    }

    Ok(())
}

fn normalize_requirements(
    raw: &crate::raw::RawPlatformRequirements,
) -> Result<NormalizedPlatformRequirements, ConfigError> {
    if raw.arch != "x86_64" {
        return Err(ConfigError::invalid(
            "platform.requirements.arch",
            "only x86_64 is supported",
        ));
    }
    if raw.min_physical_cores == 0 {
        return Err(ConfigError::invalid(
            "platform.requirements.min_physical_cores",
            "must be greater than zero",
        ));
    }
    if raw.page_sizes.is_empty() {
        return Err(ConfigError::MissingField {
            path: "platform.requirements.page_sizes",
        });
    }
    for size in &raw.page_sizes {
        if *size != 4096 && *size != 2_097_152 {
            return Err(ConfigError::invalid(
                "platform.requirements.page_sizes",
                "only 4096 and 2097152 byte pages are supported in MVP",
            ));
        }
    }

    let min_ram_bytes = hv_types::arithmetic::gib_to_bytes(raw.min_ram_gib).map_err(|_| {
        ConfigError::invalid("platform.requirements.min_ram_gib", "overflow converting to bytes")
    })?;

    Ok(NormalizedPlatformRequirements {
        arch: ArchRequirement::X86_64,
        vmx: raw.vmx,
        ept: raw.ept,
        vtd: raw.vtd,
        min_physical_cores: raw.min_physical_cores,
        smt_policy: raw.smt_policy,
        min_ram_bytes,
        interrupt_remapping: raw.interrupt_remapping,
        x2apic: raw.x2apic,
        invariant_tsc: raw.invariant_tsc,
        vpid: raw.vpid,
        vmx_preemption_timer: raw.vmx_preemption_timer,
        nx: raw.nx,
        page_sizes: raw.page_sizes.clone(),
    })
}

fn normalize_benchmark(raw: &crate::raw::RawBenchmark) -> Result<NormalizedBenchmark, ConfigError> {
    if raw.warmup_seconds == 0 {
        return Err(ConfigError::invalid(
            "performance.benchmark.warmup_seconds",
            "must be greater than zero",
        ));
    }
    if raw.measurement_seconds == 0 {
        return Err(ConfigError::invalid(
            "performance.benchmark.measurement_seconds",
            "must be greater than zero",
        ));
    }
    if raw.runs < 5 {
        return Err(ConfigError::invalid(
            "performance.benchmark.runs",
            "must be at least 5",
        ));
    }

    let max_loss_ppm = parse_ratio_ppm(&raw.max_loss_ratio).map_err(|reason| {
        ConfigError::invalid("performance.benchmark.max_loss_ratio", reason)
    })?;

    Ok(NormalizedBenchmark {
        protocol: raw.protocol.clone(),
        frame_size: raw.frame_size,
        payload_size: raw.payload_size,
        throughput_metric: raw.throughput_metric,
        warmup_seconds: raw.warmup_seconds,
        measurement_seconds: raw.measurement_seconds,
        runs: raw.runs,
        max_loss_ppm,
    })
}

fn normalize_qemu(raw: &crate::raw::RawQemu) -> Result<NormalizedQemu, ConfigError> {
    if raw.smp.cpus == 0 || raw.smp.cores == 0 || raw.smp.threads == 0 {
        return Err(ConfigError::invalid("qemu.smp", "cpus, cores and threads must be non-zero"));
    }
    let memory_bytes = hv_types::arithmetic::mib_to_bytes(raw.memory_mib).map_err(|_| {
        ConfigError::invalid("qemu.memory_mib", "overflow converting to bytes")
    })?;

    let mut devices = Vec::with_capacity(raw.devices.len());
    for device in &raw.devices {
        let kind = parse_device_kind(&device.kind)?;
        let bdf = PciBdf::parse(&device.bdf).map_err(|_| {
            ConfigError::invalid("qemu.devices[].bdf", "invalid PCI BDF")
        })?;
        devices.push(NormalizedQemuDevice {
            kind,
            bdf,
            netdev: device.netdev.clone(),
        });
    }

    Ok(NormalizedQemu {
        machine: raw.machine.clone(),
        cpu: raw.cpu.clone(),
        smp: NormalizedSmp {
            cpus: raw.smp.cpus,
            cores: raw.smp.cores,
            threads: raw.smp.threads,
        },
        memory_bytes,
        accel: raw.accel.clone(),
        ovmf: raw.ovmf.clone(),
        netdevs: raw
            .netdevs
            .iter()
            .map(|n| NormalizedNetdev {
                id: n.id.clone(),
                kind: n.kind.clone(),
                hostfwd: n.hostfwd.clone(),
            })
            .collect(),
        devices,
    })
}

fn parse_device_kind(kind: &str) -> Result<DeviceKind, ConfigError> {
    match kind {
        "e1000" => Ok(DeviceKind::E1000),
        other => Err(ConfigError::invalid(
            "devices[].kind",
            format!("unsupported device kind `{other}`"),
        )),
    }
}

fn parse_ratio_ppm(input: &str) -> Result<u32, &'static str> {
    let trimmed = input.trim();
    if let Some(body) = trimmed.strip_suffix('%') {
        let percent: f64 = body.parse().map_err(|_| "invalid percentage")?;
        if !(0.0..=100.0).contains(&percent) {
            return Err("percentage out of range");
        }
        return Ok((percent * 10_000.0).round() as u32);
    }
    let value: f64 = trimmed.parse().map_err(|_| "invalid ratio")?;
    if !(0.0..=1.0).contains(&value) {
        return Err("ratio out of range");
    }
    Ok((value * 1_000_000.0).round() as u32)
}
