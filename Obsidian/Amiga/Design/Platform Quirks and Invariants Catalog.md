---
title: "Platform Quirks & Invariants Catalog"
aliases: ["Platform Quirks Catalog", "Hardware Quirks Index", "Silicon Traps Catalog"]
tags: ["amiga", "design", "quirks", "invariants", "m68000", "chipset"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-09-12
updated: 2026-10-13
related: ["[General Architecture.md](General%20Architecture.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)", "[MemoryBus.md](MemoryBus.md)", "[Physical Memory.md](Physical%20Memory.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)"]
---

# Platform Quirks & Invariants Catalog

- **Parent Architectural Hub:** [General Architecture.md](General%20Architecture.md)
- **Owning specifications:** [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) | [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md) | [Physical Memory.md](Physical%20Memory.md) | [MemoryBus.md](MemoryBus.md)

This index highlights behavior that is easy to lose when simplifying the CPU pipeline or the Amiga bus. The linked subsystem specifications define the implemented behavior; hardware references and stated limitations distinguish it from physical behavior.

---

## 1. Scope

Instruction semantics are checked against the [SingleStep contract](Verification/CPU%20SingleStepTests.md) and Motorola references. The [CPU specification](CPU%20Motorola%20M68000.md) describes the external interface; DMA scheduling, address decoding, and ROM overlay belong in their subsystem specifications. This catalog records ordering and fault effects that a conventional instruction-level model can miss, the Amiga-specific TAS write exception, refresh sync strobes, and the limits of fixed open-bus values.

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

### CPU writes to refresh sync strobes

`STREQU`, `STRVBL`, `STRHOR`, and `STRLONG` are address strobes that Agnus normally presents during refresh slots to synchronize Denise; `STRHOR` also reaches Paula. The [register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md) records their documented function and the prohibition on Copper writes. A CPU write can address the same offsets through [MemoryBus](MemoryBus.md), so treating them as ordinary unused storage would discard a possible synchronization effect. The [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#strequ-strhor-strlong-strvbl) establishes the refresh-time strobes, but the precise effect of an extra CPU-initiated strobe on Denise or Paula has not been verified here.

The current router forwards these writes to [`Denise::receive_sync_strobe()`](../../../crates/denise/src/denise.rs) and, for `STRHOR`, [`Paula::receive_horizontal_sync_strobe()`](../../../crates/paula/src/paula.rs). Both receivers are presently inert, so CPU writes produce no modeled synchronization effect. This is an explicit emulation limit, not evidence that the physical write is harmless or that software never uses it.

### Open bus and opposite-direction register access

The emulator returns a configured byte (normally `$FF`) for unmapped [physical memory](Physical%20Memory.md) and repeats it as `$FFFF` for word reads. [MemoryBus](MemoryBus.md) independently returns `$FF` for inactive CIA lanes and absent RTC responses, and `$FFFF` for most custom-register addresses without a modeled read. These fixed values do not track which device last drove the data lines or establish what a particular physical bus cycle would return. A read of `COPJMP1` or `COPJMP2` is an implemented exception: it strobes the Copper even though the returned value is `$FFFF`.

`BLTDDAT` and `DSKDATR` are early DMA read addresses, not ordinary CPU-readable data registers. The [Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md) explains that Agnus initiates these transfers to Chip RAM according to DMA timing, rather than a read address presented by the CPU. [Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bltddat) explicitly says the microprocessor cannot read `BLTDDAT`. A CPU read of either address therefore does not fetch the blitter result or disk DMA word; the current `MemoryBus` returns its fixed open-bus approximation of `$FFFF`. This does not establish the exact value driven or retained on a physical CPU read cycle.

The [custom-register ownership matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md) records documented access directions, not guaranteed behavior for an opposite-direction cycle. The [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md) warns that reading a write-only register can disturb its contents and writing a read-only register can have unexpected results. A fixed `$FFFF` response therefore does not establish that a physical read was harmless; the emulator models only explicitly handled side effects. The manual likewise gives no universal value for unused register addresses. Determining exact values or further side effects requires evidence for the specific address and bus phase.

---

## 4. Reference Documentation & Evidence

- [Motorola 68000 instruction prefetch](../Reference/Instruction%20Prefetch%20on%20the%20Motorola%2068000%20Processor.md): Bus ordering and prefetch context.
- [Motorola 68000 Programmer's Reference Manual](../Reference/68000%20Programmer's%20Reference%20Manual): Documented instruction and exception semantics.
- [SingleStepTests-680x0](../../../ref_src/SingleStepTests-680x0/68000/v1): Silicon-derived state and bus-cycle vectors; individual cases still require interpretation.
- [Amiga Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Documents the hazards of opposite-direction and unused custom-register accesses.
- [Amiga Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Defines early DMA reads and identifies `BLTDDAT` and `DSKDATR` as dummy addresses.
