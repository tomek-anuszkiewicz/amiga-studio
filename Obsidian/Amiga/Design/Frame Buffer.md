---
title: "Frame Buffer"
aliases: ["FrameBuilder"]
tags: ["amiga", "design", "frame_builder", "video", "denise"]
category: "Design"
subsystem: "frame_builder"
status: "active"
created: 2026-09-19
updated: 2026-11-23
related: ["[Denise.md](Denise.md)", "[Agnus.md](Agnus.md)", "[GUI.md](Application/GUI.md)"]
tracked_paths:
  - "crates/frame_builder"
---
# Frame Buffer

The frame buffer holds rendered video pixels for display by the
[GUI](Application/GUI.md). [Agnus](Agnus.md) supplies display data and beam
timing. [Denise](Denise.md) uses them to build the image and stores it in
its `FrameBuilder` component.

`frame_builder` remains a separate Cargo crate. Denise's
[renderer](../../../crates/denise/src/render.rs) selects pixel colors, gates display
output and records collisions. The frame builder's role is to store the
completed ARGB pixels in the appropriate field rows.

## Host color conversion

For host output, the conversion multiplies each RGB444 channel by 17 to expand
0..15 to 0..255 and sets alpha to opaque. It keeps the channel order, ignores the
unused upper four bits and leaves gamma and CRT correction unapplied.

## Visible storage

The framebuffer contains a fixed presentation rectangle. DIW registers, pixel
colors, firmware and short or long scanlines and fields leave its size and
position unchanged. The project selects 720 x 568 samples for PAL and 720 x 480
for NTSC. These are presentation dimensions chosen for the emulator. The PAL
rectangle matches the measured WinUAE viewport, and the NTSC rectangle uses
the separately selected 720 x 480 WinUAE configuration. The analog standards
do not require these digital raster dimensions.

The rectangle starts at receiver CCK 49. It starts at receiver line 28 for PAL
and receiver line 21 for NTSC. With OCS NTSC, hires/double-line output and
centering disabled, the rectangle matches WinUAE 6.0.3's unfiltered Kickstart
1.3 image exactly. The matching region is 720 x 480 samples starting at (16, 0)
in the 752 x 484 native capture and needs no resizing. These coordinates describe the receiver model. Their relationship to the
physical beam remains uncalibrated. Chip pipelines and
collision detection continue to advance outside the rectangle. Blanked parts
inside it appear black. The right edge crosses the horizontal counter wrap.
With visible output, the row finishes two CCKs after a short line and one
CCK after a long line.

A fixed high-resolution, interlaced spatial grid preserves mixed-resolution
content, including changes within a line. A low-resolution pixel occupies two consecutive horizontal samples. A
non-interlaced line occupies two rows. Interlaced fields
occupy alternate rows. Storage reserves the PAL capacity once, with a fixed
row stride of 720 samples. The exposed slice contains exactly `width * height`
samples, without row padding. NTSC reuses the allocation's first 480 rows.
There is no raw raster buffer or separate cropped/expanded presentation image.

Denise's [frame cursor](../../../crates/denise/src/frame_cursor.rs) translates
receiver timing into visible `frame_x` and `frame_y` coordinates. After the
counter wraps, the cursor keeps using the preceding row until HBLANK completes
it. The frame builder receives the final coordinates. It owns the pixel
storage and the geometry exposed to the host. It writes completed ARGB
samples directly to their final columns. With interlace off, it writes each
sample to both destination rows.
Finished rows stay where they were written. The builder clears missing row
tails and rows a shorter field did not paint in the current parity. The
opposite interlaced field stays intact.

Denise collects the four high-resolution output samples of each CCK before
storing them together. Pixel composition and the two-sample pipeline still
advance in their original order. At the visible edge, only complete pairs within the row are stored.

The [GUI](Application/GUI.md#scaling-and-appearance) reads the visible prefix of
each stored row for texture upload and applies the selected uniform scale.
Saved previews use the same rectangle. A fixed scale may clip the centered image at the viewport edges. Fit shows
the whole image. The GUI does not infer a crop
from DIW or colors.

## Field assembly and snapshots

Denise derives field and visible-line boundaries from received equalization,
blank and horizontal strobes. After a line strobe, counter positions before
HBLANK still belong to the preceding row and keep its field parity. Vertical
blanking can suppress the final pixels at the field boundary. The first
partial field does not publish a completed frame. Before execution, machine configuration selects the fixed geometry for the
video standard. Each completed frame increments the host refresh generation
and keeps the dimensions unchanged.
The opposite interlaced field remains available, while changing interlace mode
clears incompatible rows.

The buffer, geometry and pending line/field state serialize
together. Restoration preserves both the displayed pixels and subsequent
mid-line output. No pre-release snapshot compatibility is required.

The host can allocate a reusable spare with `FrameBuilder::new_buffer` and
take ownership of the current allocation through `exchange_buffer`. The spare
must have the complete fixed storage length, and the host must own it
exclusively throughout the operation. Before swapping allocations, the
builder copies its current pixels into the spare, including the opposite
field, unfinished rows and unused NTSC capacity. The swap preserves the
logical image and assembly state. Later drawing uses the replacement
allocation and leaves the handed-off image intact. The API has no
synchronization or host handles. Snapshots continue to capture the builder's
current allocation.

## References

- [`crates/frame_builder`](../../../crates/frame_builder): Buffer implementation.
- [`crates/denise`](../../../crates/denise): Component that owns and fills the buffer.
