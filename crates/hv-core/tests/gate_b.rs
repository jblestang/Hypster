#![allow(missing_docs)]
#![allow(clippy::expect_used)]

use hv_config_model::pipeline::compile_config;
use hv_config_model::yaml::read_yaml_file;
use hv_core::fixture::qemu_validation_observed;
use hv_core::resolve_platform;

#[test]
fn gate_b_qemu_platform_resolves() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let observed = qemu_validation_observed().expect("fixture");
    let resolved = resolve_platform(&compiled.intent, &observed).expect("resolve platform");
    assert_eq!(resolved.cpu.assignments.len(), 3);
    assert_eq!(resolved.ept.partitions.len(), 3);
    assert_eq!(resolved.vtd.domains.len(), 3);
    assert_eq!(resolved.vtd.domains.iter().filter(|domain| !domain.devices.is_empty()).count(), 2);
}

#[test]
fn gate_b_fail_closed_when_vtd_missing() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read config");
    let compiled = compile_config(raw).expect("compile config");
    let mut observed = qemu_validation_observed().expect("fixture");
    observed.acpi.dmar = None;
    observed.cpu.vtd = false;
    assert!(resolve_platform(&compiled.intent, &observed).is_err());
}
