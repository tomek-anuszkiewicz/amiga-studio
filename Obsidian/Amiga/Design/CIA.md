---
title: "MOS 8520 Interface Adapters"
aliases: ["CIA", "MOS 8520", "CIA-A", "CIA-B"]
tags: ["amiga", "design", "cia"]
category: "Design"
subsystem: "cia"
status: "active"
created: 2026-09-06
updated: 2026-11-12
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


These are canonical software addresses. The motherboard's
[Gary selection contract](MemoryBus.md#cia-and-boot-overlay) also routes
`$A0-$BF` aliases and dual selection. A word read combines CIA-B high/CIA-A
low; byte writes reach every selected chip on either parity, while word
writes send each chip its own lane. Full canonical identities remain in the
router, with concrete named methods on the selected chip instances.

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

## Reconstructed engine contract

HRM Appendix F's timer, SDR, port and TOD descriptions govern both instances.
Timer A selects E or positive CNT edges; Timer B selects E, CNT, Timer A
underflow, or Timer A underflow while CNT is high. Counters reload on underflow;
one-shot clears START. Force-load and high-byte write effects remain at the
existing falling-E commitment phase. PB6/PB7 override direction when PBON is
set. Toggle mode starts high and toggles on underflow; pulse mode emits one
positive E-cycle pulse. Source edge capture and engine advancement remain
separate, with no direct SDR assignment from the keyboard.

Input SDR shifts MSB first on rising CNT and publishes only after eight bits.
Output SDR has a holding byte and a distinct shift byte. Timer A underflows
alternate CNT low/high; falling CNT establishes SP and rising CNT completes
one bit. The eighth rising edge requests ICR.SDR; a queued byte continues,
otherwise CNT stays high and SP retains the last bit. Switching serial direction
cancels partial transfer state. Input releases both drivers; output initializes
CNT high and SP low, allowing the keyboard acknowledgment without a timer
transfer. This direction-transition model follows the replacement CIA serial
reset and vAmiga CRA handling, not a calibrated silicon edge measurement. Port B accesses schedule
PC low on the third following E cycle for one cycle; debug reads are neutral.
FLAG counts falling edges only. Timer outputs and pulled-up inputs determine
resolved port readback. TOD retains 24-bit binary wrapping, alarm matching and
the established capture-phase read latch and write halt/restart behavior.

## Named interfaces, fields and functional limits

Both supported PAL/NTSC OCS machines use this same pair of 8520 interfaces.
For each connected row above, lowercase the register name to find the named
reader in [`registers.rs`](../../../crates/cia/src/registers.rs), and prefix
`set_` for its queued writer: for example `tahi()` and `set_tahi(value)`.
[`cia_registers.rs`](../../../crates/memory_bus/src/cia_registers.rs) selects
the CIA instance and calls those methods. The queue stores the selector
`0..15`, not a motherboard address. Every connected write uses `OverwritePending` mode until the next falling-E capture boundary described below. A later write to the same selector replaces only its pending operation, including ICR masks and LOAD actions; captured effects are not undone. Selector B
reads `$FF` and ignores writes without a synchronization or capture event.

Debug reads reuse the ordinary readers without side effects for PRA, directions,
timers, SDR and controls. PRB uses `prb_debug` to avoid starting PC;
ICR uses `icr_debug`, and TOD uses
`todlo_debug/todmid_debug/todhi_debug`: they inspect the live time counter,
without acquiring/releasing or consulting the held TOD read snapshot. Live
TOD readers use the held value while latched; live high/low reads schedule
acquire/release for falling E. Live ICR acknowledges only after bus acceptance.
These access classes do not certify electrical pin or engine correctness.

The register-family sections in
[Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md#register-functional-description)
define the remaining fields: each bit of a port data or direction register belongs to one pin;
the timer halves form separate 16-bit reload/counter values; the three TOD
bytes form a 24-bit count/alarm; SDR transfers eight serial bits. ICR bits 0–4
identify Timer A, Timer B, alarm, serial and FLAG sources, with bit 7 reporting
the aggregate interrupt on reads or selecting mask set/clear on writes.
CRA/CRB bits 0–4 select start, timer output on PB6/PB7, toggle/pulse,
one-shot/continuous and force-load. CRA bit 5 selects the count input, bit 6
selects serial direction and bit 7 is unused. CRB bits 6–5 select E, CNT,
Timer A underflow, or underflow gated by CNT; bit 7 selects time/alarm writes.
LOAD is an action with no stored readable bit. Port-B `PC` pulses follow
accepted live reads or committed writes; their three-E-cycle scheduling is
separate from the register capture delay. There is no attached parallel endpoint.

Port reads merge the committed output latch on output-configured bits with
delivered external levels on input-configured bits. External sampling updates
`pins.port_a_input`/`port_b_input`, not PRA/PRB's output latches; changing a
direction therefore exposes the retained output byte rather than a value
overwritten by the attached device. PB6/PB7 select the timer waveform when PBON is set, overriding DDRB.
Finer electrical drive strength is not modeled.
All thirty connected addresses retain their named access/capture paths.

## Motherboard pin boundary

[`pins.rs`](../../../crates/cia/src/pins.rs) separates SP/CNT input levels,
released/driven output levels, PC, FLAG and the last delivered TOD clock level
from the serial shift and timer state.
The [machine input phase](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
asks each CIA to sample its retained `pins.incoming` before engines step.
CIA-A SP/CNT connect to keyboard KDAT/KCLK. On the physical board its PC and
FLAG connect to parallel strobe and ACK; the emulator currently has no parallel
connector model. CIA-A PB and FLAG and CIA-B external PA0..5 stay released high;
the serial connector is also unconnected, leaving CTS/DSR/CD inactive,
while PRA/PRB, direction registers and output drivers remain available.
Chip-side port, PC strobe and FLAG interrupt logic runs with these
disconnected inputs.
An attached device's ACK/status responses and full serial/parallel exchanges
remain deferred to [12.4](../../../ROADMAP.md#serial-parallel-device-support).
CIA-B FLAG receives disk INDEX, not serial carrier detect. CIA-B PA0/PA1 also
share the BUSY/POUT lines with its SP/CNT pins. The coordinator resolves committed
Port A drivers and enabled serial output drivers together; a driven low reaches
both the serial input and Port A input readback at the next input phase. Serial
input mode releases its drivers. No attached parallel device is required for
these motherboard connections. A delivered FLAG falling edge uses the existing ICR
input; a held low level does not become repeated interrupts. The exact physical
FLAG synchronizer latency is not calibrated by this CCK capture contract.

CIA-A receives the captured vertical timing source at TOD; CIA-B receives the
horizontal source. A delivered low-to-high transition counts once in the CIA,
and a held high level does not count again. The retained input level prevents a
snapshot restore from inventing another edge. Source pulse widths are provisional
CCK markers described in [Agnus](Agnus.md#motherboard-transport-boundary).
The active-low CIA IRQ output is routed from pending ICR status into Paula-owned incoming INT2 or INT6
and sampled before its next step. Drive status reaches CIA-A input latches
through capture/delivery. Independent FIRE levels are sampled directly at the
current CCK input phase, after the retained inputs arrive. Both paths remain
independent of PRA and DDRA; direct FIRE sampling preserves the captured drive
status on PA5-PA2 and does not change CIA bus synchronization or register timing.

PC/serial output generation, eight-bit reception and timer/CNT reactions
run in the CIA engine. Component tests cover these mechanisms; transport tests
can also drive outputs deterministically to isolate recipient delivery. The
[catalog's E-clock section](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md#cpu-gary-and-cia-the-synchronous-e-clock-transaction)
also distinguishes actual VPA/VMA wiring from the bus wait calculation below.

## Runtime register capture and bus synchronization

[MemoryBus](MemoryBus.md#cia-and-boot-overlay) selects the connected receiver or receivers for each transaction and calls a named reader or setter on one of two independent `Cia`
instances. CIA's public register interface has no motherboard address or chip
identity. [`registers.rs`](../../../crates/cia/src/registers.rs) owns these
accessors and private application methods. Reads expose active state; each
setter returns whether its write entered the fixed inline mutation buffer.
A rejected full-buffer write changes neither the active register nor pending
work. A successful setter never applies its payload immediately.

The current CCK model separates bus synchronization from register capture:

| Divider phase | Modeled E-clock work |
| --- | --- |
| `0` | Rising edge; E-clock timer sources count here. |
| `1` | A synchronized CPU transfer can return `Ready`; a write enters its receiver's queue. |
| `2` | Falling edge; pending register writes and TOD latch acquire/release take effect. |
| `3`, `4` | Remaining low portion of E. |

This represents the six CPU clocks low/four high duty cycle in
[Motorola Appendix B, B.1](../Reference/68000%20User's%20Manual/13%20-%20Appendix%20B%20-%20M6800%20Peripheral%20Interface.md).
The phase origin is a model convention. The CPU invokes reads in CCK1 and
writes in CCK2; `WaitState` keeps that active micro-step and its inputs stable.
On the first connected CIA attempt, `synchronize_bus_access` records its synchronization
countdown without reading or enqueuing a write. Retries report readiness only at phase
`1`. Readiness remains pending until every receiving chip is synchronized.
The motherboard then calls
`finish_bus_access` for all receivers before the named accessors; a refused
dual transfer cannot consume just one chip's synchronization. For initial phases `0..4`, waits are `6, 5, 4, 3, 7` CCK respectively,
derived from the even CPU-phase entries in
[vAmiga's `Agnus::syncWithEClock`](../../../ref_src/vAmiga/Core/Components/Agnus/Agnus.cpp).
The implementation computes the forward CCK distance to phase `1`. If that
window is fewer than `MIN_BUS_SYNC_CCK = 3` CCK away, it adds one full
`CCK_PER_ECLOCK = 5` CCK period. This expresses the same five delays without
a phase lookup table. The minimum interval is a property of the reproduced
reference-emulator timing model, not a measured hardware delay or a CPU-clock
setup-time specification.

Focused regressions verify the wait counts at all five represented CCK phases,
accepted read effects and falling-edge write capture. They do not establish
complete production CPU bus-cycle durations against Motorola Figures B-4/B-5,
nor resolve all ten individual CPU-clock positions within E. The remaining
[production CPU integration](../../../ROADMAP.md#cpu-memory-gary-verification)
own that verification; these tests do not certify the hardware 10-19-clock range.

The [replacement-project CIA](../../../ref_src/amiga_replacement_project/cia/cia8520.v)
(revision `143ad5f1b5acae093ea66186e832a8ab4fb1bc0f`) models timer counting
on rising E and register writes/output changes on falling E. Its
[TOD helper](../../../ref_src/amiga_replacement_project/cia/cia_tod.v)
explicitly acquires/releases the read latch at the falling edge. Accordingly,
a named setter at any phase targets the next falling edge; a production bus
write accepted at phase `1` commits one CCK later. This is capture alignment,
not an additional arbitrary E-clock delay assigned to every register.

Writes to ports, directions, timer latches, time/alarm, SDR, ICR masks and
controls all use that capture path. Port-B output notifications apply
`PRB | !DDRB` followed by enabled timer overrides, so an ordinary input
releases its control line high instead of driving the stored latch value. Direction changes use the same committed output path. Timer high-byte writes load stopped timers and load/start one-shot timers
regardless of START; force-load strobes apply the reload value and do not become
readable stored bits. CRA also discards its documented unused bit 7. Time writes to the middle/high bytes halt TOD until the low byte
is written. Enabling a mask for an already pending ICR source asserts the master
interrupt. Live ICR reads retain clear-on-read at the accepted read phase;
reference implementations differ on finer acknowledgment timing, which this
model does not claim to calibrate. Debug readers neither acknowledge interrupts
nor acquire/release TOD latches or advance pending work.

Under the user-approved policy of 2026-11-11, pending writes retain the last value per selector until the next capture edge. Replacement recomputes the remaining time to that same edge rather than postponing capture. Writes to different selectors remain ordered; once captured, effects are not canceled by a later write. The sixteen-slot buffer exceeds
production CPU demand: each CIA accepts at most one transfer per E period,
and a phase-1 write commits at phase 2, before another transfer can be accepted.
CPU bus acceptance therefore checks E-clock synchronization without a queue
capacity check or a queue-related wait state. Direct callers must respect the
setter's acceptance result and drain injected writes before resuming CPU bus
traffic; direct writes are not a second concurrent producer on that bus path.
Valid direct setters retain at most one pending operation per connected selector. A due buffer of the same capacity applies all retained operations at capture; an operation replaced before capture has no effect.
The shared staging helper rejects overflow without immediate application.

The serialized CIA value contains its divider phase, pending bus countdown,
chip-local register selectors `0..15`, write values/countdowns/modes, active
registers, TOD latch operation/value, unconsumed port-output changes,
serial holding/shift state, captured CNT edges, timer waveforms and PC pipeline.
[Snapshots](Application/SaveState.md) store this encoding directly. Reset cancels pending
work. Restore preserves these values and resumes effects at their original
phase rather than polling outputs during loading. Silicon edge calibration remains separate work; the register-map table's unknown delays remain unknown.

## Reference documentation and source

- [`cia.rs`](../../../crates/cia/src/cia.rs) and [`registers.rs`](../../../crates/cia/src/registers.rs): Serializable chip state, E-clock synchronization, queued named register access and capture.
- [Motorola M68000 User's Manual, Appendix B](../Reference/68000%20User's%20Manual/13%20-%20Appendix%20B%20-%20M6800%20Peripheral%20Interface.md): Synchronous peripheral transfer, VPA/VMA and E-clock duty cycle.
- [vAmiga Agnus](../../../ref_src/vAmiga/Core/Components/Agnus/Agnus.cpp): Reference-emulator synchronization countdown used by the CCK model.
- [Replacement-project CIA](../../../ref_src/amiga_replacement_project/cia/cia8520.v) and [TOD helper](../../../ref_src/amiga_replacement_project/cia/cia_tod.v): Reconstructed edge ordering for writes, timer counts and TOD read latches; not silicon-delay measurements.
- [Amiga Hardware Reference Manual, Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md): Describes timer modes, TOD latching, serial shifting, port pins, and interrupt output.
- [Amiga Hardware Reference Manual, Appendix D](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md): Separates CIA byte lanes and addresses from the custom register block.
- [Machine Loop.md](Machine%20Loop.md) describes how the two CIA instances connect to the machine.
