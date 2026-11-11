---
title: "Agnus Models and Component Structure"
aliases: ["Agnus", "MOS 8370", "MOS 8371"]
tags: ["amiga", "design", "agnus", "dma", "beam"]
category: "Design"
subsystem: "agnus"
status: "active"
created: 2026-09-06
updated: 2026-11-11
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
range. The shared [`OCS_DMA_ADDRESS_MASK`](../../../crates/config/src/chip_ram.rs)
also bounds Chip RAM DMA accesses. This rule applies to Agnus DMA pointers,
not to every custom-chip register; in Blitter line mode, `BLTAPT` is used as
an error accumulator after loading.

Agnus uses the vertical fields of `DIWSTRT` and `DIWSTOP` to allow bitplane
DMA only on lines inside the display window. Its scheduler uses `DDFSTRT` and
`DDFSTOP` for the horizontal fetch interval; [Denise](Denise.md) uses the
horizontal DIW fields to clip the pixels produced from those words.

## Component structure

The [`agnus` crate](../../../crates/agnus/src/agnus.rs) coordinates the beam counters and uses three separate workspace crates as components:

- [`dma`](../../../crates/dma/src/dma.rs) schedules DMA slots. Its behavior is specified in [DMA.md](DMA.md).
- [`blitter`](../../../crates/blitter/src/blitter.rs) performs block and line operations. Its behavior is specified in [Blitter.md](Blitter.md).
- [`copper`](../../../crates/copper/src/copper.rs) executes display lists. Its behavior is specified in [Copper.md](Copper.md).

## Register propagation

Custom bus writes enter concrete queued setters in [registers.rs](../../../crates/agnus/src/registers.rs), [blitter_registers.rs](../../../crates/agnus/src/blitter_registers.rs) and [copper_registers.rs](../../../crates/agnus/src/copper_registers.rs). These modules share the Agnus queue; the child modules do not own separate propagation queues. Register reads remain in `registers.rs`, while [register_apply.rs](../../../crates/agnus/src/register_apply.rs) applies DMA set/clear bits, pointer masks and child Copper/Blitter changes only when due. Pending configuration leaves active DMA requests under the existing scheduler contract; commitment changes enables before later engine steps. Autonomous pointer reloads and increments remain engine operations. [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips) owns the shared propagation contract and provisional delays. The 64-entry queue exceeds two accepted CPU/Copper writes per CCK over the maximum four-CCK delay (eight entries); unsupported bursts are rejected rather than applied early. All matured events are applied and returned without an eight-event truncation.

REFPTR, beam-write and COPINS effects remain incomplete; see the
[chipset reconstruction work](../../../ROADMAP.md#chipset-reconstruction).

## Motherboard transport boundary

The serializable `AgnusPins` state in [pins.rs](../../../crates/agnus/src/pins.rs)
is exposed as `agnus.pins`. The routed DMAL level waits in
`pins.incoming_dmal_high` until Agnus samples inputs at the next input phase. It retains the active-low INT3 completion output
and horizontal/vertical timing outputs as well as DMAL input capture and timed
RGA output; DMA engines, beam counters, registers and pending bus words
remain in the coordinator.

The [machine input phase](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
drives `pins.dmal_input_high` from Paula's captured active-high output before Agnus
steps. The recipient's request-frame decoding, channel requests and scheduling
are 2.4 engine responsibilities; capturing the line is not a bus grant.
`pending_dma_write` holds an already granted audio/sprite RGA/data word until
MemoryBus accepts all recipients at the common post-custom-chip issue phase.
The existing pending bitplane and Copper outputs retain their established
routes. These outputs serialize and survive receiver refusal; reset cancels
them. [DMA](DMA.md) owns address, pointer/count and request completion contracts.

Agnus retains the current beam position directly in `hpos` and `vpos`, with
`lof` and `lol` recording field and line state. Interlace enable remains in
`bplcon0`, and `model` selects the video standard. Diagnostics read this state
without a separate position structure; Denise receives synchronization events.
The `pins.rga_strobe` output supplies refresh-time STREQU/STRVBL/STRHOR and
the optional STRLONG address to the motherboard capture phase. First and second
refresh slots use modeled HPOS 1 and 3. The current classification uses NTSC
10 equalization/21 blank lines and PAL 8 or 9 equalization/26 blank lines, as
in the alternate reference Agnus model; exact original-chip calibration remains
open. No peer pointer or Denise return request is introduced. Existing engine algorithms are not certified by these transport fields
or deterministic output tests.

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
- [A500/A2000 Technical Reference Manual, Fat Agnus](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md): Explains the beam timebase, DMA controllers, and address generation.
- [DMA.md](DMA.md), [Blitter.md](Blitter.md), and [Copper.md](Copper.md) own the component specifications.
- [Register matrix](Custom%20Chip%20Register%20Ownership%20and%20Access%20Matrix.md): Separates hardware register meaning, configured delivery and pending effects.
- [`agnus/pins.rs`](../../../crates/agnus/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
