---
title: "Agnus Models and Component Structure"
aliases: ["Agnus", "MOS 8370", "MOS 8371"]
tags: ["amiga", "design", "agnus", "dma", "beam"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-06
updated: 2026-10-13
related: ["[Copper.md](Copper.md)", "[Blitter.md](Blitter.md)", "[DMA.md](DMA.md)", "[Denise.md](Denise.md)"]
tracked_paths:
  - "crates/agnus"
---
# Agnus Models and Component Structure

The emulator implements two OCS Agnus models:

| Chip model | Video standard | Chip RAM limit | Display timing |
| :--- | :--- | :--- | :--- |
| **MOS 8370** | NTSC | **512 KB** (`$000000-$07FFFF`) | 262 lines; alternating 227 and 228 CCK scanlines (`LOL`) |
| **MOS 8371** | PAL | **512 KB** (`$000000-$07FFFF`) | 312 lines; fixed 227 CCK scanlines |

Agnus keeps the beam position that provides the timebase for display and
custom-chip work. It schedules Chip RAM access for the Copper, Blitter,
bitplanes, sprites, audio, disk, and memory refresh. For DMA it generates
memory addresses and selects where data travels; the receiving chip handles
the data for its own function. [DMA](DMA.md) describes a transfer without
duplicating the channel behavior here. This role and the supported models
are separate questions: the table above describes the current emulator
configuration, while the component notes describe hardware behavior.

Agnus uses the vertical fields of `DIWSTRT` and `DIWSTOP` to allow bitplane
DMA only on lines inside the display window. Its scheduler uses `DDFSTRT` and
`DDFSTOP` for the horizontal fetch interval; [Denise](Denise.md) uses the
horizontal DIW fields to clip the pixels produced from those words.

## Component structure

The [`agnus` crate](../../../crates/agnus/src/agnus.rs) coordinates the beam counters and uses three separate workspace crates as components:

- [`dma`](../../../crates/dma/src/dma.rs) schedules DMA slots. Its behavior is specified in [DMA.md](DMA.md).
- [`blitter`](../../../crates/blitter/src/blitter.rs) performs block and line operations. Its behavior is specified in [Blitter.md](Blitter.md).
- [`copper`](../../../crates/copper/src/copper.rs) executes display lists. Its behavior is specified in [Copper.md](Copper.md).

## Reference documentation and source

- [`AgnusModel`](../../../crates/config/src/config.rs) defines the two selectable chip models.
- [`Agnus`](../../../crates/agnus/src/agnus.rs) owns the DMA scheduler, Blitter, and Copper instances.
- [A500/A2000 Technical Reference Manual, Fat Agnus](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md): Explains the beam timebase, DMA controllers, and address generation.
- [DMA.md](DMA.md), [Blitter.md](Blitter.md), and [Copper.md](Copper.md) own the component specifications.
