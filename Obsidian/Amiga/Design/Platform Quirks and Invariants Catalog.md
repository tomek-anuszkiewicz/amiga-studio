---
title: "Platform Quirks & Invariants Catalog"
aliases: ["Platform Quirks Catalog", "Hardware Quirks Index", "Silicon Traps Catalog"]
tags: ["amiga", "design", "quirks", "invariants", "m68000", "chipset"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-09-12
updated: 2026-10-12
related: ["[General Architecture.md](General%20Architecture.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)", "[MemoryBus.md](MemoryBus.md)"]
---

# Platform Quirks & Invariants Catalog

- **Parent Architectural Hub:** [General Architecture.md](General%20Architecture.md)
- **Owning specifications:** [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) | [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md) | [MemoryBus.md](MemoryBus.md)

This index highlights behavior that is easy to lose when simplifying the CPU pipeline or the Amiga bus. The linked subsystem specifications define the complete behavior and its evidence.

---

## 1. Scope

Instruction semantics are checked against the [SingleStep contract](Verification/CPU%20SingleStepTests.md) and Motorola references. The [CPU specification](CPU%20Motorola%20M68000.md) describes the external interface; DMA scheduling, address decoding, and ROM overlay belong in their subsystem specifications. This catalog records ordering and fault effects that a conventional instruction-level model can miss, plus the Amiga-specific TAS write exception.

---

## 2. Motorola 68000 Silicon Quirks & CPU Pipeline Traps

| Case | Ordering or fault effect | Detail |
| :--- | :--- | :--- |
| Memory read-modify-write | Prefetch can read the next instruction word before the operand write. Reversing the order changes self-modifying code. | [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md) |
| Dual-memory address error | A faulting source read in `CMPM` or predecrement `ADDX` enters Vector 3 before the destination address register advances. | [CMPM](../../../crates/cpu/src/instructions/cmpm.rs) and [ADDX](../../../crates/cpu/src/instructions/addx.rs) sequences |
| `(An)+` address error | A faulting word/long read retains the postincrement; a faulting write does not increment the register. | [CPU instruction sequences](../../../crates/cpu/src/instructions) |
| `MOVE` to `-(An)` | Prefetch precedes the store. Long stores write the low word first; a rejected transfer preserves only address updates already reached. | [Long MOVE sequence](../../../crates/cpu/src/instructions/move_l.rs) |

---

## 3. Amiga Bus Exception

| Case | Hardware effect | Detail |
| :--- | :--- | :--- |
| `TAS` on Chip or Slow RAM | The Amiga bus drops the memory write phase; condition codes reflect the read, but the stored byte remains unchanged. Other memory banks use the ordinary write path. | [MemoryBus.md](MemoryBus.md#rtc-and-exceptional-access-paths) |

---

## 4. Reference Documentation & Evidence

- [Motorola 68000 instruction prefetch](../Reference/Instruction%20Prefetch%20on%20the%20Motorola%2068000%20Processor.md): Bus ordering and prefetch context.
- [Motorola 68000 Programmer's Reference Manual](../Reference/68000%20Programmer's%20Reference%20Manual): Documented instruction and exception semantics.
- [SingleStepTests-680x0](../../../ref_src/SingleStepTests-680x0/68000/v1): Silicon-derived state and bus-cycle vectors; individual cases still require interpretation.
