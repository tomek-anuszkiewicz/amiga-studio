---
title: "Amiga 500 Interrupt Controller"
aliases: ["Interrupts", "Interrupt Controller", "INTENA", "INTREQ", "IPL", "Paula Interrupts"]
tags: ["amiga", "design", "interrupts", "paula", "cpu"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-19
updated: 2026-10-12
related: ["[Paula.md](Paula.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/interrupts"
---
# Amiga 500 Interrupt Controller

The interrupt controller is part of [Paula](Paula.md). It collects interrupt requests from the Amiga's chips and peripherals, applies the enable mask, and selects the highest pending priority level. Paula drives the [Motorola 68000](CPU%20Motorola%20M68000.md) interrupt priority lines directly; the emulator's [machine loop](Machine%20Loop.md) delivers the selected level to the CPU.

The controller is implemented as a separate [`interrupts` crate](../../../crates/interrupts), which is a dependency and component of the [`paula` crate](../../../crates/paula). Paula owns and re-exports the controller.

An event such as vertical blank, a completed Blitter or disk transfer, an
audio channel event, or a CIA interrupt sets a pending request. An enable
mask determines which pending sources can reach the CPU. Paula resolves the
enabled requests to a priority level; the CPU may then enter the matching
interrupt exception when that level exceeds its current mask. The original
request remains pending until software acknowledges it, so delivering a CPU
interrupt does not erase its cause. [Machine Loop](Machine%20Loop.md) carries
the resulting level to the CPU.

## References

- [Paula](Paula.md): Chip ownership and interrupt integration.
- [Machine Loop](Machine%20Loop.md): Delivery of Paula's interrupt level to the CPU.
- [Amiga Hardware Reference Manual, Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): Hardware interrupt control and priority levels.
