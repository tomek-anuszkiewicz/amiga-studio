---
title: "Agnus Models and Component Structure"
aliases: ["Agnus", "MOS 8370", "MOS 8371"]
tags: ["amiga", "design", "agnus", "dma", "beam"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-06
updated: 2026-11-12
related: ["[Copper.md](Copper.md)", "[Blitter.md](Blitter.md)", "[DMA.md](DMA.md)", "[Denise.md](Denise.md)"]
tracked_paths:
  - "crates/agnus"
---
# Agnus Models and Component Structure

The emulator implements two OCS Agnus models:

| Chip model | Video standard | Chip RAM limit | Display timing |
| :--- | :--- | :--- | :--- |
| **MOS 8370** | NTSC | **512 KB** (`$000000-$07FFFF`) | 262 lines; alternating 227 and 228 CCK scanlines (`LOL`) |
| **MOS 8371** | PAL | **512 KB** (`$000000-$07FFFF`) | 312 lines; fixed 227 CCK scanlines |

Agnus keeps the beam position that provides the timebase for display and
custom-chip work. It schedules Chip RAM access for the Copper, Blitter,
bitplanes, sprites, audio, disk, and memory refresh. For DMA it generates
memory addresses and selects where data travels; the receiving chip handles
the data for its own function. [DMA](DMA.md) describes a transfer without
duplicating the channel behavior here. This role and the supported models
are separate questions: the table above describes the current emulator
configuration, while the component notes describe hardware behavior.

OCS Agnus stores DMA locations as 18-bit **word addresses**: 256K words in
the lower 512 KB of Chip RAM. The paired write-only location registers retain
three bits from the high word and 15 bits from the low word; the low byte-address
bit is always zero. In the emulator these locations are represented as even
byte addresses from `$000000` through `$07FFFE`. Writing either register half
discards its unused bits, and subsequent pointer arithmetic wraps within this
range. The shared [`MASK_OCS_DMA_ADDRESS`](../../../crates/config/src/chip_ram.rs)
also bounds Chip RAM DMA accesses. This rule applies to Agnus DMA pointers,
not to every custom-chip register; in Blitter line mode, `BLTAPT` is used as
an error accumulator after loading.

Agnus uses the vertical fields of `DIWSTRT` and `DIWSTOP` to allow bitplane
DMA only on lines inside the display window. Its scheduler uses `DDFSTRT` and
`DDFSTOP` for the horizontal fetch interval; [Denise](Denise.md) uses the
horizontal DIW fields to clip the pixels produced from those words.

## Component structure

The [`agnus` crate](../../../crates/agnus/src/agnus.rs) owns the beam counters and its DMA, Copper and Blitter modules. Their detailed specifications remain separate chapters of this chip's design:

- [`dma`](../../../crates/agnus/src/dma.rs) schedules DMA slots; `dma_audio.rs`, `dma_disk.rs` and `dma_sprite.rs` implement the granted transfers and request capture. See [DMA.md](DMA.md).
- [`blitter`](../../../crates/agnus/src/blitter.rs) performs block and line operations. See [Blitter.md](Blitter.md).
- [`copper`](../../../crates/agnus/src/copper.rs) executes display lists. See [Copper.md](Copper.md).

Source files remain flat under `agnus/src/`. Supporting files carry the subsystem prefix, such as `blitter_area.rs`, `blitter_line_engine.rs` and `copper_control.rs`. External consumers use the owning chip's namespaces (`agnus::blitter`, `agnus::copper`, `agnus::dma`). Component tests live in the Agnus test suite.

`Agnus.dma` is the single owner of Agnus's active `DMACON`, `BPLCON0`, `DDFSTRT` and `DDFSTOP` values. Register reads, beam control and scheduling use those same fields; committing a write updates each register once. Denise and Paula retain their own hardware latches and delivery phases. The raw display-window words remain in Agnus, with their decoded vertical bounds in the scheduler. [Snapshot version 4](Application/SaveState.md#1-state-ownership-and-persistence) removes the four former outer register copies.

## Register propagation

Custom bus writes enter concrete queued setters in [registers.rs](../../../crates/agnus/src/registers.rs), [blitter_registers.rs](../../../crates/agnus/src/blitter_registers.rs) and [copper_registers.rs](../../../crates/agnus/src/copper_registers.rs). These modules share the Agnus queue; the child modules do not own separate propagation queues. Register reads remain in `registers.rs`, while [register_apply.rs](../../../crates/agnus/src/register_apply.rs) applies DMA set/clear bits, pointer masks and child Copper/Blitter changes only when due. Pending configuration leaves active DMA requests under the existing scheduler contract; commitment changes enables before later engine steps. Autonomous pointer reloads and increments remain engine operations. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) owns the shared propagation contract and provisional delays. The 64-entry queue exceeds two accepted CPU/Copper writes per CCK over the maximum four-CCK delay (eight entries); unsupported bursts are rejected rather than applied early. All matured events are applied and returned without an eight-event truncation.

Committed VPOSW writes update V8 and LOF; VHPOSW updates V7-V0 and the
horizontal counter, preserving the other vertical bit. These follow HRM 7,
Table 7-5 and the [Appendix A register definitions](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md).
COPINS is a dummy instruction-fetch identifier, not a CPU instruction injection
port. REFPTR is a DRAM test-purpose refresh address generator; this model has
no charge/row-decay simulation, so its accepted write has no observable memory
effect. Both retain the established register delivery without a fabricated
software-visible operation.

## Motherboard transport boundary

The serializable `AgnusPins` state in [pins.rs](../../../crates/agnus/src/pins.rs)
is exposed as `agnus.pins`. The routed DMAL level waits in
`pins.incoming_dmal_high` until Agnus samples inputs at the next input phase. It retains the active-low INT3 completion output
and horizontal/vertical timing outputs as well as DMAL input capture and timed
RGA output; DMA engines, beam counters, registers and pending bus words
remain in the coordinator.

The [machine input phase](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
drives `pins.dmal_input_high` from Paula's captured active-high output before Agnus
steps. Agnus decodes the request frame, retains channel requests and schedules
eligible transfers; capturing the line is not a bus grant.
`pending_dma_write` holds an already granted audio/sprite/disk RGA/data word until
MemoryBus accepts all recipients at the common post-custom-chip issue phase.
The existing pending bitplane and Copper outputs retain their established
routes. These outputs serialize and survive receiver refusal; reset cancels
them. [DMA](DMA.md) owns address, pointer/count and request completion contracts.

[The beam engine](../../../crates/agnus/src/beam.rs) uses 312/313 PAL or
262/263 NTSC lines according to LOF. BPLCON0 LACE toggles LOF at field completion;
without LACE, the current field length stays selected, including a software
VPOSW choice. PAL lines are 227 CCK; NTSC alternates 227/228 CCK continuously
across field boundaries. This follows HRM 2, Vertical Beam Position. The CPU
readout retains its existing five-CCK lead and one-CCK vertical settling model,
using the active field length when it wraps. Half-line/sub-CCK calibration is
provisional. Image weaving and receiver-local field interpretation belong to
the [Denise](Denise.md) display pipeline.

Agnus retains the current beam position directly in `hpos` and `vpos`, with
`lof` and `lol` recording field and line state. Interlace enable remains in
`bplcon0`, and `model` selects the video standard. Diagnostics read this state
without a separate position structure; Denise receives synchronization events.
The `pins.rga_strobe` output supplies refresh-time STREQU/STRVBL/STRHOR and
the optional STRLONG address to the motherboard capture phase. Both Denise and
Paula receive the complete address set. [Paula](Paula.md#rga-synchronization-and-vertb)
derives VERTB locally; Agnus has no separate vertical-blank interrupt event.
First and second
refresh slots use modeled HPOS 1 and 3. The current classification uses NTSC
10 equalization/21 blank lines and PAL 8 or 9 equalization/26 blank lines, as
in the alternate reference Agnus model; exact original-chip calibration remains
open. No peer pointer or Denise return request is introduced. Transport checks alone do not certify the reconstructed engine algorithms.

The retained Blitter engine supplies an active-low INT3 pulse through Agnus;
the motherboard captures it and delivers Paula INT3 before the next Paula step.
The current HSYNC/VSYNC source projection uses one low CCK at the end of a
line/frame, followed by a rising boundary edge for CIA TOD. These are explicit
edge markers for the preceding engine events, not reconstructed video sync pulse
widths. Original timing, external synchronization and the complete waveform
generator remain reconstruction/calibration work. The coordinator does not
calculate a TOD tick directly from the beam counters.

## Reference documentation and source

- [`AgnusModel`](../../../crates/config/src/config.rs) defines the two selectable chip models.
- [`Agnus`](../../../crates/agnus/src/agnus.rs) owns the DMA scheduler, Blitter, and Copper instances.
- [Beam counters](../../../crates/agnus/src/beam.rs): Field/line progression and committed write fields.
- [Appendix A register definitions](../Reference/Hardware%20Reference%20Manual/09%20-%20Appendix%20A%20-%20Register%20Summary%20%28Alphabetical%29.md): Beam writes and dummy/test registers.
- [A500/A2000 Technical Reference Manual, Fat Agnus](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md): Explains the beam timebase, DMA controllers, and address generation.
- [DMA.md](DMA.md), [Blitter.md](Blitter.md), and [Copper.md](Copper.md) own the component specifications.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Separates hardware register meaning, configured delivery and pending effects.
- [`agnus/pins.rs`](../../../crates/agnus/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
