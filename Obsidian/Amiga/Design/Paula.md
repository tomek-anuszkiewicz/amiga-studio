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

- It holds serial data and control state.
- It holds disk data, synchronization and status registers, controls the disk DMA arming sequence, and reports transfer completion. [Floppy](Floppy.md) describes the drive and disk-stream integration.
- It exposes the sampled proportional input and button state supplied through [Game Ports](Game%20Ports.md).
- It coordinates delayed register changes, DMA enable signals, and interrupt requests between these functions and the rest of the machine.

## Register propagation

Concrete queued setters in [registers.rs](../../../crates/paula/src/registers.rs) delay interrupt, DMA, audio, disk, serial and port register effects until private application. DMACON retains its **2 CCK** reference-model delay; other data writes use **1 CCK**, all in `Pipeline` mode. STRHOR acts immediately at accepted selection without a queue entry. Audio pointer register ownership stays exclusively in [Agnus](Agnus.md); Paula's audio children retain their existing engine state. CPU INTREQ changes remain queued register writes. Delivered INT2/INT3/INT6
levels are sampled after due writes, independently of register queue capacity.
An asserted input wins a coincident clear under the explicit CCK phase policy;
release preserves the request latch. The 32-entry queue serves only register writes; no synthetic interrupt selector
or reserved input slot remains. Full capacity rejects writes without early
application, and replay applies and returns every matured register entry. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) defines modes, provisional timing and acceptance. Autonomous audio transitions and disk counters retain their separate engine interfaces.

The UART and STRHOR reaction remain incomplete; see the [chipset reconstruction work](../../../ROADMAP.md#chipset-reconstruction).
The [register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md#state-owners-and-hardware-meaning)
identifies the register families, children and accessors. Live `read_dskbytr`
acknowledges byte availability; `peek_dskbytr` is neutral. Interrupt register
reads do not clear requests.

## Pin transport and deferred receiver work

The serializable `PaulaPins` state in [pins.rs](../../../crates/paula/src/pins.rs)
is exposed as `paula.pins`, with pin input methods in the same module. Registers, mutation
queues and UART engine state remain in their existing modules. Delivered
active-low INT2/INT3/INT6 levels default high and survive snapshots separately
from INTREQ. They are sampled after due register writes as described above;
receiving a pin level does not insert a synthetic register mutation.
`pins.incoming` retains routed values until `sample_input_pins()` applies them
at the next input phase. It also retains the explicitly decoded POT counters
and vertical event; those fields are model bridges, not new physical pins.

[`Machine Loop`](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
captures `pins.dmal_output_high` and drives Agnus's DMAL input before the next
subsystem step. `pins.rga_strobe` receives the documented STRHOR address event from that
same capture phase. `PaulaSync` retains the local CCK phase since reception.
CPU STRHOR writes reset the same phase immediately at accepted selection, without a queue entry. Request framing/decoding and audio/disk state-machine
reaction belong to [2.4](../../../ROADMAP.md#chipset-reconstruction); the
[DMA protocol](DMA.md#paula-request-framing-and-state-ownership) fixes their
ownership and complete-exchange obligations. A held RGA/data output enters a
named AUDxDAT queue only after acceptance, without another motherboard delay.

The serial connector is unconnected: RXD stays idle high, reported by SERDATR
bit 11, and external CTS/DSR/CD remain inactive at CIA-B. There is no serial
connector pin state or motherboard transport. CIA-B PA6/PA7 retain their
RTS/DTR register and direction logic without an attached endpoint; they do
not automatically gate Paula's shift engine. `SerialPort` retains SERDAT,
SERDATR and SERPER and their register behavior. Serial framing, holding/shift transitions,
TBE/TSRE/RBF/overrun and interrupts remain unreconstructed; register state
does not prove a byte exchange. See HRM Chapter 8, Serial Interface.

[Step 2.4](../../../ROADMAP.md#chipset-reconstruction) completes the UART chip
logic with the connector disconnected; transmission still advances without a
receiver. Deterministic component inputs can verify receive logic separately.
An attached device, connector transport and full communication verification
remain deferred to [12.4](../../../ROADMAP.md#serial-parallel-device-support).

`pins.disk_read_high` receives the drive's raw read line. `pins.disk_write_high` and
`pins.disk_write_enable` drive the modeled Gary/buffered connector output, separate
from the retained legacy MFM/media bridge. These pin checks attach no image
and perform no media transfer. `receive_pot_inputs` accepts the current digital
POT button levels before engine work, without a pending copy. The earlier
decoded POT counters remain a separate captured bridge. Committed POTGO
data/direction is delivered back to the game connector at its existing phase
before those button levels are sampled. Digital pin reaction, analog counting and internal disk behavior
are not certified by that projection.

All delivered input/output levels and pending receiver register events are
serializable. Construction/reset initializes released lines; reset cancels
pending requests. Transport latency and receiver-local processing are separate;
unknown silicon/CDAC/analog delays do not acquire measured status merely
because a CCK boundary is explicit.

## References

- [Machine Loop](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words) and [signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md#three-reference-comparison-and-selected-transport): Physical connections, selected transport phases and the limits of the three reference models.

- [General Architecture](General%20Architecture.md): Places Paula in the machine and shows its component relationships.
- [`crates/paula/src/paula.rs`](../../../crates/paula/src/paula.rs): Living implementation of Paula's state and coordination.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Hardware meanings, access classes and current per-recipient delays.
- [Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Serial register/bit protocol, disk controls and POT input/output behavior.
- [`paula/pins.rs`](../../../crates/paula/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
