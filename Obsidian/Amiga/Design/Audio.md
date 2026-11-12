---
title: "Paula Audio Module"
aliases: ["Audio", "Paula Audio"]
tags: ["amiga", "design", "audio", "paula"]
category: "Design"
subsystem: "paula"
status: "active"
created: 2026-09-19
updated: 2026-11-12
related: ["[Paula.md](Paula.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/paula/src/audio.rs"
  - "crates/paula/src/audio_channel.rs"
  - "crates/paula/tests/test_audio_channels.rs"
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
Digital output retains its existing normalized stereo scale, channels 1/2 left
and 0/3 right. The fixed output ring and its indices serialize together, so
restoration retains unread samples. Exact transition edge calibration and host
playback/filter electronics remain outside this functional contract.

## Reference documentation and source

- [`audio.rs`](../../../crates/paula/src/audio.rs) is the living implementation of the audio engine.
- [Amiga Hardware Reference Manual, Chapter 5](../Reference/Hardware%20Reference%20Manual/05%20-%20Chapter%205%20-%20Audio%20Hardware.md): Explains audio block playback, sample timing, DMA requests, and channel modulation.
- [Paula.md](Paula.md) describes Paula's role in the machine.

- [Original HRM audio-state explanation](https://www.amigascene.nl/AmigaDevDocs/Hardware_Manual/Hardware_Manual_guide/node00F2.html): Startup, manual playback and attachment transitions omitted from the local prose.
