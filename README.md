# Hypster

Static x86-64 hypervisor in Rust `no_std` for safety- and security-critical
partitioned systems.

## Status

This branch completes **Gate D (before optimization)** on top of Gate C:

- `hv-ipc` — SPSC IPC ring protocol with corruption detection
- `hv-partition` — guest boot info builder and Gate D plan helpers
- `hv-e1000` — e1000 MMIO decode skeleton for device BAR emulation
- `hv-vmx` — guest VMCS field planning and VMLAUNCH instruction wrappers
- EPT planner — IPC shared-memory and MMIO device mappings
- `hv-runtime` — Gate D init with IPC ring init, guest launch planning, host-assisted e2e datapath engine
- `hv-elf` — guest ELF PT_LOAD loader for partition images
- Embedded platform plans and config hash at build time (loader + hypervisor)
- Guest partitions (`guest-in`, `guest-mid`, `guest-out`) with MID IPC relay loop

## Quick start

```bash
cargo xtask test
cargo xtask build
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
cargo xtask platform resolve configs/qemu.yaml
cargo xtask datapath smoke
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p hv-loader --target x86_64-unknown-uefi --all-features -- -D warnings
cargo clippy -p hypster --target x86_64-unknown-none --all-features -- -D warnings
cargo build -p hv-loader --target x86_64-unknown-uefi --features uefi-bin
cargo build -p hypster --target x86_64-unknown-none --features bare-metal-bin
cargo build -p guest-in --target x86_64-unknown-none --features bare-metal-bin
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
