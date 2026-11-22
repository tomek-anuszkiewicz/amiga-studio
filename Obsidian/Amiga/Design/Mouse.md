---
title: "Mouse"
aliases: ["Amiga 500 Mouse"]
tags: ["amiga", "design", "mouse", "input", "game_ports"]
category: "Design"
subsystem: "game_ports"
status: "active"
created: 2026-09-04
updated: 2026-11-16
related: ["[Game Ports.md](Game%20Ports.md)"]
tracked_paths:
  - "crates/game_ports/src/mouse.rs"
  - "crates/game_ports"
---

# Mouse

The Amiga 500 mouse connects to a game port. As the mouse moves, it generates
two phased signals for each axis. The order of their transitions tells the
Amiga which way the mouse moved. The Amiga counts the pulses in small
position counters that wrap when they reach their limits. Software compares
consecutive counter readings to calculate relative movement. The pulses
report movement alone and carry no absolute screen coordinates.
[Game port](Game%20Ports.md)

The mouse model keeps two quadrature phases and a separate signed amount of
pending movement for each axis. The model accumulates host input without
allocating memory. The pending movement on each axis saturates at the
signed-delta limit. Each CCK, the model converts one step of pending movement
on each moving axis into a Gray-code transition. This continues until it has
consumed the pending movement. Denise owns the wrapping counters.
[Denise](Denise.md)

The button switches use separate port lines. The game-port model routes the
movement and button signals to the chips that sense them.
[Game-port model](Game%20Ports.md)


## References

- Game port ownership and device connection. [Game Ports](Game%20Ports.md)
- Implementation of the mouse device model. [Mouse source](../../../crates/game_ports/src/mouse.rs)
- Implementation of the mouse connection to a game port. [Game ports source](../../../crates/game_ports/src/game_ports.rs)
- Hardware description of mouse quadrature, wrapping counters, and buttons. [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md)
