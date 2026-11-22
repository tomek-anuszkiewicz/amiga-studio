---
title: "Copper Module"
aliases: ["Copper", "Copper Coprocessor"]
tags: ["amiga", "design", "copper", "agnus"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-19
updated: 2026-11-16
related: ["[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Blitter.md](Blitter.md)"]
tracked_paths:
  - "crates/agnus/src/copper.rs"
  - "crates/agnus/src/copper_control.rs"
  - "crates/agnus/src/copper_registers.rs"
  - "crates/agnus/tests/test_copper_bus_grant.rs"
  - "crates/agnus/tests/test_copper_instructions.rs"
---
# Copper Module

The Copper runs a small program in sync with the display beam. This lets the
emulator change custom chip registers at selected screen positions without
requiring the CPU to handle each change. The Copper is part of [Agnus](Agnus.md).

## The three instructions

A Copper list contains two-word instructions. The Copper fetches them from
Chip RAM through [Agnus DMA](DMA.md). As the CPU continues its own work, the
Copper can change display or other chip settings at chosen beam positions.

- **MOVE** changes chip state by writing the value in the instruction to an
  allowed custom-chip register. The Copper reads the destination from the
  instruction and checks that it is allowed before issuing the write. MOVE
  cannot copy data between memory locations.
- **WAIT** holds execution until the beam reaches or passes the selected
  position. The comparison can ignore selected position bits and can also wait
  for the Blitter to finish. While it waits, the Copper does not occupy the
  memory bus.
- **SKIP** tests the same kind of beam condition. If the condition is satisfied,
  the Copper discards the next instruction and continues execution.

The Copper restarts its first list during each vertical blank. How its memory
accesses share the bus with other channels is described in [DMA](DMA.md).

## Fetch and suspension contract

The Copper must fetch both words of each instruction through Copper DMA, as
specified in [HRM Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md#introduction).
When Agnus grants a memory slot, the Copper transfers one word and advances
the active pointer by two bytes within the OCS address range. When Agnus denies
the slot, the Copper keeps the instruction latch and pointer as they were.
The transfer must use the same arbitration result as the rest of Agnus. An
independently advancing Copper RAM reader would violate bus ownership.

In the retained horizontal coordinate convention, odd HPOS values identify
fixed peripheral slots and even HPOS values identify Copper opportunities.
Bitplane transfers can use those Copper opportunities. This mapping agrees
with the local [reference Copper event model](../../../ref_src/vAmiga/Core/Components/Agnus/Copper/CopperEvents.cpp).
When comparing the two, keep the book's numbered memory cycles distinct from
HPOS parity. The existing end-of-line `$E0` exclusion comes from a model
constraint and carries no new silicon measurement.

Disabling Copper DMA suspends execution and preserves the current instruction
and pointer. Re-enabling Copper DMA resumes execution from that state. A jump
strobe or vertical restart explicitly selects a list.
The WAIT comparator uses the masked eight-bit vertical and horizontal positions
as an ordered pair, with vertical mask bit 7 always enabled. It waits for Blitter
completion when BFD is clear. The masked comparison and line geometry are
sufficient to make the `$FFFF/$FFFE` wait impossible to satisfy. This case needs
no special terminal opcode. SKIP suppresses exactly the following two-word
instruction. Agnus holds a MOVE at the existing common point for issuing
register operations until the operation is accepted. The existing reset
lifecycle clears instruction state, enables and pending operations.

The functional reconstruction requires mid-line execution and bus exclusion.
Exact comparator wake-up edges, instruction pipeline transients and silicon
calibration remain outside the current acceptance scope.

## Reference documentation and source

- [`copper.rs`](../../../crates/agnus/src/copper.rs) holds Copper state and advances execution. [`copper_control.rs`](../../../crates/agnus/src/copper_control.rs) applies COPCON and list restart strobes.
- MOVE, WAIT, SKIP, beam comparison and list restart are specified in the [Amiga Hardware Reference Manual, Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md).
- [Agnus.md](Agnus.md) describes Agnus and its components.
