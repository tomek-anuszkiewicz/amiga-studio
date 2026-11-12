---
title: "Copper Module"
aliases: ["Copper", "Copper Coprocessor"]
tags: ["amiga", "design", "copper", "agnus"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-19
updated: 2026-11-12
related: ["[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Blitter.md](Blitter.md)"]
tracked_paths:
  - "crates/agnus/src/copper.rs"
  - "crates/agnus/src/copper_control.rs"
  - "crates/agnus/src/copper_registers.rs"
  - "crates/agnus/tests/test_copper_bus_grant.rs"
  - "crates/agnus/tests/test_copper_instructions.rs"
---
# Copper Module

The Copper is part of [Agnus](Agnus.md). It runs a small program synchronized with the display beam, allowing the emulator to change custom chip registers at selected screen positions without the CPU handling each change.

This document is a detailed chapter of [Agnus](Agnus.md). The `agnus::copper` module and its component tests belong to the `agnus` crate.

## The three instructions

A Copper list is a sequence of two-word instructions fetched from Chip RAM by
[Agnus DMA](DMA.md). The list lets the machine change display or other chip
settings at chosen beam positions while the CPU continues its own work.

- **MOVE** sends the value carried by the instruction to a selected custom-chip
  destination. It changes chip state; it is not a general memory-to-memory move.
- **WAIT** holds execution until the beam reaches or passes the selected
  position. The comparison can ignore selected position bits and can also wait
  for the Blitter to finish. While it waits, the Copper does not occupy the
  memory bus.
- **SKIP** tests the same kind of beam condition and, when it is satisfied,
  discards the next Copper instruction instead of pausing.

The Copper restarts its first list during each vertical blank. These
instructions describe what it does; [DMA](DMA.md) describes how its memory
accesses share the bus with other channels.

## Fetch and suspension contract

[HRM Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md#introduction)
requires two instruction words obtained through Copper DMA. Each granted
memory opportunity transfers one word and advances the active pointer by two
bytes within the OCS address range. A denied opportunity changes neither the
instruction latch nor the pointer. Agnus must use its single arbitration result
for that transfer; advancing an independent Copper RAM reader would violate
bus ownership.

In the retained horizontal coordinate convention, fixed peripheral slots use
odd HPOS values and Copper opportunities use even HPOS values. This phase
mapping agrees with the local [reference Copper event model](../../../ref_src/vAmiga/Core/Components/Agnus/Copper/CopperEvents.cpp);
the book's numbered memory cycles must not be equated directly with HPOS parity.
Bitplane transfers can take those opportunities. The existing end-of-line
`$E0` exclusion remains a model constraint, not a newly measured silicon edge.

Disabling Copper DMA suspends its current instruction and pointer. Re-enabling
continues that state; a jump strobe or vertical restart selects a list explicitly.
The WAIT comparator uses the masked eight-bit vertical and horizontal positions
as an ordered pair, with vertical mask bit 7 always enabled. It waits for Blitter
completion when BFD is clear. The impossible `$FFFF/$FFFE` wait follows from
that comparison and the line geometry; it needs no special terminal opcode.
SKIP suppresses exactly the following two-word instruction. A MOVE remains
retained at Agnus's existing common register-issue boundary until accepted.
Reset clears instruction state, enables and pending operations through the
existing lifecycle.

This is the functional reconstruction contract. Exact comparator wake-up edges,
instruction pipeline transients and silicon calibration remain outside the
current acceptance scope; functional mid-line execution and bus exclusion do not.

## Reference documentation and source

- [`copper.rs`](../../../crates/agnus/src/copper.rs) holds Copper state and advances execution. [`copper_control.rs`](../../../crates/agnus/src/copper_control.rs) applies COPCON and list restart strobes.
- [Amiga Hardware Reference Manual, Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md): Defines MOVE, WAIT, SKIP, beam comparison, and list restart.
- [Agnus.md](Agnus.md) describes Agnus and its components.
