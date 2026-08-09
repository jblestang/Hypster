# Platform Contract

Hypster separates desired configuration from observed hardware using an explicit
contract chain:

```text
Desired Configuration (YAML)
        |
        v
PlatformRequirements
        |
        | compare at boot
        v
ObservedPlatform
        |
        v
ValidatedPlatform
        |
        v
StaticPlatformIR
```

This branch implements the full Gate B contract chain through `StaticPlatformIR`.
Phase 0–3 delivered `PlatformRequirements` and `StaticIntentIR`; Gate B adds
`ObservedPlatform`, validation, planners, and boot skeletons.

## PlatformRequirements

`PlatformRequirements` is derived deterministically from validated YAML. It
expresses minimum platform capabilities and expected devices:

| Field | Meaning |
|-------|---------|
| `arch` | Required architecture (`x86_64`) |
| `vmx`, `ept`, `vtd`, `nx` | Virtualization and protection requirements |
| `min_physical_cores` | Minimum physical cores |
| `total_partition_vcpus` | Sum of configured partition vCPUs |
| `smt_policy` | SMT placement policy |
| `min_ram_bytes` | Minimum host RAM declared by config |
| `total_partition_ram_bytes` | Sum of private partition RAM |
| `interrupt_remapping` | Interrupt remapping requirement level |
| `x2apic`, `invariant_tsc`, `vpid`, `vmx_preemption_timer` | Feature levels |
| `page_sizes` | Required guest page sizes |
| `expected_pci_devices` | PCI devices that must exist and be assignable |

## Fail-closed policy

When a requirement level is `required`, absence or mismatch must refuse boot.
The helper `PlatformRequirements::is_fail_closed()` encodes this policy.

Examples:

- VT-d required but DMAR missing -> boot refused
- VMX unavailable -> boot refused
- Required PCI device absent -> boot refused
- Config hash mismatch when required -> boot refused

Preferred or optional features must not silently downgrade required properties.

## ObservedPlatform

`ObservedPlatform` is built at boot from:

- CPUID and MSRs (summarized in `ObservedCpuFeatures`)
- UEFI memory map (`ConventionalRegion` list)
- ACPI summaries (RSDP → XSDT/RSDT → MADT, DMAR, MCFG)
- PCI enumeration (`ObservedPciDevice`)

Gate B host tests use `hv_core::fixture::qemu_validation_observed()` as a
deterministic QEMU stand-in. The UEFI loader discovers the RSDP from the
configuration table and passes it through `BootInfo`.

Hardware must satisfy or refuse; it must never silently rewrite YAML.

## StaticPlatformIR

`StaticPlatformIR` is produced by `resolve_platform()` after validation:

```text
StaticIntentIR + ObservedPlatform
  → validate_platform()
  → plan_cpu / plan_memory / plan_ept / plan_vtd
  → StaticPlatformIR
```

Run `cargo xtask platform resolve configs/qemu.yaml` to exercise the pipeline
on the reference configuration.

## Proof levels

| Contract step | Levels |
|---------------|--------|
| YAML -> PlatformRequirements | UNIT + PROPERTY |
| Deterministic IR | UNIT + PROPERTY |
| ObservedPlatform compare | UNIT + QEMU fixture |
| StaticPlatformIR resolve | UNIT + `cargo xtask platform resolve` |
| Fail-closed boot refusal | UNIT + Gate B integration tests |
| UEFI loader / hypervisor skeleton | BUILD (`x86_64-unknown-uefi`, `x86_64-unknown-none`) |
