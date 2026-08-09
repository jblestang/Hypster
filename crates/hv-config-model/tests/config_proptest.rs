#![allow(missing_docs)]
#![allow(clippy::expect_used)] // Property tests assert on generated valid inputs.

use hv_config_model::normalize::normalize;
use hv_config_model::raw::{
    FaultPolicy, IpcDirection, QueueFullPolicy, RawBenchmark, RawBoot, RawConfig, RawDevice,
    RawIpc, RawPartition, RawPerformance, RawPlatform, RawPlatformRequirements, RawQemu,
    RawSecurity, RawSmp, RequirementLevel, SmtPolicy, ThroughputMetric,
};
use proptest::prelude::*;

fn minimal_requirements() -> RawPlatformRequirements {
    RawPlatformRequirements {
        arch: "x86_64".into(),
        vmx: RequirementLevel::Required,
        ept: RequirementLevel::Required,
        vtd: RequirementLevel::Required,
        min_physical_cores: 3,
        smt_policy: SmtPolicy::ExclusiveCore,
        min_ram_gib: 8,
        interrupt_remapping: RequirementLevel::Required,
        x2apic: RequirementLevel::Preferred,
        invariant_tsc: RequirementLevel::Preferred,
        vpid: RequirementLevel::Preferred,
        vmx_preemption_timer: RequirementLevel::Optional,
        nx: RequirementLevel::Required,
        page_sizes: vec![4096, 2_097_152],
    }
}

fn minimal_benchmark() -> RawBenchmark {
    RawBenchmark {
        protocol: "udp_ipv4".into(),
        frame_size: 1514,
        payload_size: 1472,
        throughput_metric: ThroughputMetric::UdpPayload,
        warmup_seconds: 10,
        measurement_seconds: 30,
        runs: 5,
        max_loss_ratio: "0.001".into(),
    }
}

fn base_config(partitions: Vec<RawPartition>, ipc: Vec<RawIpc>) -> RawConfig {
    RawConfig {
        version: 1,
        name: "test".into(),
        platform: RawPlatform { requirements: minimal_requirements() },
        partitions,
        ipc,
        security: RawSecurity {
            require_config_hash: true,
            ipc_corruption_policy: FaultPolicy::StopPartition,
            ept_violation_policy: FaultPolicy::StopPartition,
            iommu_fault_policy: FaultPolicy::StopPartition,
        },
        performance: RawPerformance { benchmark: minimal_benchmark() },
        boot: RawBoot {
            loader: "loader.efi".into(),
            hypervisor: "hypster".into(),
            exit_boot_services: true,
        },
        qemu: RawQemu {
            machine: "q35".into(),
            cpu: "host".into(),
            smp: RawSmp { cpus: 4, cores: 4, threads: 1 },
            memory_mib: 8192,
            accel: "tcg".into(),
            ovmf: "/opt/OVMF/OVMF_CODE.fd".into(),
            netdevs: vec![],
            devices: vec![],
        },
    }
}

proptest! {
    #[test]
    fn duplicate_partition_names_are_rejected(name in "[a-z]{1,8}") {
        let partition = RawPartition {
            name: name.clone(),
            vcpus: 1,
            memory_gib: 1,
            image: "guest".into(),
            devices: vec![],
            stack: None,
        };
        let config = base_config(vec![partition.clone(), partition], vec![]);
        prop_assert!(normalize(config).is_err());
    }

    #[test]
    fn pci_bdf_conflict_is_rejected(
        bdf in prop_oneof!["0000:00:03.0", "0000:00:04.0"]
    ) {
        let p1 = RawPartition {
            name: "a".into(),
            vcpus: 1,
            memory_gib: 1,
            image: "guest-a".into(),
            devices: vec![RawDevice { kind: "e1000".into(), bdf: bdf.clone() }],
            stack: None,
        };
        let p2 = RawPartition {
            name: "b".into(),
            vcpus: 1,
            memory_gib: 1,
            image: "guest-b".into(),
            devices: vec![RawDevice { kind: "e1000".into(), bdf }],
            stack: None,
        };
        let config = base_config(vec![p1, p2], vec![]);
        prop_assert!(normalize(config).is_err());
    }
}

#[test]
fn valid_minimal_chain_normalizes() {
    let config = base_config(
        vec![
            RawPartition {
                name: "a".into(),
                vcpus: 1,
                memory_gib: 1,
                image: "a".into(),
                devices: vec![],
                stack: None,
            },
            RawPartition {
                name: "b".into(),
                vcpus: 1,
                memory_gib: 1,
                image: "b".into(),
                devices: vec![],
                stack: None,
            },
        ],
        vec![RawIpc {
            name: "a_to_b".into(),
            producer: "a".into(),
            consumer: "b".into(),
            direction: IpcDirection::Unidirectional,
            slot_count: 16,
            slot_size: 1024,
            queue_full_policy: QueueFullPolicy::DropTail,
        }],
    );
    assert!(normalize(config).is_ok());
}
