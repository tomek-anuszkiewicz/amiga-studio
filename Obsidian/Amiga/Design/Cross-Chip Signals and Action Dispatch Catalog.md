---
title: "Cross-Chip Signals and Action Dispatch Catalog"
aliases: ["Cross-Chip Signals Catalog", "Action Dispatch Catalog", "Inter-Chip Signal Matrix"]
tags: ["amiga", "design", "chips", "signals", "propagation", "action-dispatch"]
category: "Design"
subsystem: "general"
status: "draft"
created: 2026-09-14
updated: 2026-10-14
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Floppy.md](Floppy.md)", "[DMA.md](DMA.md)", "[Interrupts.md](Interrupts.md)", "[Game Ports.md](Game%20Ports.md)", "[Keyboard.md](Keyboard.md)"]
---

# Cross-Chip Signals and Action Dispatch Catalog

The Amiga 500 connects its chips through shared buses and dedicated electrical lines. This catalog records the connections that carry behavior across chip or peripheral boundaries. Its board-level evidence is the Commodore A500 Rev. 6A/7 schematic, available as a vector redraw, checked against the custom-chip pin list and hardware manuals. A drawn net establishes connectivity; the manuals explain what a receiver does with it. Neither establishes a general propagation delay or complete event order for an emulator.

This is a hardware model for a future [machine coordinator](Machine%20Loop.md), not an account of current code. [DMA](DMA.md) owns slot allocation and arbitration rules; this catalog identifies requests, buses, strobes, and result paths. The [custom-register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md) identifies addresses selected on `RGA`. Names prefixed by `_` in the schematic or suffixed by `*` in the manuals are active low unless otherwise specified. `DMAL` is explicitly active high. Evidence keys are linked at the end.

## Shared buses, requests, and beam timing

| Signal or bus | Source → receiver | Hardware role | Evidence |
| :--- | :--- | :--- | :--- |
| `D[15:0]`, `DRD[15:0]` | CPU and motherboard latches ↔ Chip RAM and custom chips | Shared word path. DMA reads deliver RAM data to a selected chip; disk write DMA reverses direction. The CPU side is buffered from the chip-memory side. | S2, S4, S5; P, F |
| `DRA/MA`, `RAS0/1*`, `CASU/L*`, `WE/RRW` | Agnus → DRAM | Address, row/column, byte-lane, and read/write control for CPU memory cycles, DMA, and refresh. Refresh does not deliver a peripheral word. | S2, F |
| `BLIT*`, `BLISS*` and CPU bus strobes | Agnus ↔ Gary; CPU → Agnus/Gary | On the Rev. 6A/7 drawing, Agnus and Gary exchange Blitter control lines while CPU address, read/write, and strobes feed the shared memory-control network. The older Fat Agnus description calls a related pin `DBR*`; equivalence is not established here. | S2, F |
| `RGA[8:1]` | Agnus → Denise and Paula | Selects custom registers and DMA destinations. A receiving chip samples or drives the shared data bus; the address code is not a data payload. | S2, S4, S5; P, F |
| Shared custom-register selection | CPU or Copper write → Agnus `RGA` encoding → listed chip recipients | One write may be visible to more than one chip: Appendix B lists `DMACON` for Agnus, Denise, and Paula, and `BPLCON0` for Agnus and Denise. This is register-bus fan-out, not a separate private signal between chips. | S2, S4, S5; P, B |
| `DMAL` | Paula → Agnus | Active-high request for audio and/or disk DMA. Agnus schedules and addresses the granted transfer. The pin list confirms Paula output and Agnus input. | S2, S4; P, F |
| `CCK`, `CCKQ`, `7MHz`, `CDAC` | Oscillator/Agnus clock network → CPU, Gary, Denise, Paula | Related clock phases drive chip work and bus timing. CIA `E` is a distinct CPU-side clock input, not another name for CCK. | S2, S4, S5, S6; P, F |
| `HSYNC*`, `VSYNC*`, `CSYNC*` | Agnus timing network → video path and other timing inputs | Horizontal, vertical, and composite sync. Agnus can instead accept external horizontal/vertical sync in its documented resync mode. Denise receives composite sync for video blanking. | S2, S5, S6; P, F |
| `STREQU`, `STRVBL`, `STRHOR`, `STRLONG` on `RGA` | Agnus refresh-time address code → Denise; `STRHOR` also → Paula | Sync is also encoded as register-address strobes during refresh opportunities, distinct from physical `HSYNC*`/`VSYNC*` lines. | A, F |
| `VSYNC*` or `EXTICK`; `HSYNC*` | Video timing/expansion tick → CIA-A TOD; horizontal sync → CIA-B TOD | The CIA-A TOD source is jumper-selected on this board; CIA-B counts horizontal sync events. These inputs are separate from the timer E-Clock. | S6, AF |

### Paula–Agnus DMA request timing

Agnus marks the beginning of a scan line with a horizontal sync strobe encoded on `RGA` during refresh. The Hardware Reference Manual lists `STRHOR` (`$03C`) for Paula as well as Denise. Thus Paula can synchronize its request sequence from the register-address bus and the shared clocks; it does not need a separate `HSYNC*` input. A [reconstructed Paula circuit](https://github.com/nonarkitten/amiga_replacement_project/blob/master/paula/Paula.v) resets its `DMAL` shift counter when it decodes a horizontal strobe and loads pending disk and audio requests for that line. This is a useful implementation model, not direct proof of Paula's internal sampling edge. The reconstruction also treats `STREQU` and `STRVBL` as horizontal strobes, whereas Appendix B explicitly lists Paula only for `STRHOR`.

The physical A500 [logic-analyzer capture](https://www.techtravels.org/2026/08/from-flux-to-filenames-335ms-inside-paula/) shows that Paula sends a fixed 16-position frame over the single active-high `DMAL` wire at the start of each line, one bit per CCK (`$00`–`$0F`). Agnus knows the kind of request from its position in this frame; `DMAL` carries no address or channel number. Pending work in Paula's disk word queue or audio state machines supplies the bits. Agnus then decides whether and when to grant the corresponding transfer; the request itself moves no word.

| `DMAL` position | Meaning | Corresponding transfer opportunity |
| :--- | :--- | :--- |
| `$00`–`$01` | Zero in the measured frame and reconstructed shift register. | None. |
| `$02/$03`, `$04/$05`, `$06/$07` | Three disk pairs: direction bit followed by a request bit, one pair per queued word. For disk reads the direction bits are zero. | Requests at `$03`, `$05`, `$07` led to disk grants at `$07`, `$09`, `$0B` in the capture: four CCK later. |
| `$08/$09`, `$0A/$0B`, `$0C/$0D`, `$0E/$0F` | One pair for each audio channel, 0 through 3. The reconstruction places a restart-related qualifier first and the channel's data request second. | The second-bit positions align four CCK before the audio transfer positions `$0D`, `$0F`, `$11`, `$13`; this audio timing is inferred from the reconstruction and [DMA slot schedule](DMA.md), not measured by the cited disk capture. |

For disk reads, a queue with one, two, or three waiting words asserts the last request bit, the last two, or all three. The capture verifies the disk request positions and four-CCK request-to-grant interval. The exact audio pair semantics and the treatment of alternate horizontal strobes still need original-hardware confirmation before a pin-level emulator model is fixed. The `AUDxDR` and `AUDxDSR` names in the Hardware Reference Manual describe audio state-machine requests; they are encoded into `DMAL` rather than separate board wires.

## Interrupts and reset

| Signal | Source → receiver | Hardware role | Evidence |
| :--- | :--- | :--- | :--- |
| `INT3*` | Agnus → Paula | Active-low Blitter-complete request enters Paula's interrupt controller. | S2, S4; P, F, H7 |
| `INT2*` | CIA-A and expansion connector → Paula | Active-low CIA-A/external low-level request, represented by Paula's `PORTS` level-2 source. | S4, S6, S7; P, AF, H7 |
| `INT6*` | CIA-B and expansion connector → Paula | Active-low CIA-B/external high-level request, represented by Paula's `EXTER` level-6 source. | S4, S6, S7; P, AF, H7 |
| `IPL[2:0]*` | Paula → 68000 | Paula masks and prioritizes pending requests, then drives the CPU interrupt level. Audio, disk, serial, vertical blank, and Copper are documented sources; the schematic does not imply a separate board wire for each. | S2, S4; P, H7 |
| `KBRESET*`, `RESET*` | Keyboard and power-on circuit → Gary/reset network → CPU, custom chips, CIAs | Keyboard or power-on reset reaches the shared system reset path, rather than a custom-register write. | S2, S4, S6, S7; P, H |

## Peripheral connections

| Signal family | Source → receiver | Hardware role | Evidence |
| :--- | :--- | :--- | :--- |
| `MTR*`, `SEL[3:0]*`, `SIDE*`, `DIR`, `STEP*` | CIA-B port B → drive control, partly through Gary | Motor, drive select, head side, direction, and stepping. Gary participates in the internal motor/selection path on this revision. | S4, S6, S7; AF, DI |
| `RDY*`, `TRK0*`, `WPROT*`, `CHNG*` | Drive → CIA-A port A | Drive ready, track zero, write protection, and disk change are separate from the serial disk bitstream. | S6, S7; AF, DI |
| `INDEX*` | Drive → CIA-B `FLAG*` | Index pulse reaches the CIA-B edge-sensitive input. | S6, S7; AF, DI |
| `DKRD*`, `DKWD*`, `DKWE*` | Drive → Paula read-data input; Paula → Gary/drive write-data and write-enable paths | Paula serializes and deserializes disk data. Gary gates write-side signals on the Rev. 6A/7 drawing. | S4, S7; P, DI |
| Direction/quadrature contacts | Mouse or joystick connector → multiplexer → Denise `M0H/M0V`, `M1H/M1V` | Denise counts mouse quadrature and exposes movement or joystick state through `JOY0DAT`/`JOY1DAT`. | S5, H8 |
| `FIRE0*`, `FIRE1*` | Controller pin 6 → CIA-A `PA6/PA7` | First mouse/joystick button is sampled by CIA-A, separate from Denise movement counters. | S5, S6; AF, H8 |
| `POT0X/Y`, `POT1X/Y` | Controller pins 5/9 ↔ Paula | Proportional inputs and additional mouse buttons use Paula's potentiometer circuitry. | S4, S5; P, H8 |
| `LPEN*` | Selected controller/light-pen trigger → Agnus | Optional light-pen trigger latches beam position; a board jumper selects the source. | S2, S6; P, H8 |
| `KBDATA`, `KBCLOCK` | Keyboard controller ↔ CIA-A `SP/CNT` | Synchronous keyboard data/clock and handshake reach the CIA serial circuit. Keyboard reset uses the separate path above. | S6, AF, H |
| Parallel data and handshake | CIA-A port B/`PC*` ↔ parallel connector; CIA-B port A ↔ control/status | Eight data pins and strobe/acknowledge are distinct from select, busy, and paper-out signals. | S6, AF |
| `TXD`, `RXD`; RS-232 handshake | Paula UART ↔ line drivers/receiver ↔ serial connector; CIA-B port A ↔ control lines | Paula owns serial data; CIA-B drives or senses modem-control pins. | S4, S6; P, AF |
| `LEFT`, `RIGHT`; `LED*` | Paula → analog audio filters/output; CIA-A `PA1` → filter/LED control | Paula's output is analog after the chip. CIA-A's LED line also selects the board audio-filter path. | S4, S6; P, AF |
| RGB, burst, blanking, sync | Denise plus Agnus sync → video hybrid/connector | Denise supplies color data while Agnus supplies beam timing; board video circuitry combines them for output. | S2, S5; P |

## Interpretation for a machine coordinator

The coordinator needs distinct paths for **requests** (`DMAL`, interrupt inputs), **granted transfers** (RAM address, data direction, `RGA` destination), **sampled inputs** (CIA ports, Denise movement, Paula `POT`, disk read data), and **continuous timing** (clocks, sync, TOD). A pending DMA request is not itself a completed word transfer. The [audio state-machine description](../Reference/Hardware%20Reference%20Manual/05%20-%20Chapter%205%20-%20Audio%20Hardware.md#state-machine-signal-summary) names per-channel requests such as `AUDxDR` and `AUDxDSR`, while the board drawing shows one Paula-to-Agnus `DMAL` line; those names must not become invented per-channel motherboard wires. An interrupt generated within Paula need not become a synthetic motherboard wire merely because software observes it in `INTREQ`. Shared interrupt lines can have more than one source.

These tables establish connectivity and documented effects, not a complete list of electrical connections. Exact CCK propagation, sampling edges, Gary gate delays, analog thresholds, and differences among board revisions remain unverified. The Rev. 6A/7 schematic's `BLIT*`/`BLISS*` labeling and the older Fat Agnus manual's `DBR*` description need a separate revision-aware reconciliation before assigning one emulated signal to both. The manuals document `VERTB` and refresh-time sync strobes, but the evidence above does not establish a dedicated Agnus-to-Paula vertical-blank wire. Preserve that distinction when specifying its event path. See [Interrupts](Interrupts.md), [CIA](CIA.md), [Floppy](Floppy.md), and [Game Ports](Game%20Ports.md) for behavior inside those components.

## Reference Documentation & Upstream Ground Truth

The `S` keys refer to sheets of the [Commodore A500 Rev. 6A/7 schematic, vector redraw](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf). They identify board nets, pins, and intermediate logic for this revision.

- **S2** — [processor, Gary, and Agnus (PDF page 3)](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=3): bus, DMA, clock, sync, and CPU interrupt connections.
- **S4** — [Paula, Gary floppy logic, and audio (PDF page 5)](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=5): DMA request, interrupts, disk data, potentiometers, and audio output.
- **S5** — [Denise, game ports, and video (PDF page 6)](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=6): movement input multiplexer, buses, sync, and video output.
- **S6** — [CIA-A/B, keyboard, parallel, and serial (PDF page 7)](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=7): port pins, interrupts, TOD input, and peripherals.
- **S7** — [floppy and reset connectors (PDF page 8)](https://www.amigawiki.org/dnl/schematics/A500_R6.pdf#page=8): drive wiring, expansion lines, and reset network.
- **P** — [Hardware Reference Manual, Appendix C](../Reference/Hardware%20Reference%20Manual/11%20-%20Appendix%20C%20-%20Custom%20Chip%20Pin%20Allocation%20List.md): Chip pin names, directions, and active-low convention.
- **F** — [A500/A2000 Technical Reference, Fat Agnus](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md): DMA request direction, memory control, RGA, and sync behavior.
- **A** — [Hardware Reference Manual, Appendix A](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Refresh-time sync strobes and listed receiving chips.
- **B** — [Hardware Reference Manual, Appendix B](../Reference/Hardware%20Reference%20Manual/10%20-%20Appendix%20B%20-%20Register%20Summary%20%28Address%20Order%29.md): Register-bus addresses with multiple listed chip recipients.
- **H7** — [Hardware Reference Manual, Chapter 7](../Reference/Hardware%20Reference%20Manual/07%20-%20Chapter%207%20-%20System%20Control%20Hardware.md): Interrupt sources, levels, and masking.
- **H8** — [Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Game-controller and disk/serial interface behavior.
- **AF** — [Hardware Reference Manual, Appendix F](../Reference/Hardware%20Reference%20Manual/14%20-%20Appendix%20F%20-%20Complex%20Interface%20Adapters.md): CIA ports, TOD, serial pins, and interrupts.
- **DI** — [Hardware Reference Manual, Appendix I](../Reference/Hardware%20Reference%20Manual/17%20-%20Appendix%20I%20-%20External%20Disk%20Connector%20Interface%20Specification.md): Drive line names and directions.
- **H** — [Hardware Reference Manual, Appendix H](../Reference/Hardware%20Reference%20Manual/16%20-%20Appendix%20H%20-%20Keyboard.md): Keyboard serial and reset lines.
