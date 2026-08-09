# Hypster

Static x86-64 hypervisor in Rust `no_std` for safety- and security-critical
partitioned systems.

## Status

This branch completes **Gate C (before e1000)** on top of Gate B:

- `hv-x86` / `hv-cpu` — CPUID and MSR helpers, CPU feature probe
- `hv-vmx` — VMXON region, host init, VMCS layout (hardware via `hardware` feature)
- EPT/VT-d **install** modules — page table builders from Gate B plans
- `hv-runtime` — boot-time init pipeline through `BootPhase::Running`
- UEFI loader **ExitBootServices** handoff to hypervisor entry
- Boot ABI v1.1 with memory map trailing descriptors

## Quick start

```bash
cargo xtask test
cargo xtask build
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
cargo xtask platform resolve configs/qemu.yaml
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p hv-loader --target x86_64-unknown-uefi --all-features -- -D warnings
cargo clippy -p hypster --target x86_64-unknown-none --all-features -- -D warnings
cargo build -p hv-loader --target x86_64-unknown-uefi --features uefi-bin
cargo build -p hypster --target x86_64-unknown-none --features bare-metal-bin
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
