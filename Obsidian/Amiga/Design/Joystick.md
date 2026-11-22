---
title: "Joystick"
aliases: ["Amiga Joystick"]
tags: ["amiga", "design", "joystick", "input", "game_ports"]
category: "Design"
subsystem: "game_ports"
status: "active"
created: 2026-09-04
updated: 2026-11-16
related: ["[Game Ports.md](Game%20Ports.md)"]
tracked_paths:
  - "crates/game_ports/src/joystick.rs"
  - "crates/game_ports"
---

# Joystick

An Amiga joystick plugs into a [game port](Game%20Ports.md). In the emulator,
the [`game_ports::joystick` module](../../../crates/game_ports/src/joystick.rs)
models the device, and the [`game_ports` crate](../../../crates/game_ports/src/game_ports.rs)
connects it to a port.

A digital joystick reports which directional switches are closed over four
active-low contacts. A separate line carries the fire button state. The
Amiga can read several directional contacts at once, including diagonals. Denise supplies the JOY register view. Bits 9/1 expose LEFT/RIGHT. Taking the XOR of the two low bits of each
axis gives FORWARD/BACK, including when a diagonal is held. The device itself supplies only the switch signals.
[Game Ports](Game%20Ports.md) explains how they reach the chips.


## References

- [Game Ports](Game%20Ports.md): Game port ownership and device connection.
- [Joystick source](../../../crates/game_ports/src/joystick.rs): Joystick device model.
- [Game ports source](../../../crates/game_ports/src/game_ports.rs): Joystick connection to a game port.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Explains digital joystick direction and button signals.
