---
title: "Blitter Module"
aliases: ["Blitter"]
tags: ["amiga", "design", "blitter", "agnus"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-19
updated: 2026-11-16
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

The Blitter performs fast block operations on graphics data and draws lines
without requiring the CPU to process each pixel. It is part of [Agnus](Agnus.md).


## How a blit works

The Blitter works on 16-bit words in Chip RAM. During a block operation, it
processes a rectangle of words row by row. A programmable Boolean operation
combines data from the three possible sources, A, B, and C, and sends the
result to the destination stream, D. When copying, masking or combining
images, the Blitter can supply a held constant for any source in place of a
memory read. Each stream can use its own memory region and skip to its next
row after processing the current one.

Before combining words, the Blitter can shift source bits across word
boundaries and mask the rectangle's edges. It can also fill areas between
edge bits. In line mode it uses the same memory and logic circuitry, and an
error accumulator guides pixel placement along the line. The Blitter runs
asynchronously with the CPU and uses granted Chip RAM cycles. [Agnus DMA](DMA.md)

The result is ready once the operation finishes. At that point the Blitter
exposes completion status and can request an interrupt through [Paula](Paula.md).

## Transfer and operation contract

The reconstruction follows the manual's DMA Channels, Shifts and Masks, Area
Fill Mode, Pipeline Register and Line Mode sections. [HRM Chapter 6](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md)

The Blitter consumes one Agnus grant for each source fetch or destination
write. Internal startup, computation and idle phases advance without claiming
Chip RAM. BLTPRI only changes priority for a pending transfer. Disabling DMA
suspends the operation, including any pending destination word, until DMA is
enabled again.

In an area operation, the Blitter fetches enabled sources in A/B/C order and
uses the value in the data latch for each disabled source. It applies the A
masks before shifting A. Shifted A and B keep their carry bits between words.
A CPU write to BLTBDAT loads the shift holding latch using the shift control
active at that write. All eight minterm inputs determine the Boolean result.
Fill processes the result from bit 0 through bit 15, keeping the carry between
words and reloading it from FCI at each row. Appendix A places inclusive fill
at BLTCON1 bit 3 and exclusive fill at bit 4.

The destination pipeline holds the computed word while the Blitter fetches
the next set of source words. The first two source sets therefore arrive before
the first destination write, preserving the overlapping-copy order in Chapter
6. The Blitter clears busy status and signals completion only after the last
pending word leaves the pipeline. Source pointers advance on reads, and the
destination pointer advances on writes. Each channel adds its signed modulo
at its own row boundary. In descending mode it subtracts the signed modulo.
With the destination channel disabled, the Blitter still performs logic and
zero detection using the fetched data.

In line mode, the Blitter separates the C read, pixel calculation, destination
write and coordinate/error update. The eight octants select the directions of
the major and minor axes. SIGN selects the error increment and the conditional
minor-axis step. The B data latch supplies the texture. SING suppresses
additional writes on the same horizontal row. The setup supported by the manual
enables A/C/D, disables B DMA, uses width two and initializes the masks to all
ones. Exact silicon pipeline offsets and the behavior of undocumented
line-mode configurations remain uncalibrated.

Reset and snapshot handling for the Blitter cover source carry, the pending
destination word, the current phase and line state. DMA and Agnus handle the
bus and register delivery phases. [DMA](DMA.md), [Agnus](Agnus.md)

## Reference documentation and source

- [`blitter.rs`](../../../crates/agnus/src/blitter.rs) holds Blitter state and advances execution. [`blitter_control.rs`](../../../crates/agnus/src/blitter_control.rs) starts an operation when Agnus commits BLTSIZE.
- [Amiga Hardware Reference Manual, Chapter 6](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md): Describes word-based block operations, logic, shifts, fill, line mode, and completion.
- [Agnus.md](Agnus.md) describes Agnus and its components.
