---
title: "Paula"
aliases: ["MOS 8364"]
tags: ["amiga", "design", "paula", "audio", "interrupts"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-06
updated: 2026-11-12
related: ["[Audio.md](Audio.md)", "[Interrupts.md](Interrupts.md)", "[Floppy.md](Floppy.md)", "[Game Ports.md](Game%20Ports.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/paula"
---
# Paula

Paula handles four-channel sound, system interrupts, serial communication, disk data and control, and the proportional inputs of the game ports. It receives audio and disk data through the machine's custom-chip bus; [Agnus](Agnus.md) controls the corresponding DMA transfers.

## Internal components

| Component | Role |
| --- | --- |
| [`paula`](../../../crates/paula) | Owns the Paula state, routes register access, and coordinates its components. |
| [`audio`](../../../crates/paula/src/audio.rs) | Runs the four audio channels and produces sound samples. See [Audio](Audio.md). |
| [`interrupts`](../../../crates/paula/src/interrupts.rs) | Holds interrupt requests and enables, and resolves the CPU interrupt level. See [Interrupts](Interrupts.md). |

`audio`, `interrupts`, `disk`, `dmal` and [`serial`](../../../crates/paula/src/serial.rs) are modules of the Paula crate. Source files remain flat; the audio channel implementation is `audio_channel.rs`. [Audio](Audio.md) and [Interrupts](Interrupts.md) are detailed chapters of this chip's design. Their component tests belong to Paula. The `dmal.rs` module frames both audio and disk requests; Agnus owns the corresponding RAM addresses and transfers.

## Other responsibilities in `paula`

- It holds serial data and control state.
- It holds disk data, synchronization and status registers, controls the disk DMA arming sequence, and reports transfer completion. [Floppy](Floppy.md) describes the drive and disk-stream integration.
- It exposes the sampled proportional input and button state supplied through [Game Ports](Game%20Ports.md).
- It coordinates delayed register changes, DMA enable signals, and interrupt requests between these functions and the rest of the machine.

## Register propagation

Concrete queued setters in [registers.rs](../../../crates/paula/src/registers.rs) delay interrupt, DMA, audio, disk, serial and port register effects until private application. DMACON retains its **2 CCK** reference-model delay; other data writes use **1 CCK**, all in `Pipeline` mode. Sync address strobes act immediately at accepted selection without a queue entry; their effects are defined below. Audio pointer register ownership stays exclusively in [Agnus](Agnus.md); Paula's audio children retain their existing engine state. CPU INTREQ changes remain queued register writes. Delivered INT2/INT3/INT6
levels are sampled after due writes, independently of register queue capacity.
An asserted input wins a coincident clear under the explicit CCK phase policy;
release preserves the request latch. The 32-entry queue serves only register writes; no synthetic interrupt selector
or reserved input slot remains. Full capacity rejects writes without early
application, and replay applies and returns every matured register entry. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) defines modes, provisional timing and acceptance. Autonomous audio transitions and disk counters retain their separate engine interfaces.

The [register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md#state-owners-and-hardware-meaning)
identifies the register families, children and accessors. Live `read_dskbytr`
acknowledges byte availability; `peek_dskbytr` is neutral. Interrupt register
reads do not clear requests.

## RGA synchronization and VERTB

The approved model delivers STREQU, STRVBL, STRHOR and STRLONG to Paula through
the shared RGA path. [HRM Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#strequ-strhor-strlong-strvbl)
explicitly lists Paula for STRHOR. The additional decoding and VERTB edge follow
the [replacement Paula model](../../../ref_src/amiga_replacement_project/paula/Paula.v);
they do not establish measured original-chip timing.

STREQU, STRVBL and STRHOR reset the local horizontal phase and capture the
pending audio/disk request frame, including during vertical blanking. STRLONG
does not restart that frame or change the equalization state. Entering STREQU
sets the equalization latch and requests VERTB once. Further STREQU events
leave it set; STRVBL or STRHOR clears it for the next entry. This yields one
request per field in the normal Agnus sequence, including interlaced fields.
CPU-selected sync addresses use the same decoder immediately through
`strobe_sync(address)`, independently of data-queue capacity.

For a captured Agnus strobe, [Machine Loop](Machine%20Loop.md#rga-synchronization-transport)
delivers the address before Paula steps. Paula updates its line state before
audio and DMAL work, then applies the VERTB request after due register writes,
so a new request wins a coincident INTREQ clear under this CCK model. There is
no decoded Agnus VBlank event or additional VERTB wire. Pending RGA delivery,
the local phase, captured request frame and equalization latch serialize;
reset clears them and restoration does not replay an edge.

## UART reconstruction contract

HRM 8 Serial Interface and Appendix A define a separate transmit holding word
and shift register. A nonzero SERDAT moves to an empty shifter, emits a start
bit and then data least-significant-bit first through its highest set stop bit.
Each bit lasts SERPER[14:0]+1 CCK. Zero does not start a packet. TBE becomes
true and requests its interrupt when the holding word moves; TSRE becomes true
after the final bit time. A second word can wait while the first shifts.
UARTBRK forces the exposed TXD low without attaching a receiver or gating CIA
RTS/DTR. SERPER bit 15 selects eight or nine received data bits, not TX length.

The component RX input samples the start, data and stop levels at bit centers.
SERDATR retains data plus the observed stop bit, and an RBF request marks a
completed word. Another completed word while RBF is pending replaces the data
and sets overrun. INTREQ acknowledgement clears overrun; merely reading data
is neutral. Production always supplies idle-high RXD. Component injection is
for deterministic verification and does not create a connector or endpoint.
Counter/shifter/holding state serializes for continuation. CCK quantization of
half-bit sampling is an implementation model rather than calibrated analog timing.

## Pin transport and deferred receiver work

The serializable `PaulaPins` state in [pins.rs](../../../crates/paula/src/pins.rs)
is exposed as `paula.pins`, with pin input methods in the same module. Registers, mutation
queues and UART engine state remain in their existing modules. Delivered
active-low INT2/INT3/INT6 levels default high and survive snapshots separately
from INTREQ. They are sampled after due register writes as described above;
receiving a pin level does not insert a synthetic register mutation.
`pins.incoming` retains routed values until `sample_input_pins()` applies them
at the next input phase. The explicitly decoded POT counters remain a model
bridge, not additional physical pins.

[`Machine Loop`](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
captures `pins.dmal_output_high` and drives Agnus's DMAL input before the next
subsystem step. `pins.rga_strobe` receives all four sync addresses at that
same capture phase; the [RGA receiver](#rga-synchronization-and-vertb) owns
their local effects. The [DMA protocol](DMA.md#paula-request-framing-and-state-ownership)
describes the generated request frame and Agnus decoding. A held RGA/data output
enters its named AUDxDAT or DSKDAT queue only after acceptance, without another
motherboard delay.

The serial connector is unconnected: RXD stays idle high, reported by SERDATR
bit 11, and external CTS/DSR/CD remain inactive at CIA-B. There is no serial
connector pin state or motherboard transport. CIA-B PA6/PA7 retain their
RTS/DTR register and direction logic without an attached endpoint; they do
not automatically gate Paula's shift engine. `SerialPort` reconstructs SERDAT, SERDATR and SERPER behavior with holding/shift
transitions, TBE/TSRE/RBF/overrun and interrupts. Component tests establish
framing; disconnected production operation does not prove an endpoint exchange. See HRM Chapter 8, Serial Interface.

UART transmission advances with the connector disconnected and no receiver. Deterministic component inputs can verify receive logic separately.
An attached device, connector transport and full communication verification
remain deferred to [12.4](../../../ROADMAP.md#serial-parallel-device-support).

`pins.disk_read_high` receives the drive's raw read line. `pins.disk_write_high` and
`pins.disk_write_enable` drive the modeled Gary/buffered connector output, separate
from host image buffers and deferred media generation. These pin checks attach no image
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

## Reconstructed disk engine contract

HRM Chapter 8 Disk DMA Channel Control and Appendix A govern the reconstructed
controller. Paula assembles the incoming active-low transition stream, owns a
three-word FIFO, byte-ready/read-clear status, sync comparison, two-write
arming and word count. It encodes three direction/request pairs on DMAL;
Agnus owns all memory addresses. A read selection consumes DSKDATR into RAM;
a write selection fetches RAM into the existing queued DSKDAT path. Pointer
and count advance for actual transfers, and zero count requests DSKBLK.
Writes already in the FIFO drain through the serial output. WORDSYNC waits
for a match and aligns subsequent words; each match can request DSKSYN even
when WORDSYNC is disabled. MSBSYNC skips zero bits before each byte's leading
one. DSKBYTR.DMAON requires the armed/active engine and both DMACON gates.

The CCK-level data separator uses nominal seven-clock fast and fourteen-clock
slow bit cells, with falling read pulses re-anchoring the cell and a sample
near its center. This is a functional deterministic-input model, not a
calibrated analog PLL or flux-media decoder. FIFO overflow retains buffered
words and discards a newly completed word. Sub-CCK precompensation and the
manual's timing-dependent final-bit/read-word loss are not calibrated by this
model. ADKCON retains their control bits without claiming measured pulse shifts.
Raw MFM words are never decoded into sectors inside Paula. Drive/media
execution remains outside this reconstruction's explicit boundary.

## References

- [Replacement Paula](../../../ref_src/amiga_replacement_project/paula/Paula.v): Reference model for sync decoding and the VERTB edge; not a silicon timing measurement.
- [HRM Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#strequ-strhor-strlong-strvbl): Documented refresh-slot roles and chip listings for the sync addresses.
- [Machine Loop](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words) and [signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md#three-reference-comparison-and-selected-transport): Physical connections, selected transport phases and the limits of the three reference models.

- [General Architecture](General%20Architecture.md): Places Paula in the machine and shows its component relationships.
- [`crates/paula/src/paula.rs`](../../../crates/paula/src/paula.rs): Living implementation of Paula's state and coordination.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Hardware meanings, access classes and current per-recipient delays.
- [Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Serial register/bit protocol, disk controls and POT input/output behavior.
- [`paula/pins.rs`](../../../crates/paula/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
