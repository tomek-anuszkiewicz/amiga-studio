---
title: "Architecture Specification: M68000 Cycle-Exact Micro-Step State Machine"
aliases: ["CPU Micro-Step State Machine", "Micro-Step State Machine", "68000 Microcode Model"]
tags: ["amiga", "design", "m68000", "microstep", "cycle-exact"]
category: "Design"
subsystem: "m68000"
status: "active"
created: 2026-09-08
updated: 2026-11-17
related: ["[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[CPU SingleStepTests.md](Verification/CPU%20SingleStepTests.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/cpu/src/micro"
  - "crates/cpu/src/instructions"
  - "crates/cpu/src/cpu.rs"
  - "crates/cpu/src/bus_guard.rs"
---
# Architecture Specification: M68000 Cycle-Exact Micro-Step State Machine

The Motorola 68000 is a CISC processor that uses microcode for internal control.
In the emulator, an ordered sequence of `MicroStep` actions controls
computation, bus access and elapsed time. The scope is the timing and observable
order of execution. Motorola's actual microcode ROM and electrical pin waveform
are outside that scope.

The [CPU specification](CPU%20Motorola%20M68000.md) describes the processor's
external contract. This note looks inside the emulator's execution engine.
[MemoryBus](MemoryBus.md) routes accesses, and the
[Machine Loop](Machine%20Loop.md) advances the CPU with the Amiga chips.

## Microsteps and dispatch

At instruction entry, the CPU uses the opcode to select a static descriptor
from the 65,536-entry [dispatch table](../../../crates/cpu/src/micro/dispatch_table.rs).
The descriptor supplies decoded register indices and an ordered slice of
`MicroStep` values for the CPU to cache. Each step has three parts.
[Microstep descriptor](../../../crates/cpu/src/micro/types.rs)

| Part | Role |
| --- | --- |
| `alu_fn` | Optional internal action on `CpuState`: arithmetic, effective-address calculation, flags, or path selection. It does not call `AddressBus`. |
| `bus_fn` | Optional action with access to `Cpu` and `AddressBus`. It returns `Ready` or `WaitState`. |
| `base_clocks` | Assigned CPU clocks. An ordinary CCK step uses two; a zero-clock ALU action can run within the current CCK. |

An ALU callback can run as part of a timed bus step. The engine calls it once
when the step starts, before any bus attempt. The engine can continue in the
same CCK after a zero-clock ALU step. A condition or fault can select a
different static slice. The callbacks specify the order within the emulator.
The location of arithmetic within a physical clock is outside this model.

Shared operand, prefetch, stack, and control-flow phases live in
[`common.rs`](../../../crates/cpu/src/micro/common.rs),
[`step_execution.rs`](../../../crates/cpu/src/micro/step_execution.rs), and
[`step_control.rs`](../../../crates/cpu/src/micro/step_control.rs). Individual
instruction modules arrange those phases.

## One CCK through the engine

While execution is active, each `Cpu::step_cck` call advances one Color Clock
(CCK), equal to two CPU clocks. A call advances the
[engine](../../../crates/cpu/src/cpu.rs) through the following steps.

1. At an instruction boundary, prepare the descriptor for `ir`. A stopped or
   halted CPU does not execute an active step. An eligible interrupt can wake
   a stopped CPU.
2. Add two to the CPU clock counter. On first entering the active step, load
   `base_clocks` into `clocks_remaining` and invoke `alu_fn` once.
3. Follow an ALU-selected slice immediately. Pass through zero-clock ALU steps
   in the same CCK until a timed action is reached.
4. Invoke `bus_fn` if present. `Ready` consumes two clocks from the step
   countdown. Once it reaches zero, advance `micro_step`. An internal step
   without `bus_fn` consumes its assigned time in the same way.
5. If the bus returns `WaitState`, leave the step and countdown in place.
   Retry that bus action on a later machine CCK.

When the slice ends, the engine retires the instruction, resets its temporary
state and prepares the next instruction or pending exception processing.

Each attempted active CCK consumes two CPU clocks, even if the bus refuses a
transfer. The refusal commits no transfer and leaves the step and its countdown
in place. The engine retries the bus action without running the entry ALU
callback again. For a data-dependent internal interval, the engine runs the
countdown without a memory callback. A dynamic handler such as `MOVEM` can
stay on the same step while advancing its own transfer cursor.

## Bus admission guard

CCK stepping and instruction stepping use the same execution primitive.
It wraps the address bus with one admission record per CCK and counts each
normal byte or word attempt before dispatch, including waits and suppressed
TAS stores. Debug prefetch and setup reads are outside this budget. A second
attempt in the same CCK fails before dispatch, even if the bus refused the
first.

The CPU retains the start time of an accepted bus cycle across CCKs and
snapshots. A read callback marks the start directly. For a write callback, the
CPU subtracts one CCK using wrapping arithmetic. Under this model timestamp
convention, accepted starts must be at least two CCKs apart. The timestamp
expresses callback placement and provides no silicon strobe measurement.

A completed read reserves the following CCK for completion. A new write also
needs its own setup CCK. A read can follow a completed write in the next CCK,
while successive reads or successive writes need two CCKs. On a wait, the CPU
retains the attempted address, width, direction and write value until the bus
accepts the transfer. Replacing a pending transaction or shortening its cycle
is a fatal internal invariant failure. Before advancing the instruction
boundary, retirement checks that the final read consumed its completion CCK
and that the bus accepted every pending transfer. These checks preserve the
callback placement below. Ordinary arbitration waits still apply, and silicon
strobe timing remains outside the evidence they provide.

## Bus transfer and contention

An ordinary byte or word bus cycle occupies **four CPU clocks: two CCKs**.
One CCK invokes the memory callback and the other provides setup or completion.
Both count toward the complete bus cycle.

| Transfer | First CCK: two clocks | Second CCK: two clocks |
| --- | --- | --- |
| Read | Attempt `read_byte` or `read_word` and latch successful data. | Finish the transfer and apply deferred updates; no second read. |
| Write | Prepare address and data, including alignment checks; no write callback. | Attempt `write_byte` or `write_word` and apply success-dependent updates. |

If the bus blocks a read, the CPU retries its first CCK. It retries the second
CCK for a blocked write. The pending address, data, step and phase stay in
place. A long transfer uses two ordered word cycles. If the second word waits,
the CPU retries that cycle only. An odd word or long address redirects to
Address Error before a rejected memory callback.

This two-phase arrangement leaves one CCK per ordinary transfer without a CPU
memory callback. Agnus schedules DMA and reports Chip RAM contention in CCK
units. The [machine loop](Machine%20Loop.md#one-color-clock) advances other
chips, passes contention through
[MemoryBus](MemoryBus.md#a-cpu-access-through-the-router), and steps the CPU.
When access is blocked, only the CPU transfer waits. The other chips continue.
That allows alternating CPU and specialized-chip activity in the machine
model. The two CPU clocks assigned to the callback phase describe its place
in the model. The duration of the physical transfer and the timing of
individual bus strobes require hardware evidence.

## Internal registers and execution state

The [programmer-visible registers](CPU%20Motorola%20M68000.md#programmer-visible-state)
are separate from the engine's staged operands, prefetch and transfer progress.
`instruction_pc` identifies the current opcode for relative addressing and fault
capture. The stream cursor may already point beyond it. The implementation
defines the scratch storage in the [micro-state](../../../crates/cpu/src/micro/engine.rs).

When an instruction retires in sequence, the engine shifts `prefetch` and
`irc` and advances PC. A control transfer refills the queue from its target
and commits the new opcode and cursor. Some sequences advance the prefetch
queue before their final memory write. In these cases, `prefetch_retired`
prevents a second shift. Dynamic branches and exceptions can redirect the
active slice without restarting the machine clock. The
[SingleStep contract](Verification/CPU%20SingleStepTests.md) constrains
instruction behavior. The [CPU specification](CPU%20Motorola%20M68000.md)
describes the external exception state.

The snapshot records the active static slice by its execution identity. The
identity selects a descriptor-table sequence or a named sequence used after
a branch or exception redirects execution. Restoration uses that identity
directly without decoding the current `ir` again. The restored execution
keeps the sequence, step index, staged operands and bus admission state.
This matters after early prefetch retirement as well as during exceptions.
The saved encoding contains no function address.
The [continuation tests](../../../crates/cpu/tests/test_snapshot_continuation.rs)
compare future state, memory and timed bus transfers at matching CCK boundaries,
including scripted wait states. The results concern continuation after restore.
Absolute hardware timing requires separate verification.

## Reference Documentation & Upstream Ground Truth

- The physical bus cycles that give context for the callback model are described in [Motorola User's Manual Section 5](../Reference/68000%20User's%20Manual/05%20-%20Section%205%20-%2016-Bit%20Bus%20Operations%20%28Read,%20Write,%20RMW%29.md).
- Aggregate clocks and transfer counts appear in [Motorola User's Manual Section 8](../Reference/68000%20User's%20Manual/08%20-%20Section%208%20-%2016-Bit%20Instruction%20Execution%20Timing%20%26%20Bus%20Tables.md).
- The generated-vector timing and callback comparisons and their evidence limits are described in [CPU SingleStep verification](Verification/CPU%20SingleStepTests.md).
- The model is implemented in the [descriptors](../../../crates/cpu/src/micro/types.rs), [micro-state](../../../crates/cpu/src/micro/engine.rs) and [stepping code](../../../crates/cpu/src/cpu.rs).
- Tests with a synthetic bus provide evidence for CCK transfers and waits. [Bus phase tests](../../../crates/cpu/tests/test_cck_bus.rs)
