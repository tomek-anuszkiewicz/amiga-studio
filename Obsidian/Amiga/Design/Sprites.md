---
title: "Hardware Sprites"
aliases: ["Sprites", "Sprite Engine"]
tags: ["amiga", "design", "sprites", "denise", "video"]
category: "Design"
subsystem: "denise"
status: "active"
created: 2026-09-19
updated: 2026-11-13
related: ["[Denise.md](Denise.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)"]
tracked_paths:
  - "crates/denise/src/sprites.rs"
  - "crates/denise/tests/test_sprites_output.rs"
---
# Hardware Sprites

The Amiga 500 has eight hardware sprite channels for graphics objects that
software can position independently in the video image. Agnus supplies the
sprite data through DMA, and Denise displays the sprites.
[Denise](Denise.md), [Agnus](Agnus.md), [DMA](DMA.md)

The internal [`denise::sprites` module](../../../crates/denise/src/sprites.rs)
models the channels. Sprite component tests are under `crates/denise/tests/`.
This chapter is part of the [Denise specification](Denise.md).

## One sprite across the display

Each sprite channel has a position and a stream of image data in Chip RAM.
The first words fetched determine where the sprite starts and stops. For each
active line, Agnus uses DMA to fetch two image words during horizontal
blanking, before the line becomes visible. Denise latches both words. When the
beam reaches the sprite's horizontal position, Denise shifts out their bits
as sixteen pixels. Each pixel selects one of three sprite colors or
transparency. [Agnus DMA](DMA.md), [Denise](Denise.md)

At the stopping line, the next pair of words can define another use of the same
sprite channel later in the field. When objects overlap, the display priority
circuit combines the sprite and playfield pixels and selects the visible pixel.

## Serial output and manual operation

The reconstructed output follows HRM 4, Manual Mode and the SPRxDATA/CTL
register descriptions. DATA arms the horizontal comparator, and CTL disarms
it. When the horizontal position matches, Denise loads both data latches
into sixteen-pixel shifters. The same armed data can repeat on subsequent
lines, including outside the programmed vertical DMA window. Agnus uses
VSTART/VSTOP for DMA comparisons. Denise displays manually loaded data
independently of those comparisons. The user approved correcting the previous
contrary test criterion on 2026-11-12.

Writes to position, data and control registers take effect when Denise commits
them, including during a line. A new horizontal match can start shifting pixels
again later in the same line. Sprites always use low-resolution pixel periods,
including over high-resolution playfields. The odd channel's ATTACH places
its two color bits above the even channel's two bits. Independent channels use
their pair's palette group. The lowest nontransparent channel or pair has
display priority.

Each even sprite always participates in collision detection. Its odd partner
requires its explicit CLXCON enable even when attached. All six pair-to-pair
collision bits accumulate independently of which color wins display priority.
Denise combines pair presence with the bitplane match conditions from HRM 7,
Tables 7-3 and 7-4.

## Derived state and pixel evaluation

The sprite engine stores channel state and gives inspectors read-only access.
When Denise commits a POS or CTL write, it refreshes the cached horizontal start.
A CTL write for an odd channel also refreshes its attachment state. Denise
keeps the raw CLXCON value alongside the decoded odd-channel collision enables.
The derived values change when the queued write commits, so pixels before
that point use the previous values.
[Denise register propagation](Denise.md#register-propagation)

A derived line buffer stores the winning palette index and pair at each
horizontal position, along with potential sprite/sprite and sprite/playfield
collision bits. The sprite engine waits until the first possible sprite output
to build the buffer, avoiding rebuilds for earlier DMA latch writes. Small
tables decode the spans occupied by sprites. Hidden pairs still contribute to
collision detection. Rendering reads one entry per low-resolution pixel and
uses the current palette and playfield priority.

When a sprite write takes effect, the engine first updates channel state
through the consumed pixel, then invalidates future output. A change to an
odd-channel collision enable also causes the engine to rebuild the buffer the
next time it uses it. CTL prevents future reloads while the current word
finishes, including transparent pixels. The line boundary stops the shifters
and keeps the armed input latches. If no channel is armed or shifting pixels,
the engine returns transparent output without building or reading the buffer.

Denise adds cached collisions to CLXDAT only when it consumes the corresponding
pixel. It tests cached sprite/playfield bits against the current bitplane
output and CLXCON conditions. This preserves high-resolution subpixel matches
and lets live reads clear CLXDAT partway through a line.

Snapshots store channel registers, shifters, counters, armed flags, current
pair presence and the raw CLXCON value. Deserialization rebuilds the derived
positions and attachment flags and leaves the line buffer invalid. When the
engine exposes state for inspection or serializes it, it calculates shifter
and counter values only through the pixels already consumed. It shifts words
when building the buffer or calculating that state, avoiding a shift for every
rendered pixel. Restore does not advance pixels or commit pending writes.
Reset clears both the stored sprite state and the derived state.

## References

- The hardware manual describes the Amiga's sprites.
  [Amiga Hardware Reference Manual, Chapter 4: Sprite Hardware](../Reference/Hardware%20Reference%20Manual/04%20-%20Chapter%204%20-%20Sprite%20Hardware.md)
- The video chip specification describes where sprite display fits in Denise.
  [Denise](Denise.md)
- The DMA specification describes how Agnus supplies sprite data. [DMA](DMA.md)
- The sprite implementation handles channel state, derived controls, pixel
  selection and collision groups. [`sprites.rs`](../../../crates/denise/src/sprites.rs)
