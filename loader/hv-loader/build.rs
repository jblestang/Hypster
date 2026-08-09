//! Embeds the validated configuration hash at build time.

#![allow(clippy::expect_used)]

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=configs/qemu.yaml");
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let workspace = manifest_dir.parent().and_then(|path| path.parent()).expect("workspace root");
    let config_path = workspace.join("configs/qemu.yaml");

    let raw =
        hv_config_model::yaml::read_yaml_file(config_path.to_str().expect("config path utf8"))
            .expect("read qemu.yaml");
    let compiled = hv_config_model::pipeline::compile_config(raw).expect("compile config");
    let bytes = compiled.validated.config_hash.bytes();
    let hash = compiled.validated.config_hash.to_hex();

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let path = out_dir.join("config_hash.rs");
    if std::fs::write(
        path,
        format!(
            "pub const HV_CONFIG_HASH: [u8; 32] = {bytes:?};\n\
             pub const HV_CONFIG_HASH_HEX: &str = \"{hash}\";\n"
        ),
    )
    .is_err()
    {
        std::process::exit(1);
    }
}
