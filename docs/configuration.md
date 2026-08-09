# Configuration

The file [`configs/qemu.yaml`](../configs/qemu.yaml) is the single source of
truth for:

- partitions, CPU counts, RAM, images
- PCI device ownership
- IPC topology and queue parameters
- platform requirements
- security and benchmark policies
- boot artifact paths
- QEMU launch parameters

The same data must not be duplicated manually in Rust, QEMU scripts, EPT/VT-d
tables, or benchmark scripts.

## Compiler

The configuration compiler lives in `tools/hv-config/` and is exposed through:

```bash
cargo xtask config validate configs/qemu.yaml
cargo xtask config generate configs/qemu.yaml
```

Generated artifacts are written to `target/generated/` by default:

- `static-platform.rs`
- `platform-requirements.txt`
- `memory-map.txt`
- `cpu-topology.txt`
- `core-ownership.txt`
- `ept-map.txt`
- `vtd-map.txt`
- `pci-map.txt`
- `ipc-map.txt`
- `irq-map.txt`
- `qemu-args.txt`
- `boot-layout.txt`
- `boot-manifest.txt`
- `guest-images.txt`
- `config.sha256`
- `build-manifest.txt`

## Validation rules (MVP)

- schema version must be supported
- partition and IPC names must be unique
- vCPU count and memory must be non-zero
- PCI BDF ownership is exclusive
- IPC producer and consumer must differ
- bidirectional IPC between the same pair is forbidden
- direct IPC between two NIC-owning partitions is forbidden
- total partition RAM must not exceed declared platform minimum RAM
- benchmark must use at least 5 runs, non-zero warmup and measurement periods

## Deterministic identifiers

`VmId` values are assigned by partition declaration order starting at zero.
Reordering partitions in YAML changes IDs and the configuration hash.

## Configuration hash

The hash is SHA-256 over canonical JSON derived from `NormalizedConfig`. YAML
whitespace or key order does not affect the hash.

## Validation topology

The reference QEMU configuration defines three partitions (`in`, `mid`, `out`)
and two unidirectional IPC channels (`in_to_mid`, `mid_to_out`). These names
are data, not code constants.

Official benchmark throughput metric:

```yaml
throughput_metric: udp_payload_bytes
```

This means the 200 Mbit/s gate measures useful UDP payload bytes received at
the OUT partition, not IP or Ethernet framing bytes.
