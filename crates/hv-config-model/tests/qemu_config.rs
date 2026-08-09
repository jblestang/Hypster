#![allow(missing_docs)]
#![allow(clippy::expect_used)] // Tests assert on the checked-in reference configuration.

use hv_config_model::pipeline::compile_config;
use hv_config_model::yaml::read_yaml_file;

#[test]
fn qemu_config_is_valid() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read qemu.yaml");
    let compiled = compile_config(raw).expect("compile qemu.yaml");
    assert_eq!(compiled.intent.partitions.len(), 3);
    assert_eq!(compiled.intent.ipc.len(), 2);
    assert!(compiled.validated.config_hash.to_hex().len() == 64);
}

#[test]
fn deterministic_vm_ids_follow_declaration_order() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read qemu.yaml");
    let compiled = compile_config(raw).expect("compile qemu.yaml");
    let names: Vec<_> = compiled
        .intent
        .partitions
        .iter()
        .map(|p| (p.vm_id.raw(), p.name.as_str()))
        .collect();
    assert_eq!(names, vec![(0, "in"), (1, "mid"), (2, "out")]);
}

#[test]
fn no_direct_nic_partition_ipc() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read qemu.yaml");
    let compiled = compile_config(raw).expect("compile qemu.yaml");
    let in_id = compiled.intent.partitions[0].vm_id;
    let out_id = compiled.intent.partitions[2].vm_id;
    assert!(!compiled.intent.ipc.iter().any(|ipc| {
        (ipc.producer == in_id && ipc.consumer == out_id)
            || (ipc.producer == out_id && ipc.consumer == in_id)
    }));
}

#[test]
fn compilation_is_deterministic() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../configs/qemu.yaml");
    let raw = read_yaml_file(path).expect("read qemu.yaml");
    let first = compile_config(raw.clone()).expect("first compile");
    let second = compile_config(raw).expect("second compile");
    assert_eq!(first.validated.config_hash, second.validated.config_hash);
    assert_eq!(first.intent, second.intent);
}
