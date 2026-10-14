---
title: "Frame Buffer"
aliases: ["FrameBuilder"]
tags: ["amiga", "design", "frame_builder", "video", "denise"]
category: "Design"
subsystem: "frame_builder"
status: "active"
created: 2026-09-19
updated: 2026-10-12
related: ["[Denise.md](Denise.md)", "[Agnus.md](Agnus.md)", "[GUI.md](Application/GUI.md)"]
tracked_paths:
  - "crates/frame_builder"
---
# Frame Buffer

The frame buffer holds the pixels of a rendered video frame. In the emulator, `FrameBuilder` is a component of [Denise](Denise.md). [Agnus](Agnus.md) supplies display data and beam timing; Denise uses them to build the image in the buffer. The buffer is intended to pass the rendered frame to the [GUI](Application/GUI.md) for display on screen.

## References

- [`crates/frame_builder`](../../../crates/frame_builder): Buffer implementation.
- [`crates/denise`](../../../crates/denise): Component that owns and fills the buffer.