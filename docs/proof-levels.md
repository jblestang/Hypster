# Proof Levels

Each critical requirement declares how it is validated. QEMU alone is never
sufficient for silicon-specific properties.

## Categories

| Level | Meaning |
|-------|---------|
| UNIT | Host unit tests |
| PROPERTY | Property-based tests |
| FUZZ | Fuzz targets |
| MIRI | Miri on safe core |
| MODEL | Formal or logical model |
| QEMU | QEMU/OVMF integration |
| REAL_HW | Intel hardware validation |
| PERFORMANCE | Benchmark gate |
| REVIEW | Design/code review |

## Phase 0-3 matrix

| Requirement | Levels |
|-------------|--------|
| Checked arithmetic (`hv-types`) | UNIT + PROPERTY |
| PCI BDF parsing | UNIT + PROPERTY |
| YAML parse/validate | UNIT + PROPERTY + FUZZ (planned) |
| Deterministic `VmId` assignment | UNIT + PROPERTY |
| Config hash stability | UNIT |
| `PlatformRequirements` extraction | UNIT |
| `StaticIntentIR` determinism | UNIT + PROPERTY |
| Datapath constraint: no direct NIC IPC | UNIT |
| Boot ABI layout | UNIT |
| Guest ABI layout | UNIT |
| VMX/EPT/VT-d hardware behavior | QEMU + REAL_HW (future) |
| 200 Mbit/s throughput | PERFORMANCE (future) |

## Gate B matrix

| Requirement | Levels |
|-------------|--------|
| ACPI table parsers (`hv-acpi`) | UNIT |
| ELF64 loader parser (`hv-elf`) | UNIT |
| `ObservedPlatform` construction | UNIT + QEMU fixture |
| Platform validation (fail-closed) | UNIT + Gate B integration tests |
| CPU/memory/EPT/VT-d planners | UNIT + `platform resolve` |
| Boot phase FSM | UNIT |
| UEFI loader skeleton | BUILD (`x86_64-unknown-uefi`) |
| Hypervisor entry skeleton | BUILD (`x86_64-unknown-none`) |
| VMX/EPT/VT-d hardware programming | QEMU + REAL_HW (Gate C) |

## Gate C matrix

| Requirement | Levels |
|-------------|--------|
| x86 CPUID/MSR helpers (`hv-x86`) | UNIT |
| CPU feature probe (`hv-cpu`) | UNIT |
| VMX host init (`hv-vmx`) | UNIT + BUILD + QEMU (with `hardware` feature) |
| EPT page table install | UNIT + Gate C integration tests |
| VT-d context table install | UNIT + Gate C integration tests |
| Boot ABI v1.1 memory map layout | UNIT |
| Loader ExitBootServices handoff | BUILD (`x86_64-unknown-uefi`) |
| Runtime init pipeline (`hv-runtime`) | UNIT + Gate C integration tests |
| CPU/DMA isolation at runtime | QEMU + REAL_HW (Gate D guests) |

## Gate D matrix

| Requirement | Levels |
|-------------|--------|
| IPC SPSC ring protocol (`hv-ipc`) | UNIT + malicious tests |
| Guest ABI layout (`hv-guest-abi`) | UNIT |
| Guest boot info builder (`hv-partition`) | UNIT + Gate D integration tests |
| EPT IPC shared-memory mappings | UNIT + Gate D integration tests |
| Config hash enforcement | UNIT + Gate D integration tests |
| Embedded platform plans (hypervisor) | BUILD |
| Guest partition stubs | BUILD (`x86_64-unknown-none`) |
| Guest MID IPC relay loop | BUILD + UNIT (`guest-common`) |
| End-to-end datapath (e1000, UDP) | UNIT + host-assisted integration (`cargo xtask datapath smoke`) |
| Guest VMCS launch planning | UNIT + Gate D integration tests |
| e1000 MMIO decode skeleton | UNIT + malicious tests |
| EPT MMIO device mappings | UNIT + Gate D integration tests |
| CPU/DMA isolation with guests | QEMU + REAL_HW (future) |

## Gates

- **Gate A (before UEFI)** — types, config, requirements, IR, tests
- **Gate B (before VMX)** — loader, ACPI, observed platform, planners
- **Gate C (before e1000)** — CPU/DMA isolation, lifecycle
- **Gate D (before optimization)** — end-to-end datapath, malicious tests

This branch targets Gate D.
