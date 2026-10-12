---
title: "Paula"
aliases: ["MOS 8364"]
tags: ["amiga", "design", "paula", "audio", "interrupts"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-06
updated: 2026-10-12
related: ["[Audio.md](Audio.md)", "[Interrupts.md](Interrupts.md)", "[Floppy.md](Floppy.md)", "[Game Ports.md](Game%20Ports.md)", "[Agnus.md](Agnus.md)"]
tracked_paths:
  - "crates/paula"
---
# Paula

Paula handles four-channel sound, system interrupts, serial communication, disk data and control, and the proportional inputs of the game ports. It receives audio and disk data through the machine's custom-chip bus; [Agnus](Agnus.md) controls the corresponding DMA transfers.

## Cargo crates

| Crate | Role |
| --- | --- |
| [`paula`](../../../crates/paula) | Owns the Paula state, routes register access, and coordinates its components. |
| [`audio`](../../../crates/audio) | Runs the four audio channels and produces sound samples. See [Audio](Audio.md). |
| [`interrupts`](../../../crates/interrupts) | Holds interrupt requests and enables, and resolves the CPU interrupt level. See [Interrupts](Interrupts.md). |

`paula` contains and re-exports `audio` and `interrupts`. Its serial-port code is an internal Rust module, [`serial.rs`](../../../crates/paula/src/serial.rs), rather than a separate Cargo crate.

## Other responsibilities in `paula`

- It holds serial data and control state. The serial transmit and receive clock is currently a scaffold, so this is not yet a complete working UART.
- It holds disk data, synchronization and status registers, controls the disk DMA arming sequence, and reports transfer completion. [Floppy](Floppy.md) describes the drive and disk-stream integration.
- It exposes the sampled proportional input and button state supplied through [Game Ports](Game%20Ports.md).
- It coordinates delayed register changes, DMA enable signals, and interrupt requests between these functions and the rest of the machine.

## References

- [General Architecture](General%20Architecture.md): Places Paula in the machine and shows its component relationships.
- [`crates/paula/src/paula.rs`](../../../crates/paula/src/paula.rs): Living implementation of Paula's state and coordination.
