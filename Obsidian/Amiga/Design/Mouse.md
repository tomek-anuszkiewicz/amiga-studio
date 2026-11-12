---
title: "Mouse"
aliases: ["Amiga 500 Mouse"]
tags: ["amiga", "design", "mouse", "input", "game_ports"]
category: "Design"
subsystem: "game_ports"
status: "active"
created: 2026-09-04
updated: 2026-11-12
related: ["[Game Ports.md](Game%20Ports.md)"]
tracked_paths:
  - "crates/game_ports/src/mouse.rs"
  - "crates/game_ports"
---

# Mouse

The Amiga 500 mouse connects to a [game port](Game%20Ports.md).

Movement produces two phased signals for each axis. Their order indicates
direction, and the Amiga accumulates the pulses in small wrapping position
counters. Software determines movement by comparing successive counter
readings, rather than receiving absolute screen coordinates from the mouse.
The mouse retains two quadrature phases and pending signed deltas. It emits
one Gray-code transition per axis per CCK until the deltas are consumed;
[Denise](Denise.md) owns the wrapping counters. Host input accumulation
saturates at the signed-delta limit without allocating. Button switches travel
on separate port lines. The [game-port model](Game%20Ports.md)
connects these device signals to the chips that sense them.

This document is a detailed chapter of [Game Ports](Game%20Ports.md). The `game_ports::mouse` module and its component tests belong to the `game_ports` crate.

## References

- [Game Ports](Game%20Ports.md): Game port ownership and device connection.
- [Mouse source](../../../crates/game_ports/src/mouse.rs): Mouse device model.
- [Game ports source](../../../crates/game_ports/src/game_ports.rs): Mouse connection to a game port.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Explains mouse quadrature, wrapping counters, and buttons.
