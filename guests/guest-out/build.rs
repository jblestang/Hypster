//! Applies the shared guest linker script at build time.

#![allow(clippy::expect_used)]

fn main() {
    let manifest_dir =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest"));
    let guest_ld = manifest_dir.join("../guest.ld");
    println!("cargo:rerun-if-changed={}", guest_ld.display());
    println!("cargo:rustc-link-arg=-T{}", guest_ld.display());
}
