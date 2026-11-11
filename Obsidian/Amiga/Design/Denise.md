---
title: "Denise"
aliases: ["MOS 8362", "MOS 8373", "Video", "Bitplanes"]
tags: ["amiga", "design", "denise", "video", "display"]
category: "Design"
subsystem: "denise"
status: "active"
created: 2026-09-06
updated: 2026-11-11
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

The implementation spans three workspace crates:

| Crate | Role |
| --- | --- |
| [`denise`](../../../crates/denise) | Coordinates video generation, bitplane decoding, display controls, palette, and collision state. It contains and re-exports the other two crates. |
| [`sprites`](../../../crates/sprites) | Models the hardware sprite channels and produces sprite pixels. See [Sprites](Sprites.md). |
| [`frame_builder`](../../../crates/frame_builder) | Collects raster pixels into a frame buffer for the host display. See [Frame Buffer](Frame%20Buffer.md). |

## Register propagation

Concrete queued setters in [registers.rs](../../../crates/denise/src/registers.rs) handle controls, palette, bitplane data, sprite latches and synchronization strobes. Palette and strobes use a positive **1 CCK provisional** delay. Both CPU and DMA BPLxDAT writes reach the same private latch application; committing BPL1DAT transfers the holding latches and arms serialization. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) specifies the common issue boundary. The fixed 64-entry queue exceeds CPU/Copper plus one bitplane DMA word per CCK over its one-CCK delay; every due event is applied once. Engine shifter movement and sampled port inputs remain separate operations.

JOYTEST and sync reactions remain incomplete; see the [chipset reconstruction work](../../../ROADMAP.md#chipset-reconstruction).
Live `CLXDAT` reads clear collision state; `clxdat_debug` inspects it without
acknowledgment or queue advancement.

## References

- [General Architecture](General%20Architecture.md): Places Denise and its crates in the machine architecture.
- [Amiga Hardware Reference Manual, Chapter 3](../Reference/Hardware%20Reference%20Manual/03%20-%20Chapter%203%20-%20Playfield%20Hardware.md): Explains bitplanes, fetch and display windows, resolution, dual playfields, and HAM.
- [Amiga Hardware Reference Manual, Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): Describes video priority and collision detection.
- [`crates/denise/src/denise.rs`](../../../crates/denise/src/denise.rs): Living implementation of the Denise coordinator.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Register meanings, named accessors, recipient sets and missing effects.
