---
title: "Blitter Module"
aliases: ["Blitter"]
tags: ["amiga", "design", "blitter", "agnus"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-19
updated: 2026-11-12
related: ["[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Paula.md](Paula.md)"]
tracked_paths:
  - "crates/agnus/src/blitter.rs"
  - "crates/agnus/src/blitter_area.rs"
  - "crates/agnus/src/blitter_control.rs"
  - "crates/agnus/src/blitter_line.rs"
  - "crates/agnus/src/blitter_line_engine.rs"
  - "crates/agnus/src/blitter_minterm.rs"
  - "crates/agnus/src/blitter_phase.rs"
  - "crates/agnus/src/blitter_registers.rs"
  - "crates/agnus/tests/test_blitter_area.rs"
  - "crates/agnus/tests/test_blitter_line.rs"
  - "crates/agnus/tests/test_blitter_pointer_width.rs"
---
# Blitter Module

The Blitter is part of [Agnus](Agnus.md). It performs fast block operations on graphics data and draws lines without requiring the CPU to process each pixel.

This document is a detailed chapter of [Agnus](Agnus.md). The `agnus::blitter` module and its component tests belong to the `agnus` crate.

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

## Transfer and operation contract

The reconstruction follows [HRM Chapter 6](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md),
particularly DMA Channels, Shifts and Masks, Area Fill Mode, Pipeline Register
and Line Mode. A source fetch or destination write consumes one Agnus grant.
Internal startup, computation and idle phases advance without claiming Chip
RAM. BLTPRI changes priority for a pending transfer; it does not turn every busy
cycle into a memory transfer. Disabling DMA suspends the operation, including
its pending destination word, until DMA is enabled again.

Area operations fetch enabled sources in A/B/C order. Disabled sources supply
their data latches. The A masks apply before its shift; shifted A and B retain
carry bits between words. A CPU write to BLTBDAT loads its shift holding latch
using the shift control active at that write. All eight minterm inputs determine
the Boolean result. Fill processes that result from bit 0 through bit 15, with
carry retained between words and reloaded from FCI at each row. Appendix A
places inclusive fill at BLTCON1 bit 3 and exclusive fill at bit 4.

The destination pipeline retains a computed word while the following source
set is fetched. This preserves Chapter 6's overlapping-copy ordering: the first
two source sets precede the first destination write. The last pending word must
drain before busy clears and completion is emitted. Source pointers advance on
their reads; the destination pointer advances on its write. Each channel adds
its signed modulo at its own row boundary, with subtraction in descending mode.
No destination channel still performs logic and zero detection on fetched data.

Line mode separates the C read, pixel computation, destination write and
coordinate/error update. The eight octants select major/minor axis directions;
SIGN selects the error increment and conditional minor-axis step. The B data
latch supplies the texture, and SING suppresses additional writes on the same
horizontal row. The book's supported setup enables A/C/D, disables B DMA, uses
width two and initializes the masks to all ones. Exact silicon pipeline offsets
and behavior of undocumented line-mode configurations remain uncalibrated.
Reset and snapshots include source carry, the pending destination, current
phase and line state. The bus/reg-delivery phases remain owned by [DMA](DMA.md)
and [Agnus](Agnus.md).

## Reference documentation and source

- [`blitter.rs`](../../../crates/agnus/src/blitter.rs) holds Blitter state and advances execution. [`blitter_control.rs`](../../../crates/agnus/src/blitter_control.rs) starts an operation when Agnus commits BLTSIZE.
- [Amiga Hardware Reference Manual, Chapter 6](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md): Describes word-based block operations, logic, shifts, fill, line mode, and completion.
- [Agnus.md](Agnus.md) describes Agnus and its components.
