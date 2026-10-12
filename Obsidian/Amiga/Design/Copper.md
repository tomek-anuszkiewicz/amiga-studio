---
title: "Copper Module"
aliases: ["Copper", "Copper Coprocessor"]
tags: ["amiga", "design", "copper", "agnus"]
category: "Design"
subsystem: "copper"
status: "active"
created: 2026-09-19
updated: 2026-10-12
related: ["[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Blitter.md](Blitter.md)"]
tracked_paths:
  - "crates/copper"
---
# Copper Module

The Copper is part of [Agnus](Agnus.md). It runs a small program synchronized with the display beam, allowing the emulator to change custom chip registers at selected screen positions without the CPU handling each change.

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

## Reference documentation and source

- [`copper.rs`](../../../crates/copper/src/copper.rs) implements the Copper.
- [Amiga Hardware Reference Manual, Chapter 2](../Reference/Hardware%20Reference%20Manual/02%20-%20Chapter%202%20-%20Coprocessor%20Hardware.md): Defines MOVE, WAIT, SKIP, beam comparison, and list restart.
- [Agnus.md](Agnus.md) describes Agnus and its components.
