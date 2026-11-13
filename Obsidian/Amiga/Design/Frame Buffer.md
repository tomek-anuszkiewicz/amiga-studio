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

## Visible storage

The framebuffer contains the complete visible border and playfield, independently
of DIW registers, pixel colors or firmware. Horizontal and vertical blanking
samples are not stored. Scanlines that span a vertical blanking transition are
excluded at both ends of the field, so their partial output cannot form a step
at the image edge. The existing receiver blanking phases are retained; this
storage change does not calibrate physical chip timing.

A fixed high-resolution, interlaced spatial grid preserves mixed-resolution
content, including changes within a line. Low-resolution pixels occupy two
horizontal samples; non-interlaced lines occupy two rows. Interlaced fields
occupy alternate rows. Storage reserves the maximum visible capacity once;
the exposed pixel slice contains only the active rectangle. There is no raw
raster buffer or separate cropped/expanded presentation image.

The [GUI](Application/GUI.md#scaling-and-appearance) uploads this rectangle directly
and fits it uniformly into the display area. It does not infer a crop from DIW
or colors.

## Field assembly and snapshots

Denise derives field and visible-line boundaries from received equalization,
blank and horizontal strobes. Counter positions before HBLANK complete the
preceding visible line; its field parity survives the counter wrap. The first
partial field does not publish measured geometry. Completed visible lines
determine the extent; frame completion increments the host refresh generation.
The opposite interlaced field remains available, while changing interlace mode
clears incompatible rows.

The buffer, geometry and pending line/field state serialize
together. Restoration preserves both the displayed pixels and subsequent
mid-line output. No pre-release snapshot compatibility is required.

## References

- [`crates/frame_builder`](../../../crates/frame_builder): Buffer implementation.
- [`crates/denise`](../../../crates/denise): Component that owns and fills the buffer.
