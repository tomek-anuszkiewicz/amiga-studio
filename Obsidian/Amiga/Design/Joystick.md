---
title: "Joystick"
aliases: ["Amiga Joystick"]
tags: ["amiga", "design", "joystick", "input", "game_ports"]
category: "Design"
subsystem: "joystick"
status: "active"
created: 2026-09-04
updated: 2026-10-12
related: ["[Game Ports.md](Game%20Ports.md)"]
tracked_paths:
  - "crates/joystick"
  - "crates/game_ports"
---

# Joystick

The Amiga joystick connects to a [game port](Game%20Ports.md). The [`joystick` crate](../../../crates/joystick/src/joystick.rs) models the device, and the [`game_ports` crate](../../../crates/game_ports/src/game_ports.rs) connects it to a port.

A digital joystick closes directional switches rather than reporting a
continuous position. The Amiga senses combinations of those lines to
distinguish up, down, left, right, and diagonals. Its fire button uses a
separate input line. [Game Ports](Game%20Ports.md) explains how those device
signals reach the appropriate chips in the machine.

## References

- [Game Ports](Game%20Ports.md): Game port ownership and device connection.
- [Joystick source](../../../crates/joystick/src/joystick.rs): Joystick device model.
- [Game ports source](../../../crates/game_ports/src/game_ports.rs): Joystick connection to a game port.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Explains digital joystick direction and button signals.
