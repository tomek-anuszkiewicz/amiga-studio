---
title: "Hardware Sprites"
aliases: ["Sprites", "Sprite Engine"]
tags: ["amiga", "design", "sprites", "denise", "video"]
category: "Design"
subsystem: "sprites"
status: "active"
created: 2026-09-19
updated: 2026-10-12
related: ["[Denise.md](Denise.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)"]
tracked_paths:
  - "crates/sprites"
---
# Hardware Sprites

Hardware sprites are independently positioned graphics objects that appear in the video image. The Amiga 500 has eight sprite channels. Their display circuitry is part of [Denise](Denise.md); [Agnus](Agnus.md) supplies sprite data through [DMA](DMA.md).

In the emulator, the [`sprites` crate](../../../crates/sprites/src/sprites.rs) models these channels. It is included by the [`denise` crate](../../../crates/denise/src/denise.rs), which owns the sprite subsystem.

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

## References

- [Amiga Hardware Reference Manual, Chapter 4: Sprite Hardware](../Reference/Hardware%20Reference%20Manual/04%20-%20Chapter%204%20-%20Sprite%20Hardware.md): Describes the Amiga's hardware sprites.
- [Denise](Denise.md): Places sprite display within the video chip.
- [DMA](DMA.md): Describes Agnus's role in supplying sprite data.
