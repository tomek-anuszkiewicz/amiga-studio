---
title: "Amiga 500 Real-Time Clock (RTC)"
aliases: ["RTC", "MSM6242B", "Real-Time Clock"]
tags: ["amiga", "design", "rtc", "clock"]
category: "Design"
subsystem: "rtc"
status: "active"
created: 2026-09-06
updated: 2026-11-14
related: ["[MemoryBus.md](MemoryBus.md)", "[Configuration.md](Configuration.md)", "[SaveState.md](Application/SaveState.md)", "[GUI.md](Application/GUI.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/rtc"
---
# Amiga 500 Real-Time Clock (RTC)

A real-time clock keeps the current time and calendar date. Its battery lets it continue counting while the Amiga is switched off, so the machine can read the time again at startup.

The original Amiga 500 motherboard has no built-in RTC. An RTC could be installed with the Commodore A501 trapdoor memory expansion or a compatible third-party expansion. When present, the clock is mapped at `$DC0000–$DC003F`; [MemoryBus.md](MemoryBus.md) describes how the machine routes accesses to it. [Configuration.md](Configuration.md) identifies which emulator presets include a clock.

## Host time and battery data

The emulator uses UTC seconds supplied by the host, plus a signed battery-backed
offset. Time follows supplied host UTC regardless of emulated CCKs, playback
speed or emulation pauses. The native and browser frontends supply current UTC
before the first pass of each GUI update. A headless caller supplies UTC
explicitly; the core performs
no wall-clock I/O. Between supplied samples, reads use the latest supplied second.

Guest calendar writes and the 30-second adjustment change the offset. HOLD
freezes readable calendar latches while time continues; releasing HOLD refreshes
them. STOP freezes the clock itself and excludes the stopped host interval when
released. The existing odd-byte decoding, BCD digits, weekday latch, 12/24-hour
encoding and control registers remain in effect. Repeated samples of the same
second do not overwrite guest-written latches.

Only the signed offset persists as independent battery data, under a separate
frontend storage key. Startup applies it to current host UTC. RTC time, offset,
latches and controls are excluded from [snapshots](Application/SaveState.md).
Restoration leaves the live clock intact; only its hardware presence follows
the restored machine configuration. Machine resets also retain the live clock.

## References

- [A500/A2000 Technical Reference Manual, Section 7.1](../Reference/A500%20A2000%20Technical%20Reference%20Manual/13%20-%20Section%207.1%20Clock%20calendar%20registers.md): Identifies the optional A500 clock in the A501 expansion and its `$DC0000` address.
- [`crates/rtc`](../../../crates/rtc): Contains the emulator's RTC implementation.
