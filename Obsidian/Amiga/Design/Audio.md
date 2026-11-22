---
title: "Paula Audio Module"
aliases: ["Audio", "Paula Audio"]
tags: ["amiga", "design", "audio", "paula"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-19
updated: 2026-11-16
related: ["[Paula.md](Paula.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/paula/src/audio.rs"
  - "crates/paula/src/audio_channel.rs"
  - "crates/paula/src/audio_output.rs"
  - "crates/paula/src/audio_tables.rs"
  - "crates/paula/tests/test_audio_channels.rs"
  - "crates/paula/tests/test_audio_output.rs"
  - "crates/machine_loop/tests/test_audio_output.rs"
---
# Paula Audio Module

Paula's audio module implements its four audio channels. It takes incoming
sample data, times and scales the stereo output, and buffers the resulting
samples for playback. [`audio.rs`](../../../crates/paula/src/audio.rs)


## Playback and DMA cycle

Each of Paula's four channels plays signed eight-bit samples from Chip RAM.
At the start of a block, it takes the block's starting location and length.
The channel requests a 16-bit word when it needs more sample data. Agnus
grants an audio DMA opportunity, generates the memory address, and delivers
the word over the shared bus. Paula holds the word and plays its high and low
sample bytes in succession. A period counter sets the time between samples,
and volume control scales each output value. Two channels feed the left output
and two feed the right. [Agnus](Agnus.md)

At the start of a block, the hardware copies its location and length into
active playback state and signals the CPU. Software can then supply the next
block's location and length while the current block plays. When the block
ends, the channel restarts with the configured location and length. It repeats
the same block if both values stay the same. If the configured values change,
the channel continues with the next block without a gap. A channel can also
use its data to modulate the following channel's volume or period in place of
generating its own sound. The machine loop carries the restart and interrupt
signals between Paula, Agnus, and the CPU. DMA supplies the shared-bus transfer path.
[machine loop](Machine%20Loop.md), [DMA](DMA.md)

Location-register writes enter Agnus's propagation queue. The audio engine has
no custom-bus pointer setter, and its serializable playback state stays separate
from the hardware address generator. [Agnus's propagation queue](Agnus.md#register-propagation)

Paula queues writes to the audio length, period, volume and data before they
update the audio engine. [Paula](Paula.md#register-propagation)

## Reconstructed channel contract

The original Hardware Reference Manual's audio-state explanation supplements
the local conversion, which shows a missing-diagram notice for Figure 5-8.
[HRM audio-state explanation](https://www.amigascene.nl/AmigaDevDocs/Hardware_Manual/Hardware_Manual_guide/node00F2.html)

The channel has idle, first-word, second-word, high-byte and low-byte states.
At DMA startup, the channel discards the first supplied word and starts output
with the next one. A request persists until AUDxDAT commits. Length changes
take effect at the next reload, and period changes take effect at the next
sample boundary. Zero length and zero period use wrapping 16-bit counters.
The channel holds fetched data separately from the current output word so it
still has the scheduled low byte when that byte is due. If a fetch is missed,
the channel repeats the held word. An uncleared manual-mode interrupt stops
playback at a word boundary after the current high or low byte finishes.

Volume attachment acts at the high-byte transition and period attachment at
the low-byte transition. When both are enabled, the channel acts at both
transitions. Attached sources are muted, including channel 3, which has no
following destination. Agnus alone owns the location and active memory
pointers. The audio engine keeps request and restart state separate from
interrupt state. A restart by itself does not authorize the coordinator to
write a pointer. Channels 1/2 feed the left output and channels 0/3 feed the
right. Channel clocks, DMA and interrupts follow these rules independently of
output sampling. Calibration of the exact hardware transition edges falls
outside this functional contract.

## BLEP output and host consumption

The output renderer produces 44,100 signed 16-bit stereo sample pairs per second
of emulated time. It records changes to the volume-scaled channel output at the
CCK boundary, including attachment muting and committed manual data writes.
For each stereo side, it combines simultaneous changes before recording their
time and amplitude. It keeps the full precision of each channel product until
PCM conversion. Two full-volume channels use the signed 16-bit range. Overshoot
clips to that range after filtering and mixing.

The BLEP generator supplies static A500 correction tables for PAL and NTSC,
with the LED filter enabled or disabled. Each table spans 2,048 CCKs at its
corresponding clock rate. A PCM sample is the current stereo level minus
the sum of recent steps multiplied by their table corrections. Integer phase
accumulation determines when to emit a sample without long-term drift. Linear
interpolation evaluates fractional table positions. The renderer keeps enough
history for a change every CCK, merges changes at the same boundary and
discards steps only after their correction ends. [BLEP generator](../../../tools/blep_generator/README.md)

The machine uses the effective active-low CIA-A PA1 output to select the
filter. When the renderer emits its next output sample, it uses the selected
table for the entire retained history. The table-selection model leaves
calibration of the analog transient during a physical filter switch outside
its scope. Audio counters advance at their existing phase. For each elapsed
CCK interval, the renderer emits its samples before recording channel
transitions at the interval's end. Register commits at that same boundary can
merge with the transitions while preserving channel and bus timing.

The rolling PCM ring holds 1,024 stereo pairs, about 23.2 ms. This covers the
longest PAL field between calls to `step_frame`, including interlaced field
alternation. If the consumer falls behind, the ring replaces the oldest unread
pair. A read consumes pairs in order without advancing emulated time. The
machine exposes an operation to drain the buffer for a future host adapter.
It leaves audio device access to the host and keeps samples across video
boundaries.

Snapshots retain the clock standard, output phase, current levels, pending
BLEP history, filter selection and unread PCM ring. After restoration, the
stream continues without replaying steps. Reset clears synthesis history and
unread output and retains the configured PAL/NTSC clock. Host playback remains
deferred.

## Reference documentation and source

- [`audio.rs`](../../../crates/paula/src/audio.rs) is the living implementation of the audio engine.
- [`audio_output.rs`](../../../crates/paula/src/audio_output.rs) owns timed BLEP synthesis and the rolling PCM ring.
- [BLEP generator](../../../tools/blep_generator/README.md): Produces the runtime tables; numerical regression against these tables is not original-hardware validation.
- [Amiga Hardware Reference Manual, Chapter 5](../Reference/Hardware%20Reference%20Manual/05%20-%20Chapter%205%20-%20Audio%20Hardware.md): Explains audio block playback, sample timing, DMA requests, and channel modulation.
- [Paula.md](Paula.md) describes Paula's role in the machine.

- [Original HRM audio-state explanation](https://www.amigascene.nl/AmigaDevDocs/Hardware_Manual/Hardware_Manual_guide/node00F2.html): Startup, manual playback and attachment transitions omitted from the local prose.
