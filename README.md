# Hypster

Static x86-64 hypervisor in Rust `no_std` for safety- and security-critical
partitioned systems.

## Status

This branch completes **Phase 0–3 foundation**:

- workspace, lint policy, `hv-types`
- YAML configuration compiler and validation
- `PlatformRequirements` and `StaticIntentIR`
- boot/guest ABI skeletons

## Quick start

```bash
cargo xtask test
cargo xtask build
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
cargo clippy --all-targets --all-features -- -D warnings
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
