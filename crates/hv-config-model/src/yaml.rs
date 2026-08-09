//! YAML parsing and canonical serialization.

use alloc::string::String;

use crate::error::ConfigError;
use crate::normalize::NormalizedConfig;
use crate::raw::RawConfig;

/// Reads and parses a YAML configuration file.
pub fn parse_yaml(input: &str) -> Result<RawConfig, ConfigError> {
    serde_yaml::from_str(input).map_err(|err| ConfigError::YamlParse(err.to_string()))
}

/// Reads a YAML configuration file from disk.
pub fn read_yaml_file(path: &str) -> Result<RawConfig, ConfigError> {
    let contents = std::fs::read_to_string(path).map_err(|err| ConfigError::Io(err.to_string()))?;
    parse_yaml(&contents)
}

/// Serializes a normalized configuration into canonical JSON for stable hashing.
pub fn canonicalize_normalized(config: &NormalizedConfig) -> String {
    canonical_json::to_json(config)
}

mod canonical_json {
    use alloc::format;
    use alloc::string::String;

    use crate::normalize::{
        DeviceKind, NormalizedBenchmark, NormalizedBoot, NormalizedConfig, NormalizedDevice,
        NormalizedIpc, NormalizedPartition, NormalizedQemu, NormalizedSecurity,
    };
    use crate::raw::{FaultPolicy, RequirementLevel, SmtPolicy, ThroughputMetric};

    pub fn to_json(config: &NormalizedConfig) -> String {
        let mut out = String::new();
        out.push('{');
        write_field(&mut out, "version", &config.version.to_string(), true);
        write_field(&mut out, "name", &json_string(&config.name), false);
        write_field(
            &mut out,
            "platform",
            &format!("{{\"requirements\":{}}}", requirements_json(&config.platform.requirements)),
            false,
        );
        write_field(&mut out, "partitions", &partitions_json(&config.partitions), false);
        write_field(&mut out, "ipc", &ipc_list_json(&config.ipc), false);
        write_field(&mut out, "security", &security_json(&config.security), false);
        write_field(&mut out, "benchmark", &benchmark_json(&config.benchmark), false);
        write_field(&mut out, "boot", &boot_json(&config.boot), false);
        write_field(&mut out, "qemu", &qemu_json(&config.qemu), false);
        out.push('}');
        out
    }

    fn requirements_json(req: &crate::normalize::NormalizedPlatformRequirements) -> String {
        format!(
            "{{\"arch\":\"x86_64\",\"vmx\":\"{}\",\"ept\":\"{}\",\"vtd\":\"{}\",\"min_physical_cores\":{},\"smt_policy\":\"{}\",\"min_ram_bytes\":{},\"interrupt_remapping\":\"{}\",\"x2apic\":\"{}\",\"invariant_tsc\":\"{}\",\"vpid\":\"{}\",\"vmx_preemption_timer\":\"{}\",\"nx\":\"{}\",\"page_sizes\":{}}}",
            req_level(req.vmx),
            req_level(req.ept),
            req_level(req.vtd),
            req.min_physical_cores,
            smt_policy(req.smt_policy),
            req.min_ram_bytes,
            req_level(req.interrupt_remapping),
            req_level(req.x2apic),
            req_level(req.invariant_tsc),
            req_level(req.vpid),
            req_level(req.vmx_preemption_timer),
            req_level(req.nx),
            page_sizes_json(&req.page_sizes),
        )
    }

    fn partitions_json(partitions: &[NormalizedPartition]) -> String {
        let items: Vec<String> = partitions.iter().map(partition_json).collect();
        format!("[{}]", items.join(","))
    }

    fn partition_json(partition: &NormalizedPartition) -> String {
        let devices: Vec<String> = partition.devices.iter().map(device_json).collect();
        format!(
            "{{\"vm_id\":{},\"name\":{},\"vcpus\":{},\"memory_bytes\":{},\"image\":{},\"devices\":[{}]}}",
            partition.vm_id.raw(),
            json_string(&partition.name),
            partition.vcpus,
            partition.memory_bytes,
            json_string(&partition.image),
            devices.join(","),
        )
    }

    fn device_json(device: &NormalizedDevice) -> String {
        format!(
            "{{\"kind\":\"{}\",\"bdf\":{}}}",
            device_kind(device.kind),
            json_string(&device.bdf.to_string()),
        )
    }

    fn ipc_list_json(channels: &[NormalizedIpc]) -> String {
        let items: Vec<String> = channels
            .iter()
            .map(|ipc| {
                format!(
                    "{{\"name\":{},\"producer\":{},\"consumer\":{},\"direction\":\"unidirectional\",\"slot_count\":{},\"slot_size\":{},\"queue_full_policy\":\"drop_tail\"}}",
                    json_string(&ipc.name),
                    ipc.producer.raw(),
                    ipc.consumer.raw(),
                    ipc.slot_count,
                    ipc.slot_size,
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }

    fn security_json(security: &NormalizedSecurity) -> String {
        format!(
            "{{\"require_config_hash\":{},\"ipc_corruption_policy\":\"{}\",\"ept_violation_policy\":\"{}\",\"iommu_fault_policy\":\"{}\"}}",
            security.require_config_hash,
            fault_policy(security.ipc_corruption_policy),
            fault_policy(security.ept_violation_policy),
            fault_policy(security.iommu_fault_policy),
        )
    }

    fn benchmark_json(benchmark: &NormalizedBenchmark) -> String {
        format!(
            "{{\"protocol\":{},\"frame_size\":{},\"payload_size\":{},\"throughput_metric\":\"{}\",\"warmup_seconds\":{},\"measurement_seconds\":{},\"runs\":{},\"max_loss_ppm\":{}}}",
            json_string(&benchmark.protocol),
            benchmark.frame_size,
            benchmark.payload_size,
            throughput_metric(benchmark.throughput_metric),
            benchmark.warmup_seconds,
            benchmark.measurement_seconds,
            benchmark.runs,
            benchmark.max_loss_ppm,
        )
    }

    fn boot_json(boot: &NormalizedBoot) -> String {
        format!(
            "{{\"loader\":{},\"hypervisor\":{},\"exit_boot_services\":{}}}",
            json_string(&boot.loader),
            json_string(&boot.hypervisor),
            boot.exit_boot_services,
        )
    }

    fn qemu_json(qemu: &NormalizedQemu) -> String {
        format!(
            "{{\"machine\":{},\"cpu\":{},\"smp\":{{\"cpus\":{},\"cores\":{},\"threads\":{}}},\"memory_bytes\":{},\"accel\":{},\"ovmf\":{}}}",
            json_string(&qemu.machine),
            json_string(&qemu.cpu),
            qemu.smp.cpus,
            qemu.smp.cores,
            qemu.smp.threads,
            qemu.memory_bytes,
            json_string(&qemu.accel),
            json_string(&qemu.ovmf),
        )
    }

    fn write_field(out: &mut String, key: &str, value: &str, first: bool) {
        if !first {
            out.push(',');
        }
        out.push('"');
        out.push_str(key);
        out.push_str("\":");
        out.push_str(value);
    }

    fn json_string(value: &str) -> String {
        format!("\"{}\"", escape_json(value))
    }

    fn escape_json(input: &str) -> String {
        let mut out = String::new();
        for ch in input.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                _ => out.push(ch),
            }
        }
        out
    }

    fn req_level(level: RequirementLevel) -> &'static str {
        match level {
            RequirementLevel::Required => "required",
            RequirementLevel::Preferred => "preferred",
            RequirementLevel::Optional => "optional",
            RequirementLevel::Disabled => "disabled",
        }
    }

    fn smt_policy(policy: SmtPolicy) -> &'static str {
        match policy {
            SmtPolicy::Disabled => "disabled",
            SmtPolicy::ExclusiveCore => "exclusive_core",
            SmtPolicy::SamePartitionSiblings => "same_partition_siblings",
            SmtPolicy::AllowCrossPartition => "allow_cross_partition",
        }
    }

    fn fault_policy(policy: FaultPolicy) -> &'static str {
        match policy {
            FaultPolicy::StopPartition => "stop_partition",
            FaultPolicy::FailStopSystem => "fail_stop_system",
        }
    }

    fn throughput_metric(metric: ThroughputMetric) -> &'static str {
        match metric {
            ThroughputMetric::UdpPayload => "udp_payload_bytes",
            ThroughputMetric::IpPacket => "ip_bytes",
            ThroughputMetric::EthernetFrame => "ethernet_bytes",
        }
    }

    fn device_kind(kind: DeviceKind) -> &'static str {
        match kind {
            DeviceKind::E1000 => "e1000",
        }
    }

    fn page_sizes_json(values: &[u64]) -> String {
        let items: Vec<String> = values.iter().map(|v| v.to_string()).collect();
        format!("[{}]", items.join(","))
    }
}
