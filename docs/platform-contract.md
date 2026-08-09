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

This branch implements `PlatformRequirements` and `StaticIntentIR`. Runtime
construction of `ObservedPlatform` and comparison logic begins in Phase 8/9.

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

## ObservedPlatform (future)

`ObservedPlatform` will be built from:

- CPUID and MSRs
- UEFI memory map
- ACPI RSDP, XSDT, MADT, DMAR, MCFG
- PCI enumeration

Hardware must satisfy or refuse; it must never silently rewrite YAML.

## StaticIntentIR

`StaticIntentIR` is the static planning IR consumed by later allocators for:

- CPU plan
- memory plan
- EPT plan
- VT-d plan
- PCI plan
- IPC plan
- IRQ plan
- boot plan
- QEMU plan

Each partition receives a dedicated IOMMU domain identifier in the MVP IR.

## Proof levels

| Contract step | Levels |
|---------------|--------|
| YAML -> PlatformRequirements | UNIT + PROPERTY |
| Deterministic IR | UNIT + PROPERTY |
| ObservedPlatform compare | QEMU + REAL_HW (future) |
| Fail-closed boot refusal | QEMU + REAL_HW (future) |
