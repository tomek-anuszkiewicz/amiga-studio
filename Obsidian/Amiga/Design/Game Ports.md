---
title: "Game Ports"
aliases: ["Gameports", "Controller Ports", "Port 1", "Port 2"]
tags: ["amiga", "design", "game_ports", "input", "denise", "paula", "cia"]
category: "Design"
subsystem: "game_ports"
status: "active"
created: 2026-09-13
updated: 2026-10-12
related: ["[Mouse.md](Mouse.md)", "[Joystick.md](Joystick.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Machine Loop.md](Machine%20Loop.md)", "[General Architecture.md](General%20Architecture.md)"]
---

# Game Ports

The two game ports represent the Amiga's controller sockets. Each port can host an emulated [mouse](Mouse.md), [joystick](Joystick.md), or no device. The frontend translates input from host hardware into the state of those emulated devices. A host mouse can supply mouse movement and buttons; host keyboard controls can be mapped to joystick directions and buttons. The Amiga keyboard itself is a separate peripheral.

Game port signals do not belong to a single chip. The corresponding registers and input lines are distributed across [Denise](Denise.md), [Paula](Paula.md), and [CIA-A](CIA.md). The game ports model the connected devices and their signals; those chips retain their own register state and behavior.

[MachineLoop](Machine%20Loop.md) owns the game ports alongside the chips. It samples the port signals and passes them to the appropriate chip, so the devices and chips communicate through the machine rather than through direct references to one another.

## References

- [Game ports source](../../../crates/game_ports/src/game_ports.rs): Device slots and signal sampling.
- [Machine loop source](../../../crates/machine_loop/src/machine_loop.rs): Ownership and routing to Denise, Paula, and CIA-A.
- [General Architecture](General%20Architecture.md): Placement of game ports in the machine.
