//! Raw configuration deserialized from YAML.

use alloc::string::String;
use alloc::vec::Vec;

#[cfg(feature = "std")]
use serde::Deserialize;

/// Top-level raw configuration as deserialized from YAML.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawConfig {
    /// Schema version.
    pub version: u32,
    /// Human-readable configuration name.
    pub name: String,
    /// Platform-wide settings.
    pub platform: RawPlatform,
    /// Partition definitions in declaration order.
    pub partitions: Vec<RawPartition>,
    /// IPC channel definitions.
    pub ipc: Vec<RawIpc>,
    /// Security policies.
    pub security: RawSecurity,
    /// Performance and benchmark settings.
    pub performance: RawPerformance,
    /// Boot artifact paths.
    pub boot: RawBoot,
    /// QEMU launch parameters derived from the same source of truth.
    pub qemu: RawQemu,
}

/// Platform section of raw configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawPlatform {
    /// Hardware and feature requirements.
    pub requirements: RawPlatformRequirements,
}

/// Platform requirements in raw form.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawPlatformRequirements {
    /// Target architecture string.
    pub arch: String,
    /// VMX requirement level.
    pub vmx: RequirementLevel,
    /// EPT requirement level.
    pub ept: RequirementLevel,
    /// VT-d requirement level.
    pub vtd: RequirementLevel,
    /// Minimum number of physical cores.
    pub min_physical_cores: u32,
    /// Simultaneous multithreading policy.
    pub smt_policy: SmtPolicy,
    /// Minimum host RAM in gibibytes.
    pub min_ram_gib: u64,
    /// Interrupt remapping requirement level.
    pub interrupt_remapping: RequirementLevel,
    /// x2APIC requirement level.
    pub x2apic: RequirementLevel,
    /// Invariant TSC requirement level.
    pub invariant_tsc: RequirementLevel,
    /// VPID requirement level.
    pub vpid: RequirementLevel,
    /// VMX preemption timer requirement level.
    pub vmx_preemption_timer: RequirementLevel,
    /// NX bit requirement level.
    pub nx: RequirementLevel,
    /// Supported guest page sizes in bytes.
    pub page_sizes: Vec<u64>,
}

/// Requirement strictness for optional platform features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum RequirementLevel {
    /// Feature must be present or boot is refused.
    Required,
    /// Feature is preferred but absence only emits warnings in later phases.
    Preferred,
    /// Feature is optional.
    Optional,
    /// Feature must not be used.
    Disabled,
}

/// SMT placement policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum SmtPolicy {
    /// SMT disabled platform-wide.
    Disabled,
    /// Each physical core belongs to at most one partition.
    ExclusiveCore,
    /// SMT siblings must remain in the same partition.
    SamePartitionSiblings,
    /// Cross-partition SMT allowed with explicit warnings.
    AllowCrossPartition,
}

/// Raw partition definition.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawPartition {
    /// Stable partition name.
    pub name: String,
    /// Number of vCPUs assigned to the partition.
    pub vcpus: u32,
    /// Private RAM in gibibytes.
    pub memory_gib: u64,
    /// Guest ELF path relative to workspace root.
    pub image: String,
    /// PCI devices assigned to the partition.
    #[serde(default)]
    pub devices: Vec<RawDevice>,
    /// Optional software stack hints.
    #[serde(default)]
    pub stack: Option<RawStack>,
}

/// PCI device assignment in raw configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawDevice {
    /// Device kind string.
    pub kind: String,
    /// PCI BDF in `SSSS:BB:DD.F` form.
    pub bdf: String,
}

/// Optional guest stack metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawStack {
    /// Whether smoltcp is used in the guest.
    #[serde(default)]
    pub smoltcp: bool,
    /// Network protocol used for benchmarking.
    #[serde(default)]
    pub protocol: Option<String>,
}

/// Raw IPC channel definition.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawIpc {
    /// Channel name.
    pub name: String,
    /// Producer partition name.
    pub producer: String,
    /// Consumer partition name.
    pub consumer: String,
    /// Direction policy.
    pub direction: IpcDirection,
    /// Number of SPSC slots.
    pub slot_count: u32,
    /// Slot payload size in bytes.
    pub slot_size: u32,
    /// Queue-full handling policy.
    pub queue_full_policy: QueueFullPolicy,
}

/// IPC direction policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum IpcDirection {
    /// Single producer to single consumer only.
    Unidirectional,
}

/// Queue-full policy for IPC channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum QueueFullPolicy {
    /// Drop the newest item and count the event.
    DropTail,
}

/// Security policies in raw configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawSecurity {
    /// Whether boot must verify the configuration hash.
    pub require_config_hash: bool,
    /// IPC corruption handling policy.
    pub ipc_corruption_policy: FaultPolicy,
    /// EPT violation handling policy.
    pub ept_violation_policy: FaultPolicy,
    /// IOMMU fault handling policy.
    pub iommu_fault_policy: FaultPolicy,
}

/// Fault containment policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum FaultPolicy {
    /// Stop the offending partition.
    StopPartition,
    /// Stop the entire system.
    FailStopSystem,
}

/// Performance and benchmark configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawPerformance {
    /// Benchmark definition.
    pub benchmark: RawBenchmark,
}

/// Benchmark parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawBenchmark {
    /// Protocol identifier.
    pub protocol: String,
    /// Ethernet frame size including L2 header.
    pub frame_size: u32,
    /// UDP payload size for throughput accounting.
    pub payload_size: u32,
    /// Throughput metric definition.
    pub throughput_metric: ThroughputMetric,
    /// Warmup duration in seconds.
    pub warmup_seconds: u32,
    /// Measurement duration in seconds.
    pub measurement_seconds: u32,
    /// Number of benchmark runs.
    pub runs: u32,
    /// Maximum acceptable loss ratio.
    pub max_loss_ratio: String,
}

/// Official throughput metric definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "snake_case"))]
pub enum ThroughputMetric {
    /// Count only UDP payload bytes received at OUT.
    #[cfg_attr(feature = "std", serde(rename = "udp_payload_bytes"))]
    UdpPayload,
    /// Count IP packet bytes.
    #[cfg_attr(feature = "std", serde(rename = "ip_bytes"))]
    IpPacket,
    /// Count Ethernet L2 bytes.
    #[cfg_attr(feature = "std", serde(rename = "ethernet_bytes"))]
    EthernetFrame,
}

/// Boot artifact paths.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawBoot {
    /// UEFI loader path.
    pub loader: String,
    /// Hypervisor image path.
    pub hypervisor: String,
    /// Whether ExitBootServices is required.
    pub exit_boot_services: bool,
}

/// QEMU launch configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawQemu {
    /// Machine type.
    pub machine: String,
    /// CPU model passed to QEMU.
    pub cpu: String,
    /// SMP topology.
    pub smp: RawSmp,
    /// Guest memory in mebibytes.
    pub memory_mib: u64,
    /// Accelerator backend.
    pub accel: String,
    /// OVMF firmware path.
    pub ovmf: String,
    /// Network backends.
    pub netdevs: Vec<RawNetdev>,
    /// Device bindings for QEMU command line generation.
    pub devices: Vec<RawQemuDevice>,
}

/// QEMU SMP topology.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawSmp {
    /// Total vCPU count presented to QEMU.
    pub cpus: u32,
    /// Number of cores.
    pub cores: u32,
    /// Threads per core.
    pub threads: u32,
}

/// QEMU netdev definition.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawNetdev {
    /// Netdev identifier.
    pub id: String,
    /// Netdev kind.
    pub kind: String,
    /// Optional host forwarding rule.
    pub hostfwd: Option<String>,
}

/// QEMU device binding.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct RawQemuDevice {
    /// Device kind.
    pub kind: String,
    /// PCI BDF expected in the guest.
    pub bdf: String,
    /// Netdev identifier for NIC devices.
    pub netdev: Option<String>,
}
