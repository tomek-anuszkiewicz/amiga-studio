---
title: "Blitter Module"
aliases: ["Blitter"]
tags: ["amiga", "design", "blitter", "agnus"]
category: "Design"
subsystem: "blitter"
status: "active"
created: 2026-09-19
updated: 2026-10-12
related: ["[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Paula.md](Paula.md)"]
tracked_paths:
  - "crates/blitter"
---
# Blitter Module

The Blitter is part of [Agnus](Agnus.md). It performs fast block operations on graphics data and draws lines without requiring the CPU to process each pixel.

## How a blit works

The Blitter works on 16-bit words in Chip RAM. A block operation visits a
rectangle of words row by row. Three possible input streams, conventionally
called A, B, and C, feed a programmable Boolean combination; a fourth stream,
D, receives the result. A source can instead contribute a held constant, so
copying, masking, and combining images do not always require all three reads.
Each stream can advance through a different region of memory and skip to its
next row after the current row is processed.

Before combining words, the Blitter can shift source bits across word
boundaries and mask the edges of a rectangle. It can also fill areas between
edge bits. A separate line mode follows an error accumulator to place pixels
along a line, using the same memory and logic machinery. These operations run
asynchronously with the CPU and use Chip RAM cycles granted by [Agnus DMA](DMA.md).
The result is ready only when the operation finishes; the Blitter then exposes
completion status and can request an interrupt through [Paula](Paula.md).

## Reference documentation and source

- [`blitter.rs`](../../../crates/blitter/src/blitter.rs) implements the Blitter.
- [Amiga Hardware Reference Manual, Chapter 6](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md): Describes word-based block operations, logic, shifts, fill, line mode, and completion.
- [Agnus.md](Agnus.md) describes Agnus and its components.
