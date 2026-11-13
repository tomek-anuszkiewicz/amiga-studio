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

Hardware sprites are independently positioned graphics objects that appear in the video image. The Amiga 500 has eight sprite channels. Their display circuitry is part of [Denise](Denise.md); [Agnus](Agnus.md) supplies sprite data through [DMA](DMA.md).

The internal [`denise::sprites` module](../../../crates/denise/src/sprites.rs) models these channels. This document is the sprite chapter of the [Denise specification](Denise.md); sprite component tests live under `crates/denise/tests/`.

## One sprite across the display

Each sprite channel has a position and a stream of image data in Chip RAM.
Its first fetched words establish where the sprite starts and stops. For each
line on which it is active, [Agnus DMA](DMA.md) fetches two image words during
horizontal blanking, before that line becomes visible. [Denise](Denise.md)
latches those words. When the beam reaches the sprite's horizontal position,
Denise shifts their bits out as sixteen pixels. Each pixel selects one of
three sprite colors or transparency.

At the stopping line, the next fetched pair can define another use of the
same channel later in the field. Sprite channels can therefore be reused
instead of representing permanently fixed screen objects. The display
priority circuit combines their pixels with the playfield; it decides which
visible pixel wins when objects overlap.

## Serial output and manual operation

The reconstructed output follows HRM 4, Manual Mode and the SPRxDATA/CTL
register descriptions. DATA arms the horizontal comparator; CTL disarms it.
A horizontal match loads both data latches into sixteen-pixel shifters.
The same armed data can repeat on subsequent lines, including outside the
programmed vertical DMA window. Agnus owns VSTART/VSTOP DMA comparisons;
Denise does not use them to suppress manually loaded output. The user approved
correcting the previous contrary test criterion on 2026-11-12.

Position/data/control commits remain visible during a line. A new horizontal
match can restart serialization later in that same line. Sprite output always
uses low-resolution pixel periods, including over high-resolution playfields.
The odd channel's ATTACH combines its two color bits above the even channel's
two bits. Independent channels use their pair palette group; the lowest
nontransparent channel/pair has display priority.

Collision participation differs from visual priority. Each even sprite always
participates; CLXCON explicitly enables its odd partner. Attachment does not
replace that enable. All six pair/pair collision bits accumulate independently
of the winning color; Denise combines pair presence with the bitplane match
conditions from HRM 7, Tables 7-3 and 7-4.

## Derived state and pixel evaluation

The sprite engine owns its channel storage and exposes it for read-only
inspection. Applied POS and CTL writes refresh cached horizontal starts;
odd-channel CTL writes also refresh attachment state. CLXCON retains its raw
value alongside decoded odd-channel collision enables. These derived values
change when [Denise commits the queued write](Denise.md#register-propagation),
so a pending write cannot affect an earlier pixel.

An activity mask selects pairs with at least one armed channel or a word still
being shifted. CTL stops future reloads but lets that word finish, including
transparent pixels. A line boundary ends serialization while retaining armed
input latches. Small tables map the two channel pixels to a pair color and
the four collision-presence bits to CLXDAT pair collisions. Once a color wins,
later pairs skip color selection but still advance and contribute collisions.
Current pair presence remains available to Denise's playfield collision logic.

Snapshots store channel registers, shifters, counters, armed flags, current
pair presence and the raw CLXCON value. Deserialization rebuilds the derived
positions, attachment flags and activity masks from that state without
advancing pixels or committing pending writes. Reset clears both forms.

## References

- [Amiga Hardware Reference Manual, Chapter 4: Sprite Hardware](../Reference/Hardware%20Reference%20Manual/04%20-%20Chapter%204%20-%20Sprite%20Hardware.md): Describes the Amiga's hardware sprites.
- [Denise](Denise.md): Places sprite display within the video chip.
- [DMA](DMA.md): Describes Agnus's role in supplying sprite data.
- [`sprites.rs`](../../../crates/denise/src/sprites.rs): Implements channel state, derived controls, pixel selection and collision groups.
