---
title: "MOS 8520 Interface Adapters"
aliases: ["CIA", "MOS 8520", "CIA-A", "CIA-B"]
tags: ["amiga", "design", "cia"]
category: "Design"
subsystem: "cia"
status: "active"
created: 2026-09-06
updated: 2026-10-12
related: ["[Machine Loop.md](Machine%20Loop.md)", "[Keyboard.md](Keyboard.md)", "[Floppy.md](Floppy.md)", "[Paula.md](Paula.md)"]
tracked_paths:
  - "crates/cia"
---
# MOS 8520 Interface Adapters

The Amiga 500 has two MOS 8520 interface adapters:

- **CIA-A** handles user-facing input and output, including keyboard and game-port signals.
- **CIA-B** handles control signals for the floppy drive and external ports.

Both use the Motorola E-Clock, a clock domain distinct from the main Color Clock (CCK). The emulator represents them as two instances of the same CIA implementation.

## Time, ports, and serial transfer

Each CIA has two interval timers. A timer counts clock events down from a
loaded value; reaching the end can produce an interrupt or a pulse on an
output pin. It can then stop or reload and repeat. The timers can count
E-Clock ticks or external pulses, and one timer can count events produced by
the other. This makes them useful for periodic work, elapsed-time measurement,
and peripheral timing.

The time-of-day (TOD) circuit counts external events separately from those
interval timers. It can compare its count with an alarm value and request an
interrupt. Reading its multi-part count uses a held snapshot so a carry while
the live counter advances cannot combine parts from different instants.

The CIA serial circuit shifts one byte over a data line under a clock signal.
On the A500, CIA-A receives the separate [keyboard](Keyboard.md) controller's
serial stream; after a byte arrives, it can request an interrupt. This
synchronous link is distinct from Paula's external serial interface. Each CIA
also drives or samples the individual pins of its ports, including the
[floppy drive](Floppy.md) controls and status lines. Its pending, enabled
interrupt sources are combined into one output routed through
[Paula](Paula.md).

## Reference documentation and source

- [`cia.rs`](../../../crates/cia/src/cia.rs) implements the shared CIA behavior and E-Clock stepping.
- [Amiga Hardware Reference Manual, Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md): Describes timer modes, TOD latching, serial shifting, port pins, and interrupt output.
- [Machine Loop.md](Machine%20Loop.md) describes how the two CIA instances connect to the machine.
