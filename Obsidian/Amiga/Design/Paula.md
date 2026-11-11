---
title: "Paula"
aliases: ["MOS 8364"]
tags: ["amiga", "design", "paula", "audio", "interrupts"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-06
updated: 2026-11-11
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

## Register propagation

Concrete queued setters in [registers.rs](../../../crates/paula/src/registers.rs) delay interrupt, DMA, audio, disk, serial and port register effects until private application. STRHOR uses a positive **1 CCK provisional** delay and retains its no-op receiver. Audio pointer register ownership stays exclusively in [Agnus](Agnus.md); Paula's audio children retain their existing engine state. CPU INTREQ changes and interrupt-input requests occupy distinct queue destinations. Input requests merge their pending SET bits without a second register-write delay; CPU writes cannot replace that input event. The 32-entry queue exceeds two CPU/Copper writes per CCK over two CCK plus one merged interrupt-input event (five entries). Register setters reserve one entry for the single merged interrupt-input event. Input bits merge without resetting the earliest deadline; distinct CPU events retain issue order, so a later clear can acknowledge an earlier input SET and a later input SET can follow a clear. Full register capacity rejects writes without early application, and replay applies and returns all matured entries. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) defines modes, provisional timing and acceptance. Autonomous audio transitions and disk counters retain their separate engine interfaces.

## References

- [General Architecture](General%20Architecture.md): Places Paula in the machine and shows its component relationships.
- [`crates/paula/src/paula.rs`](../../../crates/paula/src/paula.rs): Living implementation of Paula's state and coordination.
