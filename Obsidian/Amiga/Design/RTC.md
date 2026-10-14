---
title: "Amiga 500 Real-Time Clock (RTC)"
aliases: ["RTC", "MSM6242B", "Real-Time Clock"]
tags: ["amiga", "design", "rtc", "clock"]
category: "Design"
subsystem: "rtc"
status: "active"
created: 2026-09-06
updated: 2026-10-12
related: ["[MemoryBus.md](MemoryBus.md)", "[Configuration.md](Configuration.md)"]
tracked_paths:
  - "crates/rtc"
---
# Amiga 500 Real-Time Clock (RTC)

A real-time clock keeps the current time and calendar date. Its battery lets it continue counting while the Amiga is switched off, so the machine can read the time again at startup.

The original Amiga 500 motherboard has no built-in RTC. An RTC could be installed with the Commodore A501 trapdoor memory expansion or a compatible third-party expansion. When present, the clock is mapped at `$DC0000–$DC003F`; [MemoryBus.md](MemoryBus.md) describes how the machine routes accesses to it. [Configuration.md](Configuration.md) identifies which emulator presets include a clock.

## References

- [A500/A2000 Technical Reference Manual, Section 7.1](../Reference/A500%20A2000%20Technical%20Reference%20Manual/13%20-%20Section%207.1%20Clock%20calendar%20registers.md): Identifies the optional A500 clock in the A501 expansion and its `$DC0000` address.
- [`crates/rtc`](../../../crates/rtc): Contains the emulator's RTC implementation.
