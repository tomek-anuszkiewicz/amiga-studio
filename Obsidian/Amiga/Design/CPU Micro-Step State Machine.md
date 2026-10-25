---
title: "Architecture Specification: M68000 Cycle-Exact Micro-Step State Machine"
aliases: ["CPU Micro-Step State Machine", "Micro-Step State Machine", "68000 Microcode Model"]
tags: ["amiga", "design", "m68000", "microstep", "cycle-exact"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-08
updated: 2026-10-25
related: ["[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[CPU SingleStepTests.md](Verification/CPU%20SingleStepTests.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/cpu/src/micro"
  - "crates/cpu/src/instructions"
  - "crates/cpu/src/cpu.rs"
---
# Architecture Specification: M68000 Cycle-Exact Micro-Step State Machine

The Motorola 68000 is a CISC processor whose internal control uses microcode.
This emulator represents that idea with
ordered `MicroStep` actions for computation, bus access, and elapsed time. It
models the timing and observable order of execution, not Motorola's actual
microcode ROM or electrical pin waveform.

The [CPU specification](CPU%20Motorola%20M68000.md) describes the processor's
external contract. This note looks inside the emulator's execution engine.
[MemoryBus](MemoryBus.md) routes accesses, and the
[Machine Loop](Machine%20Loop.md) advances the CPU with the Amiga chips.

## Microsteps and dispatch

An opcode selects a static descriptor from the 65,536-entry
[dispatch table](../../../crates/cpu/src/micro/dispatch_table.rs). It supplies
an ordered slice of `MicroStep` values and decoded register indices. The CPU
caches that slice at instruction entry. A
[microstep descriptor](../../../crates/cpu/src/micro/types.rs) has three parts:

| Part | Role |
| --- | --- |
| `alu_fn` | Optional internal action on `CpuState`: arithmetic, effective-address calculation, flags, or path selection. It does not call `AddressBus`. |
| `bus_fn` | Optional action with access to `Cpu` and `AddressBus`. It returns `Ready` or `WaitState`. |
| `base_clocks` | Assigned CPU clocks. An ordinary CCK step uses two; a zero-clock ALU action can run within the current CCK. |

An ALU callback can be fused with a timed bus step. It runs once when the step
starts, before any bus attempt. A zero-clock ALU step can immediately continue
to the next step. A condition or fault can select a different static slice.
These callbacks specify emulator ordering; they do not locate the physical
silicon's arithmetic within a clock.

Shared operand, prefetch, stack, and control-flow phases live in
[`common.rs`](../../../crates/cpu/src/micro/common.rs),
[`step_execution.rs`](../../../crates/cpu/src/micro/step_execution.rs), and
[`step_control.rs`](../../../crates/cpu/src/micro/step_control.rs). Individual
instruction modules arrange those phases.

## One CCK through the engine

`Cpu::step_cck` advances one Color Clock (CCK), equal to two CPU clocks, while
execution is active. The [engine](../../../crates/cpu/src/cpu.rs) proceeds as
follows:

1. At an instruction boundary, prepare the descriptor for `ir`. A stopped or
   halted CPU does not execute an active step; an eligible interrupt can wake
   a stopped CPU.
2. Add two to the CPU clock counter. On first entering the active step, load
   `base_clocks` into `clocks_remaining` and invoke `alu_fn` once.
3. Follow an ALU-selected slice immediately. Pass through zero-clock ALU steps
   in the same CCK until a timed action is reached.
4. Invoke `bus_fn` if present. `Ready` consumes two clocks from the step
   countdown. Once it reaches zero, advance `micro_step`. An internal step
   without `bus_fn` consumes its assigned time in the same way.
5. `WaitState` returns with the same step and countdown. A later machine CCK
   retries that bus action. At the end of the slice, retire the instruction,
   reset its temporary state, and prepare the next instruction or pending
   exception processing.

Each attempted active CCK consumes two CPU clocks, even if a bus attempt is
refused. A refusal neither repeats the entry ALU nor advances the step, reduces
its countdown, or commits the rejected transfer. A data-dependent internal
interval uses the countdown without a memory callback. A dynamic handler such
as `MOVEM` can hold the same step while its own transfer cursor advances.

## Bus transfer and contention

An ordinary byte or word bus cycle occupies **four CPU clocks: two CCKs**.
Only one of those CCKs invokes the memory callback; the other provides setup
or completion. Both count toward the complete bus cycle.

| Transfer | First CCK: two clocks | Second CCK: two clocks |
| --- | --- | --- |
| Read | Attempt `read_byte` or `read_word` and latch successful data. | Finish the transfer and apply deferred updates; no second read. |
| Write | Prepare address and data, including alignment checks; no write callback. | Attempt `write_byte` or `write_word` and apply success-dependent updates. |

A blocked read retries its first CCK; a blocked write retries its second.
The pending address, data, step and phase stay in place. A long transfer consists
of two ordered word cycles, so a completed first word is not repeated if the
second waits. An odd word/long address redirects to Address Error before a
rejected memory callback.

This two-phase arrangement leaves one CCK per ordinary transfer without a CPU
memory callback. Agnus schedules DMA and reports Chip RAM contention in CCK
units. The [machine loop](Machine%20Loop.md#one-color-clock) advances other
chips, passes contention through
[MemoryBus](MemoryBus.md#a-cpu-access-through-the-router), and steps the CPU.
When access is blocked, only the CPU transfer waits; the other chips continue.
That allows alternating CPU and specialized-chip activity in the machine
model. Callback placement does not mean that physical data transfer lasts
exactly two CPU clocks or expose individual bus strobes.

## Internal registers and execution state

[`CpuState`](../../../crates/cpu/src/state.rs) holds programmer-visible Dn,
An, PC, and SR alongside execution state. `ir` is the executing opcode,
`prefetch` the staged next word, and `micro.irc` an incoming word.
`instruction_pc` identifies the current opcode for relative addresses and
fault capture; `cycle_counter` counts elapsed CPU clocks. `stopped`, `halted`,
`trace_pending`, and `ipl` affect instruction-boundary behavior. These fields
are emulator state, not a claim about one-to-one silicon registers.

[`CpuMicroState`](../../../crates/cpu/src/micro/engine.rs) carries values
between CCKs:

| Fields | Purpose |
| --- | --- |
| `current_steps`, `micro_step`, `clocks_remaining` | Active sequence, position, and remaining time. |
| `reg_src`, `reg_dst` | Decoded register indices. |
| `source`, `destination` | Staged operands and result; exception entry can reuse them for captured values. |
| `ea_addr`, `ea_high`, `addr1`, `addr2` | Resolved address, partial high word, and addresses for multi-operand or split transfers. |
| `irc`, `target_refill`, `prefetch_retired` | Incoming instruction word and controls for prefetch retirement. |
| `movem_mask`, `movem_state` | Register mask and transfer-loop progress. |
| `fault_addr`, `info_word`, `ssp_base` | Address Error capture and original supervisor stack base. |

Sequential retirement shifts `prefetch` and `irc` and advances PC. A control
transfer refills from its target and commits the new opcode and cursor. Some
sequences advance the prefetch queue before their final memory write;
`prefetch_retired` prevents a second shift. Dynamic branches and exceptions
can redirect the active slice without restarting the machine clock. The
[SingleStep contract](Verification/CPU%20SingleStepTests.md) constrains
instruction behavior; the [CPU specification](CPU%20Motorola%20M68000.md)
describes the external exception state.

The micro-state is serializable except for `current_steps`, a runtime pointer
to a static slice. [Restoration](../../../crates/cpu/src/cpu.rs) rebuilds it
from `ir`. A snapshot taken during a redirected branch or exception, or after
an early prefetch has changed `ir` while a write remains pending, may therefore
restore the wrong active slice. Arbitrary in-flight restoration is not
guaranteed.

## Reference Documentation & Upstream Ground Truth

- [Motorola User's Manual Section 5](../Reference/68000%20User's%20Manual/05%20-%20Section%205%20-%2016-Bit%20Bus%20Operations%20%28Read,%20Write,%20RMW%29.md): Physical bus-cycle context, distinct from emulator callbacks.
- [Motorola User's Manual Section 8](../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md): Aggregate clocks and transfer counts.
- [CPU SingleStep verification](Verification/CPU%20SingleStepTests.md): Generated-vector timing and callback comparisons with stated evidence limits.
- [Descriptors](../../../crates/cpu/src/micro/types.rs), [micro-state](../../../crates/cpu/src/micro/engine.rs), and [stepping](../../../crates/cpu/src/cpu.rs): Living implementation of this model.
- [Bus phase tests](../../../crates/cpu/tests/test_cck_bus.rs): Synthetic evidence for CCK transfers and waits.
