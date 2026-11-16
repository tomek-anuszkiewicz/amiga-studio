---
title: "Frame Buffer"
aliases: ["FrameBuilder"]
tags: ["amiga", "design", "frame_builder", "video", "denise"]
category: "Design"
subsystem: "frame_builder"
status: "active"
created: 2026-09-19
updated: 2026-11-17
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

The framebuffer contains a fixed presentation rectangle, independently of DIW
registers, pixel colors, firmware and short or long scanlines and fields.
PAL publishes 720 x 568 samples; NTSC publishes 720 x 480, selected explicitly
for this project's presentation. These dimensions are not digital-raster
requirements of the analog video standards. PAL matches the measured WinUAE
viewport; NTSC uses the separately selected 720 x 480 WinUAE configuration.

The rectangle starts at receiver CCK 49. PAL starts at receiver line 28;
NTSC starts at receiver line 21. With OCS NTSC, hires/double-line output and
centering disabled, the selected rectangle matches WinUAE 6.0.3's unfiltered
Kickstart 1.3 image exactly: 720 x 480 samples starting at (16, 0) in its
752 x 484 native capture, without resizing. These coordinates describe this receiver model,
not calibrated physical beam offsets. Samples before and after the rectangle
still advance all chip pipelines and collisions. Blanked portions inside it
are black. The right edge crosses the horizontal counter wrap: the first two
CCKs of the following short line, or one CCK after a long line, finish the row.

A fixed high-resolution, interlaced spatial grid preserves mixed-resolution
content, including changes within a line. Low-resolution pixels occupy two
horizontal samples; non-interlaced lines occupy two rows. Interlaced fields
occupy alternate rows. Storage reserves the PAL capacity once, with a fixed
row stride of 720 samples. The exposed slice contains exactly `width * height`
samples, without row padding. NTSC reuses the allocation's first 480 rows.
There is no raw raster buffer or separate cropped/expanded presentation image.

Denise's [frame cursor](../../../crates/denise/src/frame_cursor.rs) translates
receiver timing into visible `frame_x` and `frame_y` coordinates. It retains
the preceding row across counter wrap until HBLANK finishes that row; the
frame builder receives final coordinates and owns storage and published geometry.
Completed ARGB samples go directly
to their final columns and, without interlace, both destination rows. Finished
rows are never moved. Missing row tails and rows not painted by a shorter field
are cleared in the current parity; the opposite interlaced field remains intact.

Denise collects the four high-resolution output samples of each CCK before
storing them together. Pixel composition and the two-sample pipeline still
advance in their original order. At the visible edge, only complete pairs within the row are stored.

The [GUI](Application/GUI.md#scaling-and-appearance) reads the visible prefix of
each stored row for texture upload and applies the selected uniform scale.
Saved previews use the same rectangle. Fixed scales may clip the centered
image to the viewport; Fit shows it completely. The GUI does not infer a crop
from DIW or colors.

## Field assembly and snapshots

Denise derives field and visible-line boundaries from received equalization,
blank and horizontal strobes. Counter positions before HBLANK complete the
preceding visible line; its field parity survives the counter wrap. The first
partial field does not publish a completed frame. Machine configuration selects
the fixed standard-specific geometry before execution; frame completion
increments the host refresh generation without changing its dimensions.
The opposite interlaced field remains available, while changing interlace mode
clears incompatible rows.

The buffer, geometry and pending line/field state serialize
together. Restoration preserves both the displayed pixels and subsequent
mid-line output. No pre-release snapshot compatibility is required.

The host can allocate a reusable spare with `FrameBuilder::new_buffer` and
hand off the current allocation through `exchange_buffer`. The replacement must
have the complete fixed storage length. Before swapping allocations, the builder
seeds the replacement with the current pixels, including the opposite field,
unfinished rows and unused NTSC capacity. Logical image and assembly state remain unchanged;
subsequent drawing cannot change the handed-off image. The host must exclusively
own the spare throughout the operation. This API contains no synchronization or
host handles, and snapshots continue to capture the builder's current allocation.

## References

- [`crates/frame_builder`](../../../crates/frame_builder): Buffer implementation.
- [`crates/denise`](../../../crates/denise): Component that owns and fills the buffer.
