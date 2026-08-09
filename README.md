# Hypster

Static x86-64 hypervisor in Rust `no_std` for safety- and security-critical
partitioned systems.

## Status

This branch completes **Gate B (before VMX)** on top of Phase 0–3:

- ACPI + ELF parsers, CPU/memory/EPT/VT-d planners
- `ObservedPlatform` validation and `StaticPlatformIR` resolution
- UEFI loader skeleton (`hv-loader`) and hypervisor entry (`hypster`)
- boot phase FSM and Gate B integration tests

## Quick start

```bash
cargo xtask test
cargo xtask build
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
cargo xtask platform resolve configs/qemu.yaml
cargo clippy --workspace --exclude hv-loader --exclude hypster --all-targets --all-features -- -D warnings
cargo build -p hv-loader --target x86_64-unknown-uefi
cargo build -p hypster --target x86_64-unknown-none
```

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Platform contract](docs/platform-contract.md)
- [Proof levels](docs/proof-levels.md)

## Configuration source of truth

[`configs/qemu.yaml`](configs/qemu.yaml)

## License

MIT OR Apache-2.0
