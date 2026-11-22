---
title: "Platform Quirks & Invariants Catalog"
aliases: ["Platform Quirks Catalog", "Hardware Quirks Index", "Silicon Traps Catalog"]
tags: ["amiga", "design", "quirks", "invariants", "m68000", "chipset"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-09-12
updated: 2026-11-12
related: ["[General Architecture.md](General%20Architecture.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md)", "[MemoryBus.md](MemoryBus.md)", "[Physical Memory.md](Physical%20Memory.md)", "[Custom Chip Register Ownership and Access Matrix.md](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)"]
---

# Platform Quirks & Invariants Catalog

- **Parent Architectural Hub:** [General Architecture.md](General%20Architecture.md)
- **Owning specifications:** [CPU Motorola M68000.md](CPU%20Motorola%20M68000.md) | [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md) | [Physical Memory.md](Physical%20Memory.md) | [MemoryBus.md](MemoryBus.md)

This catalog points out behavior that a simpler CPU pipeline or Amiga bus
model could miss. The linked subsystem specifications define the
implementation. The hardware references show where its behavior can be
compared with the physical machine, and the notes identify what those
sources leave open.

---

## 1. Scope

CPU verification checks instruction semantics against SingleStep and the
Motorola references. [SingleStep contract](Verification/CPU%20SingleStepTests.md)

The CPU specification describes the external interface. The subsystem
specifications define DMA scheduling, address decoding, and the ROM overlay.
This catalog records ordering and fault effects that an instruction-level
model can miss, the Amiga-specific TAS write exception, refresh sync strobes,
and the limits of fixed open-bus values. [CPU specification](CPU%20Motorola%20M68000.md)

---

## 2. Motorola 68000 Silicon Quirks & CPU Pipeline Traps

| Case | Ordering or fault effect | Detail |
| :--- | :--- | :--- |
| Memory read-modify-write | Prefetch can read the next instruction word before the operand write. Reversing the order changes self-modifying code. | [CPU Micro-Step State Machine.md](CPU%20Micro-Step%20State%20Machine.md) |
| `DBcc` counter expiry | With a false condition and `Dn.w = 0`, the CPU decrements the counter to `$FFFF`, reads the branch target speculatively, then refills from the fallthrough address. The path takes 14 CPU clocks and three reads. | The supplied SingleStep `DBcc.json` covers the 12-clock condition-true path but omits counter expiry. See [Motorola User's Manual, Table 8-9](../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md), the [SingleStep coverage limit](Verification/CPU%20SingleStepTests.md#12-input-scope-and-complete-verification), and the [focused CPU test](../../../crates/cpu/tests/test_dbcc.rs). |
| Dual-memory address error | A faulting source read in `CMPM` or predecrement `ADDX` enters Vector 3 before the destination address register advances. | [CMPM](../../../crates/cpu/src/instructions/cmpm.rs) and [ADDX](../../../crates/cpu/src/instructions/addx.rs) sequences |
| `(An)+` address error | A faulting word/long read retains the postincrement; a faulting write does not increment the register. | [CPU instruction sequences](../../../crates/cpu/src/instructions) |
| `MOVE` to `-(An)` | Prefetch precedes the store. Long stores write the low word first; a rejected transfer preserves only address updates already reached. | [Long MOVE sequence](../../../crates/cpu/src/instructions/move_l.rs) |

---

## 3. Amiga Bus Exception

| Case | Hardware effect | Detail |
| :--- | :--- | :--- |
| `TAS` on Chip or Slow RAM | The Amiga bus drops the memory write phase; condition codes reflect the read, but the stored byte remains unchanged. Other memory banks use the ordinary write path. | [MemoryBus.md](MemoryBus.md#rtc-and-exceptional-access-paths) |

### CPU writes to refresh sync strobes

`STREQU`, `STRVBL`, `STRHOR`, and `STRLONG` are address strobes that Agnus
normally presents during refresh slots to synchronize Denise. The manual also
lists Paula for `STRHOR`. The register matrix records the documented function
of these strobes and the prohibition on Copper writes.
[Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)

A CPU write can select the same offsets, so treating them as unused storage
would discard a possible synchronization effect. [MemoryBus](MemoryBus.md)

The manual establishes the refresh-time strobes. The precise effect on Denise
or Paula of an extra CPU-initiated strobe remains unverified here.
[Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#strequ-strhor-strlong-strvbl)

The current router delivers all four addresses to Denise and Paula.
[Denise](../../../crates/denise/src/registers.rs) and [Paula](../../../crates/paula/src/registers.rs)

Paula handles these addresses through `strobe_sync(address)`. It captures
DMAL on the three line markers, requests VERTB on entry into STREQU, and
leaves its state unchanged on STRLONG. When the CPU selects an address, the
strobe takes effect immediately with no queue entry, even when the data
queues are full. The tests establish these effects in the model. The physical
effect and exact edge timing of an extra CPU strobe remain open.
[Approved receiver model](Paula.md#rga-synchronization-and-vertb)

### Open bus and opposite-direction register access

The router uses fixed open-bus values for unmapped physical memory,
inactive CIA lanes, absent RTC responses, and custom-register addresses
without a modeled read. It returns `$FF` for an undriven byte lane and
`$FFFF` for a word, regardless of which device last drove the data lines.
The value a particular physical bus cycle returns remains open. A read
of `COPJMP1` or `COPJMP2` strobes the Copper in the model and also
returns `$FFFF`.
[Physical memory](Physical%20Memory.md) and [MemoryBus](MemoryBus.md)

`BLTDDAT` and `DSKDATR` are early DMA read addresses. Agnus initiates these
transfers to Chip RAM at the scheduled DMA time. A CPU read of either address
does not fetch the blitter result or disk DMA word. The manual explicitly
says that the microprocessor cannot read `BLTDDAT`. The current `MemoryBus`
returns the fixed open-bus approximation `$FFFF`. The exact value driven
or retained during a physical CPU read cycle remains open.
[Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md)
and [Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#bltddat)

The custom-register ownership matrix lists the documented directions for
access. The manual warns that access in the opposite direction can affect
the device. Reading a write-only register can disturb its contents, and
writing a read-only register can produce unexpected results. The manual
gives no universal value for unused register addresses.
[Custom-register ownership matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md)
and [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md)

The emulator reproduces only explicitly handled side effects. A fixed
`$FFFF` response leaves open whether the corresponding physical read would
disturb the device. Investigation of exact values and other side effects is
deferred. [Deferred bus fidelity work](../../../ROADMAP.md#deferred-bus-and-peripherals)

---

### Gary aliases and inactive CIA lanes

The motherboard decoder uses independent active-low A12/A13 selects for CIA
banks `$A0-$BF` and also routes both `$DE/$DF` custom banks. Inactive CIA
read lanes and unconnected selector B return fixed `$FF`, independent of the
data word last driven on the motherboard. Byte writes reach every selected
CIA regardless of address parity. These rules describe address selection.
The effects of an accepted transfer and queued register capture follow
separate rules. [Motherboard decoder](MemoryBus.md#cia-and-boot-overlay)

Additional model-dependent mirrors, data retention and decay, and external
bus-error behavior remain deferred. [Roadmap 12.1](../../../ROADMAP.md)

## 4. Reference Documentation & Evidence

- [Motorola 68000 instruction prefetch](../Reference/Instruction%20Prefetch%20on%20the%20Motorola%2068000%20Processor.md): Bus ordering and prefetch context.
- [Motorola 68000 Programmer's Reference Manual](../Reference/68000%20Programmer's%20Reference%20Manual): Documented instruction and exception semantics.
- [Motorola 68000 User's Manual, Table 8-9](../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): Distinguishes the 10-, 12-, and 14-clock `DBcc` paths and their read counts.
- [SingleStepTests-680x0](../../../ref_src/SingleStepTests-680x0/68000/v1): Software-generated state and bus-cycle vectors; individual cases still require interpretation.
- [Amiga Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Documents the hazards of opposite-direction and unused custom-register accesses.
- [Amiga Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Defines early DMA reads and identifies `BLTDDAT` and `DSKDATR` as dummy addresses.
