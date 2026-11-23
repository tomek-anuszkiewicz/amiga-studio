---
title: "MOS 8520 Interface Adapters"
aliases: ["CIA", "MOS 8520", "CIA-A", "CIA-B"]
tags: ["amiga", "design", "cia"]
category: "Design"
subsystem: "cia"
status: "active"
created: 2026-09-06
updated: 2026-11-23
related: ["[Machine Loop.md](Machine%20Loop.md)", "[MemoryBus.md](MemoryBus.md)", "[Keyboard.md](Keyboard.md)", "[Floppy.md](Floppy.md)", "[Paula.md](Paula.md)"]
tracked_paths:
  - "crates/cia"
---
# MOS 8520 Interface Adapters

The Amiga 500 uses two MOS 8520 interface adapters.

- **CIA-A** handles user-facing input and output, including keyboard and game-port signals.
- **CIA-B** handles control signals for the floppy drive and external ports.

Both chips run on the Motorola E-Clock, a clock domain separate from the main
Color Clock (CCK). The emulator uses two instances of the same CIA implementation.

## Time, ports, and serial transfer

Each CIA has two interval timers. A timer counts clock events down from a
loaded value. When it reaches the end of the count, it can produce an interrupt
or a pulse on an output pin. It can then stop or reload and repeat. The timers
can count E-Clock ticks or external pulses, and one timer can count events from
the other. These options support periodic work, elapsed-time measurement and
peripheral timing.

The time-of-day (TOD) circuit counts external events separately from the
interval timers. It can compare that count with an alarm value and request an
interrupt. Software can read the count one byte at a time from a held copy.
All the bytes then represent the same instant, even if the live counter
carries into another byte during the read.

The CIA serial circuit uses a clock signal to shift one byte at a time over
a data line.
On the A500, CIA-A receives the serial stream from the separate
[keyboard](Keyboard.md) controller and can request an interrupt after receiving
a byte. This synchronous link operates separately from Paula's external serial
interface. Each CIA also drives or samples its individual port pins. The
[floppy drive](Floppy.md) controls and status lines use these pins. Each CIA
combines its pending, enabled interrupt sources into one output, which the
machine routes through [Paula](Paula.md).

## Register address map

Both 8520s use the same 16 register selectors on different byte addresses and
with different port wiring. Address bits 11–8 select the internal register.
CIA-A uses the odd-byte lane at `$BFE001–$BFEF01` and CIA-B uses the even-byte
lane at `$BFD000–$BFDF00`. [Appendix D](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md)
These addresses are separate from the [Agnus, Denise, and Paula register block](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md).
The tables follow the documented register map. The TOD high-byte entries
include the alarm-mode rule described below. [8520 register and port maps in Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md)

The tables give canonical software addresses. The motherboard also routes
`$A0-$BF` aliases and can select both chips at once. A word read from both
chips takes its high byte from CIA-B and its low byte from CIA-A. A byte
write reaches every selected chip regardless of address parity. A word write
sends each selected chip the byte from its own lane. The router retains the
full canonical register identities and calls named methods on the selected
chip instances. [Gary selection contract](MemoryBus.md#cia-and-boot-overlay)

Each row shows what a read returns and what a write changes. `RW` means both
directions are documented. A read and a write at one address can still use
different stored values. `COR` identifies a read that clears status.
`Unconnected` means Appendix F shows no internal register at selector `B`.
`Scope` covers only the immediate circuit or connected signals. `Delay` stays
`Unknown` unless Appendix F gives a specific interval. Knowing that a circuit
uses the E-Clock gives no CCK delay by itself.

Reading a timer address returns the running counter. Writing it updates a
separate reload latch. A high-byte write can load the counter and starts it
in one-shot mode. At TOD addresses, reads return the live or held time.
Writes target time when `CRB7=0` or the alarm when `CRB7=1`. A write to the TOD
clock stops it until a write to the low byte. Reading the high byte can hold a
new snapshot only in time mode. A low-byte read releases the snapshot in
either mode. Reading `ICR` returns and clears
pending interrupt status. Writing `ICR` sets or clears selected mask bits
according to bit 7. The `LOAD` bit in `CRA` and `CRB` acts as a write strobe and
reads back as zero. When enabled, timer output can replace PB6 or PB7.

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
| `$BFEA01` | `TODHI` time high | `TODHI` time/alarm high | RW | TOD / alarm | Unknown | High-byte read captures a new snapshot only with `CRB7=0`. |
| `$BFEB01` | — | — | Unconnected | None specified | Unknown | Appendix F labels selector `B` not used. |
| `$BFEC01` | `SDR` received data | `SDR` transmit data | RW | Keyboard serial link | Unknown | Synchronous serial data; CIA-A receives the keyboard stream. |
| `$BFED01` | `ICR` pending status | `ICR` interrupt mask | COR / write | Interrupt output `INT2` | Unknown | Read clears status and releases IRQ; bit 7 of a write selects set or clear for masked sources. |
| `$BFEE01` | `CRA` control bits | `CRA` control and `LOAD` strobe | RW | Timer A, PB6, serial mode | Unknown | Timer A mode and source; `LOAD` bit reads as zero. |
| `$BFEF01` | `CRB` control bits | `CRB` control and `LOAD` strobe | RW | Timer B, PB7, TOD | Unknown | Timer B mode. Bit 7 selects time or alarm writes and permits a new TOD snapshot only when clear. |

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
| `$BFDA00` | `TODHI` time high | `TODHI` time/alarm high | RW | TOD / alarm | Unknown | High-byte read captures a new snapshot only with `CRB7=0`. |
| `$BFDB00` | — | — | Unconnected | None specified | Unknown | Appendix F labels selector `B` not used. |
| `$BFDC00` | `SDR` received data | `SDR` transmit data | RW | Serial circuit | Unknown | Register exists, but the Amiga assigns no serial peripheral to CIA-B. |
| `$BFDD00` | `ICR` pending status | `ICR` interrupt mask | COR / write | Interrupt output `INT6` | Unknown | Read clears status and releases IRQ; bit 7 of a write selects set or clear for masked sources. |
| `$BFDE00` | `CRA` control bits | `CRA` control and `LOAD` strobe | RW | Timer A, PB6, serial mode | Unknown | Timer A mode and source; `LOAD` bit reads as zero. |
| `$BFDF00` | `CRB` control bits | `CRB` control and `LOAD` strobe | RW | Timer B, PB7, TOD | Unknown | Timer B mode. Bit 7 selects time or alarm writes and permits a new TOD snapshot only when clear. |

## Reconstructed engine contract

Both instances follow the timer, SDR, port and TOD descriptions in HRM
Appendix F, with an exception for TOD latch acquisition. An accepted TODHI
read can acquire a new latch only when committed CRB bit 7 is clear.
[Evidence for the TOD latch rule](#reference-documentation-and-source)

Timer A counts either E or positive CNT edges. Timer B counts E,
CNT, Timer A underflows, or Timer A underflows while CNT is high. A counter
reloads on underflow. In one-shot mode, the underflow also clears START.
Force-load and high-byte writes take effect at the existing falling edge of E.
When PBON is set, the timer drives PB6/PB7 regardless of the port direction.
In toggle mode the output starts high and changes level on each underflow.
Pulse mode emits one positive pulse lasting an E cycle. The model captures
source edges separately from the engine step. The keyboard supplies serial
input edges for the CIA to process without assigning SDR directly.

In input mode, the serial circuit shifts MSB first on rising CNT and makes
the byte available in SDR only after eight bits. In output mode, SDR keeps
a holding byte separate from its shift byte. Timer A underflows alternate
CNT low and high. Falling CNT sets the SP level and rising CNT completes one
bit. The eighth rising edge requests ICR.SDR. If a byte is queued, the
transfer continues. Otherwise CNT stays high and SP retains the last bit.

Switching serial direction cancels a partial transfer. Input mode releases
both drivers. Output mode initializes CNT high and SP low, which allows the
keyboard acknowledgment without a timer transfer. The direction-transition
model follows the replacement CIA serial reset and vAmiga CRA handling.
Silicon edge timing remains uncalibrated.

A Port B access schedules a one-cycle low pulse on PC for the third following
E cycle. Debug reads leave PC unchanged. FLAG counts falling edges only.
Timer outputs and pulled-up inputs determine the values read back from the
ports. TOD wraps as a 24-bit binary count and compares that count with its
alarm. The model keeps the established capture phase for the read latch and
for writes that halt and restart the count.

## Named interfaces, fields and functional limits

Both supported PAL/NTSC OCS machines use the same pair of 8520 interfaces.
For each connected register, its lowercase name gives the reader and the
`set_` prefix gives the queued writer. For example, the Timer A high-byte
methods are `tahi()` and `set_tahi(value)`. [`registers.rs`](../../../crates/cia/src/registers.rs)
The motherboard router selects the CIA instance and calls its named methods.
[`cia_registers.rs`](../../../crates/memory_bus/src/cia_registers.rs)

The queue identifies each register by its chip-local selector `0..15`. A
write to a connected register uses `OverwritePending` until the next falling-E
capture boundary. A later write to the same selector replaces only the pending
operation. This also applies to ICR masks and LOAD actions. Effects already
captured stay in force. Selector B reads `$FF` and ignores writes. Access to
that selector causes no synchronization or capture event.

Debug reads reuse the ordinary side-effect-free readers for PRA, directions,
timers, SDR and controls. They use `prb_debug` to inspect PRB without starting
PC, and `icr_debug` for ICR. The TOD debug readers
`todlo_debug/todmid_debug/todhi_debug` return the live time counter and leave
the held snapshot unread and unchanged. When a snapshot is held, live TOD
reads return its bytes in either mode. An accepted low-byte read returns
the visible low byte and schedules release. An accepted high-byte read
schedules capture of the visible count only when committed CRB bit 7 is
clear. Pending capture and release take effect at falling E.
A live ICR read acknowledges the interrupt only after the bus accepts the
access. These access rules provide
no certification of electrical pin or engine correctness.

The register-family sections in
[Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md#register-functional-description)
define the remaining fields. Each bit of a port data or direction register
belongs to one pin. The timer halves form separate 16-bit reload and counter
values. The three TOD bytes form a 24-bit count or alarm. SDR transfers eight
serial bits. ICR bits 0–4
identify Timer A, Timer B, alarm, serial and FLAG sources, with bit 7 reporting
the aggregate interrupt on reads or selecting mask set/clear on writes.
CRA/CRB bits 0–4 select start, timer output on PB6/PB7, toggle/pulse,
one-shot/continuous and force-load. CRA bit 5 selects the count input, bit 6
selects serial direction and bit 7 is unused. CRB bits 6–5 select E, CNT,
Timer A underflow, or underflow gated by CNT. CRB bit 7 selects time or alarm
writes and enables a new TOD read latch only in time mode. LOAD triggers an
action and has no stored readable bit. Accepted
live reads and committed writes to Port B schedule `PC` pulses. The
three-E-cycle pulse scheduling is separate from the register capture delay.
There is no attached parallel endpoint.

Port reads take each output bit from its committed output latch and each
input bit from the delivered external level. External sampling updates
`pins.port_a_input`/`port_b_input` and leaves the PRA/PRB output latches intact.
Changing a pin from input to output therefore exposes the retained output byte.
When PBON is set, PB6/PB7 select the timer waveform and override DDRB. The
model does not resolve finer electrical drive strength. All thirty connected
addresses retain their named access and capture paths.

## Motherboard pin boundary

The CIA keeps SP/CNT input levels, released or driven output levels, PC, FLAG
and the last delivered TOD clock level separate from serial shift and timer
state. [`pins.rs`](../../../crates/cia/src/pins.rs)
During the [machine input phase](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words),
each CIA samples its retained `pins.incoming` before the engines step.

CIA-A SP/CNT connect to keyboard KDAT/KCLK. On the physical board, CIA-A PC
and FLAG connect to the parallel strobe and ACK. The emulator currently has no
parallel connector model. CIA-A PB and FLAG and CIA-B external PA0..5 stay
released high. The serial connector is also unconnected, leaving CTS/DSR/CD
inactive. PRA/PRB, the direction registers and output drivers remain available,
and chip-side port, PC strobe and FLAG interrupt logic still runs. An attached
device's ACK/status responses and full serial/parallel exchanges remain
deferred. [12.4](../../../ROADMAP.md#serial-parallel-device-support)

Disk INDEX drives CIA-B FLAG. Serial carrier detect is a separate signal.
CIA-B PA0/PA1 share the BUSY/POUT lines with its SP/CNT pins. To determine the
level of each line, the coordinator checks the committed Port A drivers and
the enabled serial output drivers. If a driver pulls a line low, both the serial
input and Port A input readback receive that level at the next input phase.
Serial input mode releases its drivers. These motherboard connections work
without a parallel device attached. A delivered falling edge on FLAG triggers
the existing ICR input. A held low level yields no repeated interrupts.
The CCK capture contract leaves the exact physical FLAG
synchronizer latency uncalibrated.

CIA-A TOD receives the captured vertical timing source. CIA-B TOD receives
the horizontal source. Each delivered low-to-high transition counts once,
and a held high level adds no count. The CIA retains the input level so a
snapshot restore cannot produce an extra edge. The source pulse widths are
provisional CCK markers. [Agnus](Agnus.md#motherboard-transport-boundary)

Pending ICR status drives the active-low CIA IRQ output. The machine captures
that level in Paula's own incoming INT2 or INT6 state. Paula samples the stored
level before its next step. The machine captures drive status and delivers it
to CIA-A's input latches. It samples the independent FIRE levels directly and
delivers them to CIA-A in the current CCK input phase, after the retained
inputs arrive. Both paths operate independently of PRA and DDRA. Sampling FIRE
directly preserves captured drive status on PA5-PA2 and leaves CIA bus
synchronization and register timing untouched.

The CIA engine generates PC and serial outputs, receives eight-bit transfers
and reacts to timers and CNT. Component tests cover these mechanisms.
Transport tests can also drive outputs deterministically to isolate delivery
to the recipient. The
[catalog's E-clock section](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md#cpu-gary-and-cia-the-synchronous-e-clock-transaction)
also distinguishes actual VPA/VMA wiring from the bus wait calculation below.

## Runtime register capture and bus synchronization

[MemoryBus](MemoryBus.md#cia-and-boot-overlay) selects the connected receiver
or receivers for each transaction and calls a named reader or setter on each
selected `Cia` instance. There are two independent instances. Their public
register interface operates without a motherboard address or chip identity.
The named accessors and their private methods for applying changes are in
[`registers.rs`](../../../crates/cia/src/registers.rs).

Reads return active state. A setter reports whether it queued its write for
later application in the fixed inline mutation buffer. If a new write needs
a slot and all slots are occupied, the owner stops before changing the active
register or pending work. At capacity, the buffer can still replace a pending
write.

The current CCK model uses different E-clock phases for bus synchronization
and register capture.

| Divider phase | Modeled E-clock work |
| --- | --- |
| `0` | Rising edge; E-clock timer sources count here. |
| `1` | A synchronized CPU transfer can return `Ready`; a write enters its receiver's queue. |
| `2` | Falling edge; pending register writes and TOD latch acquire/release take effect. |
| `3`, `4` | Remaining low portion of E. |

This represents the duty cycle of six CPU clocks low and four high.
[Motorola Appendix B, B.1](../Reference/68000%20User's%20Manual/13%20-%20Appendix%20B%20-%20M6800%20Peripheral%20Interface.md)
The starting phase is a model convention. The CPU invokes reads in CCK1 and
writes in CCK2. If the bus returns `WaitState`, the CPU holds the active
micro-step and its inputs stable.

On the first attempt to access a connected CIA, `synchronize_bus_access`
records the synchronization countdown. The attempt ends before the chip reads
a value or queues a write. Retries can report readiness only at phase `1`.
Each chip retains its completed countdown while the motherboard waits for all
receiving chips to be ready. The motherboard then calls `finish_bus_access` for
each receiver before calling the named accessors. A refused transfer to both
CIAs cannot consume just one chip's synchronization.

For initial phases `0..4`, the waits are `6, 5, 4, 3, 7` CCK respectively. They
come from the even CPU-phase entries in
[vAmiga's `Agnus::syncWithEClock`](../../../ref_src/vAmiga/Core/Components/Agnus/Agnus.cpp).
The implementation computes the forward CCK distance to phase `1`. If that
window is fewer than `MIN_BUS_SYNC_CCK = 3` CCK away, it adds a full period of
`CCK_PER_ECLOCK = 5` CCK. The calculation produces the five delays without
a phase lookup table. The minimum interval comes from the reproduced
reference-emulator timing model. Measured hardware delay and CPU-clock setup
time fall outside what that interval establishes.

Focused regression tests verify the wait counts at all five represented CCK
phases, the effects of accepted reads and falling-edge write capture.
Production CPU timing remains incompletely verified. Comparing complete CPU
bus-cycle durations against Motorola Figures B-4/B-5 and resolving all ten
CPU-clock positions within E require further evidence. Guest-visible
integration faults can emerge during [native disk verification](../../../ROADMAP.md#native-disk-verification).
The hardware 10-19-clock range requires separate evidence.

In the [replacement-project CIA](../../../ref_src/amiga_replacement_project/cia/cia8520.v)
at revision `143ad5f1b5acae093ea66186e832a8ab4fb1bc0f`, timers count on rising
E and register writes and output changes occur on falling E. Its
[TOD helper](../../../ref_src/amiga_replacement_project/cia/cia_tod.v)
explicitly acquires and releases the read latch at the falling edge. In keeping
with that model, a named setter targets the next falling edge at any phase.
A production bus write accepted at phase `1` therefore commits one CCK later.
The delay follows from aligning capture to the falling edge. There is no
additional arbitrary E-clock delay assigned to each register.

Writes to ports, directions, timer latches, time and alarm, SDR, ICR masks and
controls all use that capture path. Port B output notifications start with
`PRB | !DDRB`, then apply enabled timer overrides. This makes an ordinary
input release the control line high regardless of the stored latch value.
Direction changes use the same committed output path.

A timer high-byte write loads a stopped timer. In one-shot mode it loads and
starts the timer regardless of START. Force-load strobes apply the reload value
without storing a readable bit. CRA also discards its documented unused bit 7.
Writing the middle or high time byte halts TOD until a write to the low byte.

Enabling a mask for an already pending ICR source asserts the master interrupt.
A live ICR read clears status at the accepted read phase. Reference
implementations differ in finer acknowledgment timing, which this model leaves
uncalibrated. Debug readers leave interrupt status, TOD latches and pending
work unchanged.

Under the policy the user approved on 2026-11-11, the queue keeps the last
value per selector until the next capture edge. Replacing a pending write
recomputes the remaining time to the same edge. It preserves the capture
schedule. Writes to different selectors remain ordered. A later write cannot
cancel an effect already captured.

The sixteen-slot buffer exceeds production CPU demand because each CIA accepts
at most one transfer per E period. A phase-1 write commits at phase 2, before
the chip can accept another transfer. The CPU bus first checks E-clock
synchronization and then checks for room in every selected queue. It completes
both checks before consuming either receiver's synchronization or staging any
data. An unavailable slot is fatal. The bus has no queue-related wait state.
Directly injected writes must drain before CPU bus traffic resumes. The path
permits only one producer at a time.

Valid direct setters keep at most one pending operation per connected selector.
At capture, a due buffer of the same capacity applies all retained operations.
A write replaced before capture has no effect. The shared staging helper treats
attempted overflow as a fatal internal failure, without immediate application
or a queue-related retry.

The serialized CIA value contains its divider phase, pending bus countdown,
chip-local register selectors `0..15`, write values/countdowns/modes, active
registers, TOD latch operation/value, unconsumed port-output changes,
serial holding/shift state, captured CNT edges, timer waveforms and PC pipeline.
[Snapshots](Application/SaveState.md) store this encoding directly. Reset cancels
pending work. Restore preserves the saved values and resumes effects at their
original phase. The loader leaves output polling to subsequent execution.
Silicon edge calibration remains separate work. The register map still lists
unspecified delays as unknown.

## Reference documentation and source

- Serializable chip state, E-clock synchronization, queued named register access and capture are implemented in [`cia.rs`](../../../crates/cia/src/cia.rs) and [`registers.rs`](../../../crates/cia/src/registers.rs).
- Synchronous peripheral transfer, VPA/VMA and E-clock duty cycle are described in the [Motorola M68000 User's Manual, Appendix B](../Reference/68000%20User's%20Manual/13%20-%20Appendix%20B%20-%20M6800%20Peripheral%20Interface.md).
- The CCK model uses a reference-emulator synchronization countdown from [vAmiga Agnus](../../../ref_src/vAmiga/Core/Components/Agnus/Agnus.cpp).
- The [replacement-project CIA](../../../ref_src/amiga_replacement_project/cia/cia8520.v) and its [TOD helper](../../../ref_src/amiga_replacement_project/cia/cia_tod.v) supply reconstructed edge ordering for writes, timer counts and TOD read latches. Measuring silicon delays requires separate evidence.
- Timer modes, TOD latching, serial shifting, port pins and interrupt output are described in the [Amiga Hardware Reference Manual, Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md).
- Appendix F describes the high-byte latch without an alarm-mode exception. The vAmigaTS results reported for an A500 8A show a frozen middle byte in `latch6` and a counting middle byte in `latch9`, which selects the alarm for the high-byte read and then returns to time mode. The tests observe subsequent reads. The emulator adopts the vAmiga and WinUAE rule that checks CRB bit 7 at the high-byte read. [vAmigaTS results](../../../ref_src/vAmigaTS/CIA/TOD/tod/README.md#latch1-to-latch9), [vAmiga register reads](../../../ref_src/vAmiga/Core/Components/CIA/CIARegs.cpp), [WinUAE TOD reads](https://github.com/tonioni/WinUAE/blob/95462f58f85eca77541f777a708e441db274a98b/cia.cpp#L1601-L1612).
- The CIA byte lanes and addresses are mapped separately from the custom register block in the [Amiga Hardware Reference Manual, Appendix D](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md).
- [Machine Loop.md](Machine%20Loop.md) describes how the two CIA instances connect to the machine.
