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

The interrupt controller in [Paula](Paula.md) collects requests from the
Amiga's chips and peripherals. It uses the enable mask to determine which pending requests can reach the
CPU, then selects the highest priority level among them. On the Amiga, Paula
drives the interrupt priority lines of the [Motorola 68000](CPU%20Motorola%20M68000.md)
directly. In the emulator, the [machine loop](Machine%20Loop.md) delivers
the selected level to the CPU.

The internal [`paula::interrupts` module](../../../crates/paula/src/interrupts.rs)
implements the controller. This chapter belongs to the [Paula specification](Paula.md).
The controller's component tests live under `crates/paula/tests/`.

An event such as vertical blank, a completed Blitter or disk transfer, an
audio channel event, or a CIA interrupt sets a pending request. An enable
mask determines which pending sources can reach the CPU. Paula selects a
priority level from the enabled requests, and the [Machine Loop](Machine%20Loop.md)
carries it to the CPU. If the level exceeds the CPU's current mask, the CPU may enter the
corresponding interrupt exception. The request stays pending until software
acknowledges it.

CIA-A IRQ, Agnus INT3 and CIA-B IRQ reach Paula as captured active-low
INT2/INT3/INT6 input levels. Paula applies due CPU register writes before
sampling the inputs. An asserted level sets its request latch. The latch
retains the request when the input releases. If an input assertion coincides with a CPU acknowledgment, this ordering
lets Paula set the latch again when it samples the input. Silicon
measurements of the race are unavailable here. The motherboard drives
IPL in the same phase before the CPU runs, with no additional CCK delay.
[Machine Loop](Machine%20Loop.md#connections-advanced-within-the-step) defines
the phases, and [Paula](Paula.md#register-propagation) defines the order of
register writes and input sampling.

## Register and priority contract

The approved [Paula sync model](Paula.md#rga-synchronization-and-vertb) requests
VERTB on entry into STREQU, once per field in the normal Agnus sequence.
Repeated STREQU and STRLONG do not request it again. A captured sync event sets VERTB after Paula applies due INTREQ writes.
CPU sync selections act immediately.
The request remains pending until software clears it, independently of the
equalization latch. Agnus no longer supplies a decoded VBlank event.

HRM 7 Interrupts and Figure 7-4 define fourteen request sources. The master bit in INTENA controls delivery. INTREQ retains pending
requests. SET/CLR affects only bits written as one, and a read leaves
pending requests intact. The highest
enabled source group selects IPL. TBE/DSKBLK/SOFT use
level 1, PORTS level 2, COPER/VERTB/BLIT level 3, audio level 4, RBF/DSKSYN
level 5 and EXTER level 6. There is no Paula level-7 source. In the common bit
assignment in Appendix A, the register latches retain bits 0-14 and only bits
0-13 take part in choosing the source. CPU exception handling leaves those
latches intact. Under the existing CCK policy, Paula applies local audio/UART
request pulses and delivered input levels after due software writes.

## References

- [Paula](Paula.md): Chip ownership and interrupt integration.
- [Machine Loop](Machine%20Loop.md): Delivery of Paula's interrupt level to the CPU.
- [Amiga Hardware Reference Manual, Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): Hardware interrupt control and priority levels.
