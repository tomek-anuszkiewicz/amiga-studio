---
title: "Frame Buffer"
aliases: ["FrameBuilder"]
tags: ["amiga", "design", "frame_builder", "video", "denise"]
category: "Design"
subsystem: "frame_builder"
status: "active"
created: 2026-09-19
updated: 2026-11-13
related: ["[Denise.md](Denise.md)", "[Agnus.md](Agnus.md)", "[GUI.md](Application/GUI.md)"]
tracked_paths:
  - "crates/frame_builder"
---
# Frame Buffer

The frame buffer holds the pixels of a rendered video frame. In the emulator, `FrameBuilder` is a component of [Denise](Denise.md). [Agnus](Agnus.md) supplies display data and beam timing; Denise uses them to build the image in the buffer. The buffer is intended to pass the rendered frame to the [GUI](Application/GUI.md) for display on screen.

`frame_builder` remains a separate Cargo crate. Denise's [renderer](../../../crates/denise/src/render.rs) selects pixel colors, applies display gating and records collisions. The frame builder receives completed ARGB pixels and stores them in the appropriate field rows; it does not repeat the chip's pixel composition.

## Host color conversion

RGB444 channels expand from 0..15 to 0..255 by multiplication by 17, with
opaque alpha. This host normalization preserves channel order and ignores the
unused upper four bits. It does not apply gamma or CRT correction.

## Field assembly and snapshots

The fixed allocation covers 912 high-resolution pixels by 626 rows, including
two complete 313-line PAL fields. Non-interlaced output maps a receiver line
to one row. Interlaced output maps it to twice the line plus field parity,
retaining the opposite field. The visible buffer dimensions are 313/263 rows
for PAL/NTSC or 626/526 when interlaced; until a complete field is observed,
the receiver uses the maximum PAL geometry. Changing interlace mode clears
stale rows without allocating a new buffer.

Denise derives field boundaries from its received equalization/blank/horizontal
strobes. Frame completion increments a generation counter for host refresh;
CPU/host time does not advance this counter. The buffer and field assembly
state serialize together so restoration preserves already rendered rows and
the opposite interlaced field. No pre-reconstruction snapshot compatibility
is required by the approved 2.4 plan.

## References

- [`crates/frame_builder`](../../../crates/frame_builder): Buffer implementation.
- [`crates/denise`](../../../crates/denise): Component that owns and fills the buffer.
