---
title: "Amiga 500 Interrupt Controller"
aliases: ["Interrupts", "Interrupt Controller", "INTENA", "INTREQ", "IPL", "Paula Interrupts"]
tags: ["amiga", "design", "interrupts", "paula", "cpu"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-19
updated: 2026-11-12
related: ["[Paula.md](Paula.md)", "[CPU Motorola M68000.md](CPU%20Motorola%20M68000.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/paula/src/interrupts.rs"
  - "crates/paula/tests/test_interrupts_priority.rs"
---
# Amiga 500 Interrupt Controller

The interrupt controller is part of [Paula](Paula.md). It collects interrupt requests from the Amiga's chips and peripherals, applies the enable mask, and selects the highest pending priority level. Paula drives the [Motorola 68000](CPU%20Motorola%20M68000.md) interrupt priority lines directly; the emulator's [machine loop](Machine%20Loop.md) delivers the selected level to the CPU.

The internal [`paula::interrupts` module](../../../crates/paula/src/interrupts.rs) implements the controller. This document is the interrupt chapter of the [Paula specification](Paula.md); its component tests live under `crates/paula/tests/`.

An event such as vertical blank, a completed Blitter or disk transfer, an
audio channel event, or a CIA interrupt sets a pending request. An enable
mask determines which pending sources can reach the CPU. Paula resolves the
enabled requests to a priority level; the CPU may then enter the matching
interrupt exception when that level exceeds its current mask. The original
request remains pending until software acknowledges it, so delivering a CPU
interrupt does not erase its cause. [Machine Loop](Machine%20Loop.md) carries
the resulting level to the CPU.

CIA-A IRQ, Agnus INT3 and CIA-B IRQ reach Paula as captured active-low
INT2/INT3/INT6 input levels. Paula samples them after due CPU register writes;
an asserted level sets its request latch, while release alone does not clear it.
This explicit model phase makes assertion win a coincident CPU acknowledgment;
it is not a measured silicon race rule. The motherboard drives the resulting
IPL level in the same pre-CPU phase, without another blanket CCK delay.
[Machine Loop](Machine%20Loop.md#connections-advanced-within-the-step) owns those phases,
and [Paula](Paula.md#register-propagation) owns register/input ordering.

## Register and priority contract

The approved [Paula sync model](Paula.md#rga-synchronization-and-vertb) requests
VERTB on entry into STREQU, once per field in the normal Agnus sequence.
Repeated STREQU and STRLONG do not request it again. A captured sync event
sets VERTB after due INTREQ writes; CPU sync selections act immediately.
The request remains pending until software clears it, independently of the
equalization latch. Agnus no longer supplies a decoded VBlank event.

HRM 7 Interrupts and Figure 7-4 define fourteen request sources. INTENA's master
bit gates delivery without erasing INTREQ. SET/CLR updates only written one
bits; reads acknowledge nothing. The highest enabled source group selects IPL:
TBE/DSKBLK/SOFT level 1, PORTS level 2, COPER/VERTB/BLIT level 3, audio level 4,
RBF/DSKSYN level 5 and EXTER level 6. There is no Paula level-7 source. The
register latches retain bits 0-14 under the Appendix A common bit assignment;
only bits 0-13 participate in source arbitration. CPU exception handling does
not mutate these latches. Local audio/UART request pulses, like delivered input
levels, are applied after due software writes under the existing CCK policy.

## References

- [Paula](Paula.md): Chip ownership and interrupt integration.
- [Machine Loop](Machine%20Loop.md): Delivery of Paula's interrupt level to the CPU.
- [Amiga Hardware Reference Manual, Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): Hardware interrupt control and priority levels.
