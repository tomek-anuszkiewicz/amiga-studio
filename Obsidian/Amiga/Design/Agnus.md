---
title: "Agnus Models and Component Structure"
aliases: ["Agnus", "MOS 8370", "MOS 8371"]
tags: ["amiga", "design", "agnus", "dma", "beam"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-06
updated: 2026-11-16
related: ["[Copper.md](Copper.md)", "[Blitter.md](Blitter.md)", "[DMA.md](DMA.md)", "[Denise.md](Denise.md)"]
tracked_paths:
  - "crates/agnus"
---
# Agnus Models and Component Structure

The emulator implements two OCS Agnus models. The selected model determines
the video standard and display timing. Both models have the same Chip RAM
limit.

| Chip model | Video standard | Chip RAM limit | Display timing |
| :--- | :--- | :--- | :--- |
| **MOS 8370** | NTSC | **512 KB** (`$000000-$07FFFF`) | 262 lines; alternating 227 and 228 CCK scanlines (`LOL`) |
| **MOS 8371** | PAL | **512 KB** (`$000000-$07FFFF`) | 312 lines; fixed 227 CCK scanlines |

Agnus tracks the beam position and uses it to time display and custom-chip
work. It schedules access to Chip RAM for the Copper, Blitter, bitplanes,
sprites, audio, disk, and memory refresh. During DMA, Agnus generates the
memory address and selects the data's destination. The receiving chip then
uses the data for its own function. This section and the component notes
describe hardware behavior. The shared transfer path is described in [DMA](DMA.md).

OCS Agnus stores DMA locations as 18-bit **word addresses**, covering 256K
words in the lower 512 KB of Chip RAM. Each pair of write-only location
registers retains three bits from the high word and 15 bits from the low word.
The low byte-address bit is always zero. The emulator represents these
locations as even byte addresses from `$000000` through `$07FFFE`. A write to
either half discards the unused bits, and subsequent pointer arithmetic wraps
within this range. The same mask bounds Chip RAM DMA accesses. The limit
applies to Agnus DMA pointers. In Blitter line mode, `BLTAPT` becomes an error
accumulator after loading. [`MASK_OCS_DMA_ADDRESS`](../../../crates/config/src/chip_ram.rs)

Agnus uses the vertical fields of `DIWSTRT` and `DIWSTOP` to allow bitplane
DMA only on lines inside the display window. Its scheduler uses `DDFSTRT` and
`DDFSTOP` to determine when to fetch words on each line. Denise uses the
horizontal DIW fields to clip the pixels produced from those words. [Denise](Denise.md)

## Component structure

The Agnus crate owns the beam counters and contains the DMA, Copper and
Blitter modules. Each module has its own design chapter.
[`agnus` crate](../../../crates/agnus/src/agnus.rs)

- [`dma`](../../../crates/agnus/src/dma.rs) schedules DMA slots; `dma_audio.rs`, `dma_disk.rs` and `dma_sprite.rs` implement the granted transfers and request capture. See [DMA.md](DMA.md).
- [`blitter`](../../../crates/agnus/src/blitter.rs) performs block and line operations. See [Blitter.md](Blitter.md).
- [`copper`](../../../crates/agnus/src/copper.rs) executes display lists. See [Copper.md](Copper.md).

`Agnus.dma` alone owns Agnus's active `DMACON`, `BPLCON0`, `DDFSTRT` and
`DDFSTOP` values. Register reads, beam control and scheduling use the same fields.
Committing a write updates the affected register once. Denise and Paula keep
their own hardware latches and delivery phases. Agnus keeps the raw
display-window words, and the scheduler keeps the decoded vertical bounds.
The four former outer register copies were removed in [Snapshot version 4](Application/SaveState.md#1-state-ownership-and-persistence).

## Register propagation

For custom bus data writes routed to Agnus, register-specific setters add the
writes to the shared Agnus queue. Agnus owns the single register propagation
queue used by the child modules.
[registers.rs](../../../crates/agnus/src/registers.rs), [blitter_registers.rs](../../../crates/agnus/src/blitter_registers.rs), [copper_registers.rs](../../../crates/agnus/src/copper_registers.rs)

Register reads remain in `registers.rs`. When a queued write becomes due,
the write handler applies DMA set/clear bits, pointer masks and changes to
the Copper or Blitter as appropriate. Active DMA requests follow the
existing scheduling rules while a configuration write waits. Committing the
write updates enables before later engine steps. The engines themselves
reload and increment their pointers. [register_apply.rs](../../../crates/agnus/src/register_apply.rs)

MemoryBus defines the shared propagation contract and provisional delays.
The 64-entry queue has room for two accepted CPU/Copper writes per CCK over
the maximum four-CCK delay (eight entries). An attempted overflow stops the
owner before any write can be applied early. The owner applies and returns
all due events, including batches larger than eight events.
[MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips)

Committed VPOSW writes update V8 and LOF; VHPOSW updates V7-V0 and the
horizontal counter, preserving the other vertical bit. These follow HRM 7,
Table 7-5 and the [Appendix A register definitions](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md).
COPINS is a dummy identifier for instruction fetches. A CPU write to it cannot
inject an instruction. REFPTR generates refresh addresses for DRAM testing.
The model omits charge and row decay, so an accepted write has no observable
memory effect. Both registers retain the established delivery behavior, with
no additional software-visible operation.

## Motherboard transport boundary

The coordinator exposes the serializable `AgnusPins` state as `agnus.pins`.
It holds the captured DMAL input, timed RGA output, active-low INT3 completion
output and horizontal and vertical timing outputs. The routed DMAL level
waits in `pins.incoming_dmal_high` until Agnus samples its inputs at the next
input phase. The coordinator retains the DMA engines, beam counters,
registers and pending bus words. [pins.rs](../../../crates/agnus/src/pins.rs)

Before Agnus steps, the machine input phase drives `pins.dmal_input_high`
from Paula's captured active-high output. Agnus decodes the request frame
and retains each channel's request for scheduling. An eligible transfer
proceeds when the scheduler grants it a slot.
[machine input phase](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)

After a grant, `pending_dma_write` holds the audio, sprite or disk RGA/data
word until MemoryBus accepts it for every recipient in the common issue
phase after the custom chips step. Pending bitplane and Copper outputs keep
their established routes. These outputs are included in serialized state
until delivery, and reset cancels them. A receiver queue overflow stops the
owner without retrying the pending output. The address, pointer, count and
request completion rules are defined in [DMA](DMA.md).

The beam engine selects 312/313 PAL or 262/263 NTSC lines according to LOF.
With BPLCON0 LACE enabled, it toggles LOF when a field completes. With LACE
disabled, the current field length stays selected, including a software
VPOSW choice. PAL lines last 227 CCK. NTSC alternates 227/228 CCK across
field boundaries. This follows HRM 2, Vertical Beam Position. The CPU readout
keeps its existing five-CCK lead and one-CCK vertical settling model, and
uses the active field length when wrapping. Half-line/sub-CCK calibration
is provisional. [The beam engine](../../../crates/agnus/src/beam.rs)

Image weaving and interpretation of field timing at the receiver belong to
Denise's display pipeline. [Denise](Denise.md)

The limit of 226 allocatable memory cycles applies to the DMA allocation
budget. HRM 2 gives the NTSC beam-counter line lengths as alternating
227/228-clock lines and distinguishes them from the display's 227.5-clock
period. [HRM 6, Blitter Operations and System DMA](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md#blitter-operations-and-system-dma)

At refresh time, the motherboard capture phase receives STREQU/STRVBL/STRHOR
and the optional STRLONG address through `pins.rga_strobe`. It delivers the
complete address set to both Denise and Paula. Paula derives VERTB from
the received addresses. Agnus has no separate vertical-blank interrupt event.
The modeled first and second refresh slots use HPOS 1 and 3. The current
classification follows the alternate reference Agnus model, with NTSC
10 equalization/21 blank lines and PAL 8 or 9 equalization/26 blank lines.
Exact original-chip calibration remains open. The transport uses no peer
pointer or Denise return request. Transport checks by themselves cannot
establish that the reconstructed engine algorithms are correct.
[Paula](Paula.md#rga-synchronization-and-vertb)

The retained Blitter engine sends an active-low INT3 pulse through Agnus.
The motherboard captures the pulse and delivers Paula INT3 before Paula's
next step. The current HSYNC/VSYNC outputs go low for one CCK at the end of
a line/frame, then send a rising boundary edge for CIA TOD. These edges mark
the preceding engine events. Original video sync pulse widths, other timing,
external synchronization and the complete waveform generator remain work
for reconstruction and calibration. The coordinator uses the timing outputs
for CIA TOD without calculating a TOD tick directly from the beam counters.

## Reference documentation and source

- [`AgnusModel`](../../../crates/config/src/config.rs) defines the two selectable chip models.
- [`Agnus`](../../../crates/agnus/src/agnus.rs) owns the DMA scheduler, Blitter, and Copper instances.
- [Beam counters](../../../crates/agnus/src/beam.rs): Field/line progression and committed write fields.
- [Appendix A register definitions](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Beam writes and dummy/test registers.
- [A500/A2000 Technical Reference Manual, Fat Agnus](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md): Explains the beam timebase, DMA controllers, and address generation.
- [DMA.md](DMA.md), [Blitter.md](Blitter.md), and [Copper.md](Copper.md) own the component specifications.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Separates hardware register meaning, configured delivery and pending effects.
- [`agnus/pins.rs`](../../../crates/agnus/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
