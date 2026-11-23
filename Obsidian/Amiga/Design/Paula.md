---
title: "Paula"
aliases: ["MOS 8364"]
tags: ["amiga", "design", "paula", "audio", "interrupts"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-06
updated: 2026-11-23
related: ["[Audio.md](Audio.md)", "[Interrupts.md](Interrupts.md)", "[Floppy.md](Floppy.md)", "[Game Ports.md](Game%20Ports.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/paula"
---
# Paula

Paula handles four-channel sound, system interrupts, serial communication,
disk data and control, and the proportional inputs of the game ports. Audio
and disk data reach Paula through the machine's custom-chip bus. Paula encodes
requests for that data on DMAL, and Agnus controls the corresponding DMA
transfers and owns their memory addresses. [Agnus](Agnus.md)

The audio and interrupt chapters specify those circuits. The floppy chapter
explains the drive-side stream, and the game ports chapter specifies controller
connections. [Audio](Audio.md), [Interrupts](Interrupts.md),
[Floppy](Floppy.md), and [Game Ports](Game%20Ports.md)

## Register propagation

Paula's register setters queue writes for interrupt, DMA, audio, disk,
serial, and port registers. Each write takes effect when Paula applies its
queue entry. Direct application is private to Paula. DMACON retains its
**2 CCK** reference-model delay. Other data writes use **1 CCK**. All use
`Pipeline` mode. Paula applies a sync address strobe immediately when the
bus router accepts the selection, bypassing the queue. The strobe's effects
are described below.
[registers.rs](../../../crates/paula/src/registers.rs)

Agnus alone owns the audio pointer registers. Paula's audio children retain
their existing engine state. Autonomous audio transitions and disk counters
keep their separate engine interfaces. [Agnus](Agnus.md)

Paula queues CPU writes to INTREQ. During its step, Paula applies due
writes and then samples the delivered INT2/INT3/INT6 levels. If a write
clears a request in the same step that Paula samples the corresponding
asserted input, that input sets the request again. Sampling a released
input leaves the request latch as it is. The 32-entry queue holds only
register writes. Paula samples inputs regardless of queue capacity. The
design has no synthetic interrupt selector or reserved input slot.

Attempting to insert an entry beyond capacity stops the owner without early
application or retry. Queue replay applies and returns every matured register
entry. The motherboard router defines modes, provisional timing, and
acceptance. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips)

The register matrix identifies the register families, children, and
accessors. [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md#state-owners-and-hardware-meaning)

A live `read_dskbytr` acknowledges byte availability. Inspection through
`peek_dskbytr` leaves state unchanged, and reading interrupt registers
leaves requests pending.

## RGA synchronization and VERTB

The approved model delivers STREQU, STRVBL, STRHOR, and STRLONG to Paula
through the shared RGA path. The hardware manual explicitly lists Paula for
STRHOR. The additional decoding and VERTB edge follow the replacement Paula
model. Original-chip timing remains unmeasured by these sources.
[HRM Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#strequ-strhor-strlong-strvbl)
and [replacement Paula model](../../../ref_src/amiga_replacement_project/paula/Paula.v)

STREQU, STRVBL, and STRHOR reset Paula's horizontal phase and capture the
pending audio and disk request frame, including during vertical blanking.
STRLONG leaves the equalization state alone and does not restart the frame.
When STREQU arrives with the equalization latch clear, Paula sets the
latch and requests VERTB once. Further STREQU events leave the latch set.
STRVBL or STRHOR clears it so a later entry can request VERTB again. In
the normal Agnus sequence this yields one request per field, including
interlaced fields. A CPU write that selects a sync address uses
the same decoder immediately through `strobe_sync(address)`, regardless of
data-queue capacity.

The machine loop delivers a captured Agnus strobe address before Paula
steps. Paula updates its line state before audio and DMAL work and applies
the VERTB request after due register writes. Under this CCK model, a new
request therefore wins a coincident INTREQ clear. Paula derives the VERTB
request from the address. The model has no decoded Agnus VBlank event or
additional VERTB wire.
[Machine Loop](Machine%20Loop.md#rga-synchronization-transport)

Snapshots preserve pending RGA delivery, the local phase, the captured
request frame, and the equalization latch. Reset clears them. Restoration
resumes from the saved state without replaying an edge.

## UART reconstruction contract

HRM 8 Serial Interface and Appendix A define a transmit holding word and a
separate shift register. When the shifter is empty, a nonzero SERDAT moves
from the holding word to the shifter. Transmission sends the start bit
first, then sends data least-significant-bit first through the highest
set bit, which is the stop bit. Each bit lasts SERPER[14:0]+1 CCK. Zero
produces no packet. TBE becomes true and requests its interrupt when the
holding word moves. TSRE becomes true after the final bit time. A second
word can wait in the holding register while the first shifts.

UARTBRK forces the exposed TXD low. The connector remains without a receiver,
and the CIA RTS/DTR lines are independent of this action. The transmitted
word determines TX length. SERPER bit 15 selects eight or nine received data bits.

Paula's RX input samples the start, data, and stop levels at bit centers.
When a word completes, Paula stores its data and the sampled stop bit in
SERDATR and requests RBF. If another word completes while RBF is still
pending, Paula replaces the data and sets overrun. Acknowledging INTREQ
clears overrun. Reading the data leaves the state as it was.

Production always supplies idle-high RXD. Component injection supplies
deterministic inputs for verification without creating a connector or
endpoint. Snapshots preserve the counter, shifter, and holding state so
execution can continue. The implementation quantizes half-bit sampling to
CCK boundaries. Analog timing remains uncalibrated.

## Pin transport and deferred receiver work

Paula exposes its serializable `PaulaPins` state as `paula.pins`. The same
module provides the pin input methods. Registers, mutation queues, and UART
engine state remain in their existing modules.
[pins.rs](../../../crates/paula/src/pins.rs)

`pins.incoming` holds routed values until `sample_input_pins()` applies them
at the next input phase. Delivered active-low INT2/INT3/INT6 levels default
high and survive snapshots separately from INTREQ. Paula samples them after
due register writes, without inserting synthetic register mutations.
The decoded POT counters still use a separate model bridge that adds no
physical pins.

The coordinator captures `pins.dmal_output_high` and drives Agnus's DMAL
input before the next subsystem step. `pins.rga_strobe` receives all four
sync addresses at the same capture phase. Paula's RGA receiver handles
their local effects. A held RGA/data output enters its named AUDxDAT or
DSKDAT queue only after acceptance, with no further motherboard delay.
[`Machine Loop`](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
and [RGA receiver](#rga-synchronization-and-vertb)

The DMA protocol describes how Paula generates the request frame and how
Agnus decodes it. [DMA protocol](DMA.md#paula-request-framing-and-state-ownership)

The serial connector is unconnected. RXD stays idle high, as reported by
SERDATR bit 11, and CIA-B receives inactive external CTS/DSR/CD. The
motherboard implements no serial connector pin state or transport. CIA-B
PA6/PA7 still provide RTS/DTR register and direction logic without an
attached endpoint. Paula's shift engine runs independently of automatic
RTS/DTR gating.

`SerialPort` reconstructs SERDAT, SERDATR, and SERPER behavior, including
holding and shift transitions, TBE/TSRE/RBF/overrun, and interrupts.
Component tests establish framing. An exchange with a serial endpoint remains
unverified by disconnected production operation. See HRM Chapter 8, Serial Interface.

UART transmission advances with the connector disconnected and no receiver.
Deterministic component inputs can verify receive logic separately. An attached
device, connector transport, and full communication verification remain deferred.
[12.4](../../../ROADMAP.md#serial-parallel-device-support)

`pins.disk_read_high` receives the drive's raw read line.
`pins.disk_write_high` and `pins.disk_write_enable` drive the modeled
Gary/buffered connector output. The raw track buffers belong to the drive
and are separate from these pins. These pin checks attach no image and
perform no media transfer.

Before sampling digital POT buttons, the model delivers committed POTGO
data and direction back to the game connector at the existing phase.
`receive_pot_inputs` accepts the current button levels before engine work
without keeping a pending copy. The earlier decoded POT counters still use
a separate captured bridge.
Digital pin reaction, analog counting, and internal disk behavior remain
outside the evidence from the POTGO output projection.

Snapshots preserve all delivered input and output levels and all pending
receiver register events. Construction and reset initialize released
lines. Reset also cancels pending requests. Transport latency and
receiver-local processing have separate timing. The CCK boundary defines
the model phase. The boundary alone gives no measurement of unknown
silicon, CDAC, or analog delays.

## Reconstructed disk engine contract

HRM Chapter 8 Disk DMA Channel Control and Appendix A govern the
reconstructed controller. Paula assembles the incoming active-low
transition stream and owns the three-word FIFO, the byte-ready status
that clears on a read, sync comparison, two-write arming, and word count.
It encodes three direction and request pairs on DMAL. Agnus supplies all
memory addresses.

On a read selection, the machine transfers the word Paula presents on
DSKDATR into RAM and consumes it. On a write selection, the machine
fetches a word from RAM and sends it through the queued DSKDAT path.
The pointer and count advance only on actual transfers. Paula requests
DSKBLK when the count reaches zero, and any write words already in the
FIFO continue through the serial output.

With WORDSYNC enabled, Paula waits for the first DSKSYNC match and starts
read DMA with the following word. Each later match sets the boundary for
the next word. If the match completes a word, Paula queues it when space is available.
If it ends between word boundaries, Paula discards the partial word.
If a read starts on the first of two consecutive `$4489` sync marks,
the second mark is the first DMA word.
[HRM Chapter 8, DSKSYNC](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md#dsksync--disk-input-synchronizer)
and [Inside Paula's Floppy Controller, Part I, section 4.9 and Figure 4-4](https://www.techtravels.org/wp-content/uploads/2026/08/Inside_Paulas_Floppy_Controller_Part_I.pdf?v=20260904#page=28)

Each match can request DSKSYN even when WORDSYNC is disabled. MSBSYNC skips
zero bits before each byte's leading one. DSKBYTR.DMAON requires the
armed/active engine and both DMACON gates.

The CCK-level data separator uses nominal seven-clock fast and fourteen-clock
slow bit cells. A falling read pulse re-anchors the cell, and the separator
samples near its center. The model handles deterministic inputs. Analog PLL
calibration and decoding from flux media are outside its scope. If the FIFO
overflows, Paula keeps the buffered words and discards the newly completed
word. Sub-CCK precompensation and the manual's timing-dependent loss of the
final bit or read word remain uncalibrated. ADKCON retains their control
bits. The model provides no measurement of pulse shifts.

Paula handles raw MFM words and does not decode them into sectors. The
autonomous floppy read stream delivers nominal two-microsecond cells
through the existing connector. The separator re-anchors on transitions
despite seven/eight-CCK cell quantization. These read-boot observations
leave analog PLL calibration and media writes unverified.
[Floppy](Floppy.md#autonomous-adf-reads-and-continuation)

The GUI activity observation records an event only when a live read consumes
a fresh byte, a granted read word is consumed after RAM admission, or a
buffered write word starts serialization. Register setup, empty retries, and
debugger peeks add no event. [GUI activity observation](Application/GUI.md)

The motherboard supplies the recipient context separately from the physical
pins. Snapshots preserve fixed wrapping sequences for each drive without
changing guest-visible counts. [Motherboard](Machine%20Loop.md)

## References

- [Replacement Paula](../../../ref_src/amiga_replacement_project/paula/Paula.v): Reference model for sync decoding and the VERTB edge; not a silicon timing measurement.
- [HRM Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md#strequ-strhor-strlong-strvbl): Documented refresh-slot roles and chip listings for the sync addresses.
- [Machine Loop](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words) and [signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md#three-reference-comparison-and-selected-transport): Physical connections, selected transport phases and the limits of the three reference models.

- [General Architecture](General%20Architecture.md): Places Paula in the machine and shows its component relationships.
- [`crates/paula/src/paula.rs`](../../../crates/paula/src/paula.rs): Living implementation of Paula's state and coordination.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Hardware meanings, access classes and current per-recipient delays.
- [Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Serial register/bit protocol, disk controls and POT input/output behavior.
- [`paula/pins.rs`](../../../crates/paula/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
