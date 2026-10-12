---
title: "MOS 8520 Interface Adapters"
aliases: ["CIA", "MOS 8520", "CIA-A", "CIA-B"]
tags: ["amiga", "design", "cia"]
category: "Design"
subsystem: "cia"
status: "active"
created: 2026-09-06
updated: 2026-10-12
related: ["[Machine Loop.md](Machine%20Loop.md)", "[MemoryBus.md](MemoryBus.md)", "[Keyboard.md](Keyboard.md)", "[Floppy.md](Floppy.md)", "[Paula.md](Paula.md)"]
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

## Register address map

The two 8520s have the same 16 register selectors but different byte addresses and port wiring. [Appendix D](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md) places CIA-A on the odd-byte lane at `$BFE001–$BFEF01` and CIA-B on the even-byte lane at `$BFD000–$BFDF00`. Address bits 11–8 select the internal register. These addresses are separate from the [Agnus, Denise, and Paula register block](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md). The tables below follow the [8520 register and port maps in Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md); they describe documented hardware behavior, not implementation coverage.

Each row separates what a read returns from what a write changes. `RW` means both directions are documented, not that they address the same stored value. `COR` identifies a read that clears status. `Unconnected` means Appendix F shows no internal register at selector `B`. `Scope` names the immediate circuit or connected signals, not every downstream effect. `Delay` is `Unknown` unless Appendix F gives a specific interval; no CCK delay is inferred from the E-Clock domain.

The timer addresses read the running counter and write a separate reload latch. Writing a timer high byte may load the counter; in one-shot mode it starts counting. At TOD addresses, reads always return the time counter, while writes target time when `CRB7=0` or the alarm when `CRB7=1`. A write to the TOD clock stops it until its low byte is written. Reading the TOD high byte holds a snapshot until the low byte is read. Reading `ICR` returns and clears pending interrupt status; writing `ICR` sets or clears selected mask bits according to bit 7. The `LOAD` bit in `CRA` and `CRB` is a write strobe and reads back as zero. Timer output can replace PB6 or PB7 when enabled.

### CIA-A

| Address | Read identity | Write identity | Access | Scope | Delay | Function and book notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `$BFE001` | `PRA` pin levels | `PRA` output latch | RW | Port A pins | Unknown | Bits 7–6: game-port fire; 5–2: disk status; 1: LED; 0: memory overlay. |
| `$BFE101` | `PRB` pin levels | `PRB` output latch | RW | Parallel port, `PC` | `PC`: third CIA cycle; others unknown | Eight parallel data pins; a port B access also asserts `PC` for handshaking. |
| `$BFE201` | `DDRA` direction bits | `DDRA` direction bits | RW | Port A pins | Unknown | One bit per pin; 1 selects output. |
| `$BFE301` | `DDRB` direction bits | `DDRB` direction bits | RW | Port B pins | Unknown | One bit per pin; 1 selects output. |
| `$BFE401` | `TALO` counter low | `TALO` latch low | RW | Timer A | Unknown | Low byte of Timer A. |
| `$BFE501` | `TAHI` counter high | `TAHI` latch high | RW | Timer A | Unknown | High-byte write can load or start the timer according to its mode. |
| `$BFE601` | `TBLO` counter low | `TBLO` latch low | RW | Timer B | Unknown | Low byte of Timer B. |
| `$BFE701` | `TBHI` counter high | `TBHI` latch high | RW | Timer B | Unknown | High-byte write can load or start the timer according to its mode. |
| `$BFE801` | `TODLO` time low | `TODLO` time/alarm low | RW | TOD / alarm | Unknown | Low-byte read releases a held TOD snapshot. |
| `$BFE901` | `TODMID` time middle | `TODMID` time/alarm middle | RW | TOD / alarm | Unknown | Middle byte of the 24-bit event count. |
| `$BFEA01` | `TODHI` time high | `TODHI` time/alarm high | RW | TOD / alarm | Unknown | High-byte read captures a stable TOD snapshot. |
| `$BFEB01` | — | — | Unconnected | None specified | Unknown | Appendix F labels selector `B` not used. |
| `$BFEC01` | `SDR` received data | `SDR` transmit data | RW | Keyboard serial link | Unknown | Synchronous serial data; CIA-A receives the keyboard stream. |
| `$BFED01` | `ICR` pending status | `ICR` interrupt mask | COR / write | Interrupt output `INT2` | Unknown | Read clears status and releases IRQ; bit 7 of a write selects set or clear for masked sources. |
| `$BFEE01` | `CRA` control bits | `CRA` control and `LOAD` strobe | RW | Timer A, PB6, serial mode | Unknown | Timer A mode and source; `LOAD` bit reads as zero. |
| `$BFEF01` | `CRB` control bits | `CRB` control and `LOAD` strobe | RW | Timer B, PB7, TOD | Unknown | Timer B mode; bit 7 selects TOD time or alarm writes. |

### CIA-B

| Address | Read identity | Write identity | Access | Scope | Delay | Function and book notes |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `$BFD000` | `PRA` pin levels | `PRA` output latch | RW | Port A pins | Unknown | Serial handshake and parallel control/status: `DTR`, `RTS`, `CD`, `CTS`, `DSR`, `SEL`, `POUT`, `BUSY`. |
| `$BFD100` | `PRB` pin levels | `PRB` output latch | RW | Floppy control pins | Unknown | Motor, drive selects 3–0, side, direction, and step. |
| `$BFD200` | `DDRA` direction bits | `DDRA` direction bits | RW | Port A pins | Unknown | One bit per pin; 1 selects output. |
| `$BFD300` | `DDRB` direction bits | `DDRB` direction bits | RW | Port B pins | Unknown | One bit per pin; 1 selects output. |
| `$BFD400` | `TALO` counter low | `TALO` latch low | RW | Timer A | Unknown | Low byte of Timer A. |
| `$BFD500` | `TAHI` counter high | `TAHI` latch high | RW | Timer A | Unknown | High-byte write can load or start the timer according to its mode. |
| `$BFD600` | `TBLO` counter low | `TBLO` latch low | RW | Timer B | Unknown | Low byte of Timer B. |
| `$BFD700` | `TBHI` counter high | `TBHI` latch high | RW | Timer B | Unknown | High-byte write can load or start the timer according to its mode. |
| `$BFD800` | `TODLO` time low | `TODLO` time/alarm low | RW | TOD / alarm | Unknown | Low-byte read releases a held TOD snapshot. |
| `$BFD900` | `TODMID` time middle | `TODMID` time/alarm middle | RW | TOD / alarm | Unknown | Middle byte of the 24-bit event count. |
| `$BFDA00` | `TODHI` time high | `TODHI` time/alarm high | RW | TOD / alarm | Unknown | High-byte read captures a stable TOD snapshot. |
| `$BFDB00` | — | — | Unconnected | None specified | Unknown | Appendix F labels selector `B` not used. |
| `$BFDC00` | `SDR` received data | `SDR` transmit data | RW | Serial circuit | Unknown | Register exists, but the Amiga assigns no serial peripheral to CIA-B. |
| `$BFDD00` | `ICR` pending status | `ICR` interrupt mask | COR / write | Interrupt output `INT6` | Unknown | Read clears status and releases IRQ; bit 7 of a write selects set or clear for masked sources. |
| `$BFDE00` | `CRA` control bits | `CRA` control and `LOAD` strobe | RW | Timer A, PB6, serial mode | Unknown | Timer A mode and source; `LOAD` bit reads as zero. |
| `$BFDF00` | `CRB` control bits | `CRB` control and `LOAD` strobe | RW | Timer B, PB7, TOD | Unknown | Timer B mode; bit 7 selects TOD time or alarm writes. |

## Reference documentation and source

- [`cia.rs`](../../../crates/cia/src/cia.rs) implements the shared CIA behavior and E-Clock stepping.
- [Amiga Hardware Reference Manual, Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md): Describes timer modes, TOD latching, serial shifting, port pins, and interrupt output.
- [Amiga Hardware Reference Manual, Appendix D](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md): Separates CIA byte lanes and addresses from the custom register block.
- [Machine Loop.md](Machine%20Loop.md) describes how the two CIA instances connect to the machine.
