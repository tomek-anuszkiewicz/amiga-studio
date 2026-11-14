---
title: "Paula Audio Module"
aliases: ["Audio", "Paula Audio"]
tags: ["amiga", "design", "audio", "paula"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-19
updated: 2026-11-14
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

[`audio.rs`](../../../crates/paula/src/audio.rs) is part of Paula and implements its four audio channels. It turns incoming sample data into timed, volume-controlled stereo output and buffers the samples for playback.

This document is a detailed chapter of [Paula](Paula.md). The `paula::audio` module and its component tests belong to the `paula` crate.

## Playback and DMA cycle

Each of Paula's four channels plays a stream of signed eight-bit samples from
Chip RAM. At the start of a block, the channel takes a starting location and
length for that block. It requests a 16-bit word when it needs more sample
data. [Agnus](Agnus.md) grants an audio DMA opportunity, generates the memory
address, and delivers the word over the shared bus; Paula holds the word and
plays its high and low sample bytes in succession. A period counter sets the
time between samples, and volume control scales each output value. Two
channels feed the left audio output and two feed the right.

At the start of a block, the hardware copies its location and length into
active playback state and signals the CPU. Software can then supply the
location and length of the next block while the current one is still playing.
When the active block ends, the channel restarts using the configured values:
it repeats the same block if they were not changed, or continues with the next
one without a gap. Channels can also use their data to
modulate the following channel's volume or period instead of producing an
independent sound. The [machine loop](Machine%20Loop.md) carries restart and
interrupt signals between Paula, Agnus, and the CPU; [DMA](DMA.md) explains
the shared-bus transfer path.

Location-register writes enter [Agnus's propagation queue](Agnus.md#register-propagation); the audio engine exposes no custom-bus pointer setter. Its existing serializable playback bookkeeping remains separate from the hardware address generator. [Paula](Paula.md#register-propagation) queues the audio length, period, volume and data writes before updating this engine.

## Reconstructed channel contract

The original [HRM audio-state explanation](https://www.amigascene.nl/AmigaDevDocs/Hardware_Manual/Hardware_Manual_guide/node00F2.html)
supplements the local conversion, whose Figure 5-8 is a missing-diagram notice.
The channel has idle, first-word, second-word, high-byte and low-byte states.
DMA startup discards its first supplied word; the next word starts output.
Requests persist until AUDxDAT commitment. Length changes affect the next reload;
period changes affect the next sample boundary. Zero length/period use wrapping
16-bit counters. Holding data and the current output word are separate, so a
fetch cannot replace the low byte already scheduled for output. Missing fetches
repeat the held word. An uncleared manual-mode interrupt stops playback at the
word boundary; it does not discard a currently playing high or low byte.

Volume attachment acts at the high-byte transition and period attachment at the
low-byte transition. Both enabled use both transitions; attached sources are
muted, including channel 3 which has no following destination. Agnus exclusively
owns location and active memory pointers. Request/restart and interrupt state
are separate; a restart does not itself authorize a coordinator pointer write.
Stereo routing remains channels 1/2 left and 0/3 right. Channel clocks, DMA and
interrupts retain this contract independently of output sampling. Exact hardware
transition edge calibration remains outside this functional contract.

## BLEP output and host consumption

The output renderer produces 44,100 signed 16-bit stereo sample pairs per second
of emulated time. It records changes in the volume-scaled channel outputs at
their CCK boundary, including attachment muting and committed manual data writes.
It combines simultaneous changes on each stereo side before retaining their
time and amplitude. Channel products retain their precision until PCM conversion.
Two full-volume channels use the signed 16-bit range; overshoot clips at that
range after filtering and mixing.

The [BLEP generator](../../../tools/blep_generator/README.md) supplies static A500
correction tables for PAL and NTSC, with the LED filter enabled or disabled.
Each table spans 2,048 CCKs at its corresponding clock rate. A PCM sample is the
current stereo level minus the sum of recent steps multiplied by their table
corrections. Integer phase accumulation determines the output instants without
long-term drift; linear interpolation evaluates fractional table positions.
The renderer retains enough history for a change every CCK, merges changes at
the same boundary and expires steps only after their correction ends.

The machine derives filter selection from the effective active-low CIA-A PA1
output. Selection applies to the complete retained history at the next output
sample. This table-selection model does not calibrate the analog transient when
the physical filter switches. Audio counters advance at their existing phase;
the renderer emits samples for each elapsed CCK interval before recording the
channel transitions at its ending boundary. Register commits at that boundary
can merge with those transitions without moving the channel or bus timing.

The rolling PCM ring holds 1,024 stereo pairs, about 23.2 ms. This covers the
longest PAL field, including interlaced field alternation, between calls to
`step_frame`. If the consumer falls behind, the ring replaces the oldest unread
pair. Reading consumes pairs in order and never changes emulated time. The
machine exposes buffer draining for a future host adapter; it neither opens an
audio device nor discards samples automatically at video boundaries.

Snapshots retain the clock standard, output phase, current levels, pending BLEP
history, filter selection and unread PCM ring. Restoration continues the same
stream without replaying steps. Reset clears synthesis history and unread output
while retaining the configured PAL/NTSC clock. Host playback remains deferred.

## Reference documentation and source

- [`audio.rs`](../../../crates/paula/src/audio.rs) is the living implementation of the audio engine.
- [`audio_output.rs`](../../../crates/paula/src/audio_output.rs) owns timed BLEP synthesis and the rolling PCM ring.
- [BLEP generator](../../../tools/blep_generator/README.md): Produces the runtime tables; numerical regression against these tables is not original-hardware validation.
- [Amiga Hardware Reference Manual, Chapter 5](../Reference/Hardware%20Reference%20Manual/05%20-%20Chapter%205%20-%20Audio%20Hardware.md): Explains audio block playback, sample timing, DMA requests, and channel modulation.
- [Paula.md](Paula.md) describes Paula's role in the machine.

- [Original HRM audio-state explanation](https://www.amigascene.nl/AmigaDevDocs/Hardware_Manual/Hardware_Manual_guide/node00F2.html): Startup, manual playback and attachment transitions omitted from the local prose.
