# Hypster Architecture

Hypster is a static x86-64 hypervisor written in Rust `no_std`. The system
partitions a platform into isolated VMs connected by unidirectional IPC
channels. Configuration is the single source of truth; the runtime never
hardcodes business topology such as IN, MID, or OUT.

## Workspace layout

```text
crates/
  hv-types/          Strong newtypes and checked arithmetic
  hv-config-model/   YAML -> validation -> IR pipeline
  hv-boot-abi/       Loader/hypervisor boot ABI
  hv-guest-abi/      Guest boot ABI
tools/
  hv-config/         Configuration compiler CLI
xtask/               Developer task runner
configs/
  qemu.yaml          Validation topology source of truth
docs/                Architecture and contract documentation
```

## Configuration pipeline

```text
configs/qemu.yaml
      |
      v
   RawConfig
      |
      v
 syntax + semantic validation
      |
      v
 NormalizedConfig
      |
      v
 PlatformRequirements
      |
      v
 StaticIntentIR
      |
      v
 generated artifacts + StaticPlatformIR (later phases)
```

The runtime iterates generic partition descriptors:

```rust
for partition in config.partitions() {
    // no start_in()/start_mid()/start_out()
}
```

## Platform contract

Desired configuration lowers to [`PlatformRequirements`](platform-contract.md).
At boot, firmware and hardware observation produce `ObservedPlatform`. The
hypervisor accepts the platform only when the contract is satisfied; required
properties are fail-closed.

## Phases completed in this branch

- Phase 0: workspace, lint policy, `hv-types`, checked arithmetic, xtask
- Phase 1: YAML schema, `RawConfig`, strict validation
- Phase 2: `NormalizedConfig`, `PlatformRequirements`, `StaticIntentIR`
- Phase 3 design: `hv-boot-abi` and `hv-guest-abi` skeletons

Virtualization runtime, loader, and hardware programming begin after Gate A.

## Safety ordering

1. Safety
2. Security
3. Determinism
4. Correctness
5. Performance
6. Testability
7. Auditability
8. Simplicity
9. Small TCB

## Steady-state rule

After initialization the hypervisor steady state must not perform dynamic
allocation. All datapath buffers, IPC rings, and VMCS/EPT/IOMMU tables are
preallocated from validated configuration.
