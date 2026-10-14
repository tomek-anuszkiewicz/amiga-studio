---
title: "Paula Audio Module"
aliases: ["Audio", "Paula Audio"]
tags: ["amiga", "design", "audio", "paula"]
category: "Design"
subsystem: "audio"
status: "active"
created: 2026-09-19
updated: 2026-10-12
related: ["[Paula.md](Paula.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/audio"
---
# Paula Audio Module

[`audio.rs`](../../../crates/audio/src/audio.rs) is part of Paula and implements its four audio channels. It turns incoming sample data into timed, volume-controlled stereo output and buffers the samples for playback.

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

## Reference documentation and source

- [`audio.rs`](../../../crates/audio/src/audio.rs) is the living implementation of the audio engine.
- [Amiga Hardware Reference Manual, Chapter 5](../Reference/Hardware%20Reference%20Manual/05%20-%20Chapter%205%20-%20Audio%20Hardware.md): Explains audio block playback, sample timing, DMA requests, and channel modulation.
- [Paula.md](Paula.md) describes Paula's role in the machine.
