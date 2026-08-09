//! Generated artifact bundle from compiled configuration.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::intent::StaticIntentIR;
use crate::pipeline::CompiledConfig;

/// Named generated artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    /// Relative output path.
    pub path: String,
    /// File contents.
    pub contents: String,
}

/// Full artifact bundle produced by the config compiler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedArtifacts {
    /// Generated files.
    pub files: Vec<Artifact>,
}

impl GeneratedArtifacts {
    /// Generates all compiler artifacts from a compiled configuration.
    #[must_use]
    pub fn from_compiled(compiled: &CompiledConfig) -> Self {
        let intent = &compiled.intent;
        let hash = compiled.validated.config_hash.to_hex();
        let mut files = vec![
            Artifact {
                path: "config.sha256".into(),
                contents: format!("{hash}\n"),
            },
            Artifact {
                path: "platform-requirements.txt".into(),
                contents: render_platform_requirements(intent),
            },
            Artifact {
                path: "validated-platform.txt".into(),
                contents: render_validated_platform_placeholder(intent),
            },
            Artifact {
                path: "static-platform.rs".into(),
                contents: render_static_platform_rs(intent),
            },
            Artifact {
                path: "memory-map.txt".into(),
                contents: render_memory_map(intent),
            },
            Artifact {
                path: "cpu-topology.txt".into(),
                contents: render_cpu_topology(intent),
            },
            Artifact {
                path: "core-ownership.txt".into(),
                contents: render_core_ownership(intent),
            },
            Artifact {
                path: "ept-map.txt".into(),
                contents: render_ept_map(intent),
            },
            Artifact {
                path: "vtd-map.txt".into(),
                contents: render_vtd_map(intent),
            },
            Artifact {
                path: "pci-map.txt".into(),
                contents: render_pci_map(intent),
            },
            Artifact {
                path: "ipc-map.txt".into(),
                contents: render_ipc_map(intent),
            },
            Artifact {
                path: "irq-map.txt".into(),
                contents: render_irq_map(intent),
            },
            Artifact {
                path: "qemu-args.txt".into(),
                contents: render_qemu_args(intent),
            },
            Artifact {
                path: "boot-layout.txt".into(),
                contents: render_boot_layout(intent),
            },
            Artifact {
                path: "boot-manifest.txt".into(),
                contents: render_boot_manifest(intent, &hash),
            },
            Artifact {
                path: "guest-images.txt".into(),
                contents: render_guest_images(intent),
            },
            Artifact {
                path: "build-manifest.txt".into(),
                contents: render_build_manifest(intent, &hash),
            },
        ];
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Self { files }
    }
}

fn render_platform_requirements(intent: &StaticIntentIR) -> String {
    let req = &intent.platform_requirements;
    let mut out = String::new();
    out.push_str("# PlatformRequirements\n");
    out.push_str(&format!("config_name={}\n", req.config_name));
    out.push_str("arch=x86_64\n");
    out.push_str(&format!("vmx={:?}\n", req.vmx));
    out.push_str(&format!("ept={:?}\n", req.ept));
    out.push_str(&format!("vtd={:?}\n", req.vtd));
    out.push_str(&format!("min_physical_cores={}\n", req.min_physical_cores));
    out.push_str(&format!("total_partition_vcpus={}\n", req.total_partition_vcpus));
    out.push_str(&format!("smt_policy={:?}\n", req.smt_policy));
    out.push_str(&format!("min_ram_bytes={}\n", req.min_ram_bytes));
    out.push_str(&format!(
        "total_partition_ram_bytes={}\n",
        req.total_partition_ram_bytes
    ));
    out.push_str(&format!("interrupt_remapping={:?}\n", req.interrupt_remapping));
    out.push_str(&format!("x2apic={:?}\n", req.x2apic));
    out.push_str(&format!("invariant_tsc={:?}\n", req.invariant_tsc));
    out.push_str(&format!("vpid={:?}\n", req.vpid));
    out.push_str(&format!(
        "vmx_preemption_timer={:?}\n",
        req.vmx_preemption_timer
    ));
    out.push_str(&format!("nx={:?}\n", req.nx));
    out.push_str("page_sizes=");
    for (idx, size) in req.page_sizes.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(&size.to_string());
    }
    out.push('\n');
    out.push_str("# expected_pci_devices\n");
    for device in &req.expected_pci_devices {
        out.push_str(&format!(
            "pci owner={} kind={} bdf={}\n",
            device.owner, device.kind, device.bdf
        ));
    }
    out
}

fn render_validated_platform_placeholder(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# ValidatedPlatform\n");
    out.push_str("# ObservedPlatform comparison happens at runtime boot.\n");
    out.push_str(&format!("config_name={}\n", intent.name));
    out.push_str("status=pending_observed_platform\n");
    out
}

fn render_static_platform_rs(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("// @generated by tools/hv-config\n");
    out.push_str("#![allow(dead_code)]\n\n");
    out.push_str(&format!("pub const CONFIG_NAME: &str = {};\n", json_str(&intent.name)));
    out.push_str(&format!(
        "pub const CONFIG_HASH: &str = {};\n",
        json_str(&intent.config_hash.to_hex())
    ));
    out.push_str(&format!("pub const PARTITION_COUNT: usize = {};\n", intent.partitions.len()));
    out.push_str("\npub const PARTITIONS: [PartitionSpec; PARTITION_COUNT] = [\n");
    for partition in &intent.partitions {
        out.push_str("    PartitionSpec {\n");
        out.push_str(&format!("        vm_id: {},\n", partition.vm_id.raw()));
        out.push_str(&format!("        name: {},\n", json_str(&partition.name)));
        out.push_str(&format!("        vcpus: {},\n", partition.vcpus));
        out.push_str(&format!("        memory_bytes: {},\n", partition.memory_bytes));
        out.push_str(&format!("        image: {},\n", json_str(&partition.image)));
        out.push_str(&format!(
            "        iommu_domain: {},\n",
            partition.iommu_domain.raw()
        ));
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");
    out.push_str("#[derive(Clone, Copy, Debug)]\n");
    out.push_str("pub struct PartitionSpec {\n");
    out.push_str("    pub vm_id: u32,\n");
    out.push_str("    pub name: &'static str,\n");
    out.push_str("    pub vcpus: u32,\n");
    out.push_str("    pub memory_bytes: u64,\n");
    out.push_str("    pub image: &'static str,\n");
    out.push_str("    pub iommu_domain: u16,\n");
    out.push_str("}\n");
    out
}

fn render_memory_map(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# Memory plan intent\n");
    for partition in &intent.partitions {
        out.push_str(&format!(
            "partition vm{} name={} private_ram_bytes={}\n",
            partition.vm_id.raw(),
            partition.name,
            partition.memory_bytes
        ));
    }
    for channel in &intent.ipc {
        out.push_str(&format!(
            "ipc {} producer=vm{} consumer=vm{} shared_bytes={}\n",
            channel.name,
            channel.producer.raw(),
            channel.consumer.raw(),
            channel.shared_bytes
        ));
    }
    out
}

fn render_cpu_topology(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# CPU topology intent\n");
    out.push_str(&format!(
        "platform_min_physical_cores={}\n",
        intent.platform_requirements.min_physical_cores
    ));
    out.push_str(&format!(
        "smt_policy={:?}\n",
        intent.platform_requirements.smt_policy
    ));
    for partition in &intent.partitions {
        out.push_str(&format!(
            "partition vm{} name={} vcpus={}\n",
            partition.vm_id.raw(),
            partition.name,
            partition.vcpus
        ));
    }
    out
}

fn render_core_ownership(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# Core ownership intent\n");
    out.push_str("# Physical core IDs are resolved against ObservedPlatform in later phases.\n");
    for partition in &intent.partitions {
        out.push_str(&format!(
            "assign vm{} name={} dedicated_vcpus={}\n",
            partition.vm_id.raw(),
            partition.name,
            partition.vcpus
        ));
    }
    out
}

fn render_ept_map(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# EPT plan intent\n");
    for partition in &intent.partitions {
        out.push_str(&format!(
            "ept vm{} name={} guest_ram_bytes={} default_deny=true\n",
            partition.vm_id.raw(),
            partition.name,
            partition.memory_bytes
        ));
    }
    out
}

fn render_vtd_map(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# VT-d plan intent\n");
    for partition in &intent.partitions {
        out.push_str(&format!(
            "domain {} owner=vm{} name={}\n",
            partition.iommu_domain.raw(),
            partition.vm_id.raw(),
            partition.name
        ));
        for device in &partition.devices {
            out.push_str(&format!(
                "  map domain={} bdf={} kind={:?}\n",
                partition.iommu_domain.raw(),
                device.bdf,
                device.kind
            ));
        }
    }
    out
}

fn render_pci_map(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# PCI ownership map\n");
    for partition in &intent.partitions {
        for device in &partition.devices {
            out.push_str(&format!(
                "owner=vm{} name={} bdf={} kind={:?}\n",
                partition.vm_id.raw(),
                partition.name,
                device.bdf,
                device.kind
            ));
        }
    }
    out
}

fn render_ipc_map(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# IPC map\n");
    for channel in &intent.ipc {
        out.push_str(&format!(
            "channel={} producer=vm{} consumer=vm{} slots={} slot_size={} shared_bytes={}\n",
            channel.name,
            channel.producer.raw(),
            channel.consumer.raw(),
            channel.slot_count,
            channel.slot_size,
            channel.shared_bytes
        ));
    }
    out
}

fn render_irq_map(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# IRQ plan intent\n");
    out.push_str("# MSI/MSI-X remapping is resolved against DMAR in later phases.\n");
    for partition in &intent.partitions {
        for device in &partition.devices {
            out.push_str(&format!(
                "owner=vm{} bdf={} policy=interrupt_remapping_required\n",
                partition.vm_id.raw(),
                device.bdf
            ));
        }
    }
    out
}

fn render_qemu_args(intent: &StaticIntentIR) -> String {
    let qemu = &intent.qemu.plan;
    let mut out = String::new();
    out.push_str("# QEMU arguments generated from configs/qemu.yaml\n");
    out.push_str(&format!("-machine {}\n", qemu.machine));
    out.push_str(&format!("-cpu {}\n", qemu.cpu));
    out.push_str(&format!(
        "-smp cpus={},cores={},threads={}\n",
        qemu.smp.cpus, qemu.smp.cores, qemu.smp.threads
    ));
    out.push_str(&format!("-m {}\n", qemu.memory_bytes / (1024 * 1024)));
    out.push_str(&format!("-accel {}\n", qemu.accel));
    out.push_str(&format!("-bios {}\n", qemu.ovmf));
    for netdev in &qemu.netdevs {
        if let Some(hostfwd) = &netdev.hostfwd {
            out.push_str(&format!(
                "-netdev {},id={},hostfwd={}\n",
                netdev.kind, netdev.id, hostfwd
            ));
        } else {
            out.push_str(&format!("-netdev {},id={}\n", netdev.kind, netdev.id));
        }
    }
    for device in &qemu.devices {
        if let Some(netdev) = &device.netdev {
            out.push_str(&format!(
                "-device {},netdev={},bus=pcie.0,addr={}\n",
                device_kind_qemu(device.kind),
                netdev,
                device.bdf.device.raw()
            ));
        }
    }
    out
}

fn render_boot_layout(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# Boot layout intent\n");
    out.push_str(&format!("loader={}\n", intent.boot.loader));
    out.push_str(&format!("hypervisor={}\n", intent.boot.hypervisor));
    out.push_str(&format!(
        "exit_boot_services={}\n",
        intent.boot.exit_boot_services
    ));
    out
}

fn render_boot_manifest(intent: &StaticIntentIR, hash: &str) -> String {
    let mut out = String::new();
    out.push_str("# Boot manifest\n");
    out.push_str(&format!("config_name={}\n", intent.name));
    out.push_str(&format!("config_hash={hash}\n"));
    out.push_str(&format!("loader={}\n", intent.boot.loader));
    out.push_str(&format!("hypervisor={}\n", intent.boot.hypervisor));
    for partition in &intent.partitions {
        out.push_str(&format!(
            "guest vm{} name={} image={}\n",
            partition.vm_id.raw(),
            partition.name,
            partition.image
        ));
    }
    out
}

fn render_guest_images(intent: &StaticIntentIR) -> String {
    let mut out = String::new();
    out.push_str("# Guest images\n");
    for partition in &intent.partitions {
        out.push_str(&format!(
            "vm{} name={} path={}\n",
            partition.vm_id.raw(),
            partition.name,
            partition.image
        ));
    }
    out
}

fn render_build_manifest(intent: &StaticIntentIR, hash: &str) -> String {
    let mut out = String::new();
    out.push_str("# Build manifest\n");
    out.push_str("rustc=1.85.0\n");
    out.push_str("target=x86_64-unknown-none\n");
    out.push_str(&format!("config_name={}\n", intent.name));
    out.push_str(&format!("config_hash={hash}\n"));
    out.push_str(&format!(
        "throughput_metric=udp_payload_bytes\nofficial_benchmark_runs={}\n",
        intent.benchmark.runs
    ));
    out
}

fn json_str(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn device_kind_qemu(kind: crate::normalize::DeviceKind) -> &'static str {
    match kind {
        crate::normalize::DeviceKind::E1000 => "e1000",
    }
}
