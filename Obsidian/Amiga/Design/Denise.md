---
title: "Denise"
aliases: ["MOS 8362", "MOS 8373", "Video", "Bitplanes"]
tags: ["amiga", "design", "denise", "video", "display"]
category: "Design"
subsystem: "denise"
status: "active"
created: 2026-09-06
updated: 2026-11-13
related: ["[Sprites.md](Sprites.md)", "[Frame Buffer.md](Frame%20Buffer.md)", "[Agnus.md](Agnus.md)", "[Game Ports.md](Game%20Ports.md)"]
tracked_paths:
  - "crates/denise"
---
# Denise

Denise is the Amiga display chip. It turns bitplane and sprite data supplied through the custom-chip bus into video pixels, combines them using display controls and the color palette, and holds collision state. It also exposes game-port input state. [Agnus](Agnus.md) supplies the display data; [Game Ports](Game%20Ports.md) describes the input connections.

## From Chip RAM to a pixel

A playfield is stored as separate bitplanes in Chip RAM. Each plane contains
one bit for each pixel position. [Agnus](Agnus.md) fetches words from the
planes during the line's data-fetch interval and supplies them to Denise.
Denise shifts the corresponding bits out together: the bits at one position
form a color index, which selects a color from its palette. A word supplies
sixteen successive pixel positions in a plane. The plane pointers advance as
words are fetched, then skip the unused portion of a larger memory image
before the next line. Thus the image in memory may be wider than the portion
shown on screen.

The data-fetch interval determines which memory words become available. Agnus
uses the vertical fields of `DIWSTRT` and `DIWSTOP` to gate bitplane DMA by
line; Denise uses their horizontal fields to clip playfield pixels within a
line. Fetching more data can support scrolling or a wider image, while the
horizontal window still clips what appears. Denise combines the playfield
with [hardware sprites](Sprites.md) according to display priority and records
collisions.

## Amiga 500 display modes

In an ordinary single playfield, up to five planes combine into a palette
index, allowing up to 32 simultaneous selected colors. Low and high horizontal
resolution change the rate at which pixels are emitted; interlace changes the
field pattern vertically. High resolution has less time to fetch each line's
data and supports fewer planes.

Dual-playfield mode groups odd and even planes into two independently
positioned pictures. Each has transparent pixels, and display priority decides
which picture is visible where they overlap. Hold-and-modify (HAM) uses six
planes differently: some pixel codes choose a palette color, while others
change one red, green, or blue component of the preceding pixel's color.
The official playfield chapter describes these modes for the A500; this
overview stays within the behavior supported by that source.

## Components

The Denise crate owns its sprite engine as an internal module. The frame builder remains a separate crate:

| Crate | Role |
| --- | --- |
| [`denise`](../../../crates/denise) | Coordinates video generation, bitplane decoding, display controls, palette, and collision state. `render.rs` selects the final pixel color. |
| [`denise::sprites`](../../../crates/denise/src/sprites.rs) | Models the hardware sprite channels and produces sprite pixels. See [Sprites](Sprites.md), the detailed chapter owned by Denise. |
| [`frame_builder`](../../../crates/frame_builder) | Collects raster pixels into a frame buffer for the host display. See [Frame Buffer](Frame%20Buffer.md). |

## Reconstructed pixel processing

HRM Appendix A BPLxDAT defines BPL1DAT as the parallel-to-serial trigger.
Its committed write captures all six data latches into shifters; another word
is not invented by periodically reloading unchanged latches. High resolution
shifts four bits per CCK; low resolution shifts two and occupies two samples per pixel on the shared
high-resolution framebuffer grid.
BPLCON1 delays odd/even plane streams independently by 0-15 pixel periods,
retaining history across adjacent words. The local line boundary resets
serialization and HAM's held color; it does not defer mid-line register commits.

Palette, dual-playfield, HAM6 and Extra Half-Brite decoding applies to the current
pixel and current committed mode. HAM modifies one component of the preceding
color; EHB halves each four-bit component separately. Sprites advance once per
low-resolution pixel regardless of bitplane resolution. The existing horizontal
window comparisons, blanking region and two-high-resolution-pixel output delay
are retained model timing, without a claim of calibrated silicon edge offsets.

HRM 7 Tables 7-1/7-2 place the two playfields among sprite pairs. A normal
playfield uses PF2P; in dual mode, each nontransparent playfield supplies its own
priority condition and PF2PRI chooses between their colors. Collision logic
uses all participating sprite pairs and serialized plane bits after BPU masks
inactive planes, before visual priority selection. CLXCON enables and match polarities govern odd/even groups
independently of display mode; excluding every plane always satisfies its
match. CLXDAT accumulates all documented pair and plane collisions until a live
read clears it. Debug reads remain neutral.

The [sprite line buffer](Sprites.md#derived-state-and-pixel-evaluation) supplies
potential collision bits alongside its palette index. Applied CLXCON writes
precompute the allowed playfield collision bits for all 64 plane codes. Each
visible output sample combines that table entry with current sprite presence;
precomputation itself never accumulates CLXDAT or fixes a future palette color.

## Register propagation

Concrete queued setters in [registers.rs](../../../crates/denise/src/registers.rs) handle controls, palette, bitplane data and sprite latches with **1 CCK Pipeline** delivery. Address strobes act immediately at accepted selection without a queue entry. Both CPU and DMA BPLxDAT writes reach the same private latch application; committing BPL1DAT transfers the holding latches and arms serialization. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) specifies the common issue boundary. The fixed 64-entry queue exceeds CPU/Copper plus one bitplane DMA word per CCK over its one-CCK delay; every due event is applied once. Engine shifter movement and sampled port inputs remain separate operations.

JOYTEST updates both counter pairs as described under [Digital controller counters](#digital-controller-counters).
Live `CLXDAT` reads clear collision state; `clxdat_debug` inspects it without
acknowledgment or queue advancement.

## Delivered timing and controller inputs

The serializable `DenisePins` state in [pins.rs](../../../crates/denise/src/pins.rs)
is exposed as `denise.pins`. Its `incoming` values retain the RGA event until
the next input sampling phase. Independent game-port levels are sampled directly
before engine work in the current CCK, without another pending copy. It groups delivered RGA strobe events and game port levels separately from
registers, sprite state and the pixel pipeline.

`pins.rga_strobe` holds an unconsumed synchronization event. The
[Machine Loop](Machine%20Loop.md#rga-synchronization-transport) delivers the
captured RGA address before receiver work; no Agnus counter snapshot is passed.
`DeniseSync` stores the local horizontal/line cursor, blank/equalization state,
long-line marker and whether synchronization has begun. CPU sync writes use
the same local operations immediately at accepted selection, without a queue entry.

A transition into equalization begins a field; successive line strobes advance
the local vertical cursor. The first partial field is discarded for geometry
measurement. Subsequent complete fields determine PAL/NTSC height and alternate
interlaced buffer rows. STRLONG marks the current long line without adding a
line. Repeated identical strobes remain distinct events. Snapshot restoration
preserves the cursor, unconsumed input, pixel pipeline and partially drawn
buffer. These receiver counters and retained edge offsets are a functional
model, not measured silicon phase or copies of Agnus's absolute beam position.

`pins.game_port_inputs` holds the delivered raw direction/quadrature levels for both
[game ports](Game%20Ports.md#connector-delivery). The motherboard no longer
assigns JOY registers from device counters. Counting/decoding belongs to
Denise's receiver, not to motherboard transport. Sprite RGA/data words enter
the same named queued setters as bus writes, retain the source on refusal,
and begin receiver delay after the common issue phase. Distinct endpoint words
prove latch delivery without claiming that the old sprite scheduler or pixel
pipeline is correct.

## Digital controller counters

[`game_ports.rs`](../../../crates/denise/src/game_ports.rs) advances both JOY
registers from the current connector levels. Each axis uses the pin-derived
binary phase for its low two bits and wraps its eight-bit count on forward or
backward quadrature transitions. A simultaneous ambiguous two-phase jump
updates the visible pin bits without inventing a direction. JOYTEST writes
`$FCFC` on both registers and preserves the four input-derived low bits,
following HRM Appendix A. No device-side position is copied into a JOY register.

## References

- [General Architecture](General%20Architecture.md): Places Denise and its crates in the machine architecture.
- [Amiga Hardware Reference Manual, Chapter 3](../Reference/Hardware%20Reference%20Manual/03%20-%20Chapter%203%20-%20Playfield%20Hardware.md): Explains bitplanes, fetch and display windows, resolution, dual playfields, and HAM.
- [Amiga Hardware Reference Manual, Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): Describes video priority and collision detection.
- [`crates/denise/src/denise.rs`](../../../crates/denise/src/denise.rs): Living implementation of the Denise coordinator.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Register meanings, named accessors, recipient sets and missing effects.
- [`denise/pins.rs`](../../../crates/denise/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
