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

## Gates

- **Gate A (before UEFI)** — types, config, requirements, IR, tests
- **Gate B (before VMX)** — loader, ACPI, observed platform, planners
- **Gate C (before e1000)** — CPU/DMA isolation, lifecycle
- **Gate D (before optimization)** — end-to-end datapath, malicious tests

This branch targets Gate A.
