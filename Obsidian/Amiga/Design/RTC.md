---
title: "Amiga 500 Real-Time Clock (RTC)"
aliases: ["RTC", "MSM6242B", "Real-Time Clock"]
tags: ["amiga", "design", "rtc", "clock"]
category: "Design"
subsystem: "rtc"
status: "active"
created: 2026-09-06
updated: 2026-11-16
related: ["[MemoryBus.md](MemoryBus.md)", "[Configuration.md](Configuration.md)", "[SaveState.md](Application/SaveState.md)", "[GUI.md](Application/GUI.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/rtc"
---
# Amiga 500 Real-Time Clock (RTC)

A real-time clock keeps the time and date. Its battery powers the clock while
the Amiga is off so the machine can read the current time at startup.

The original Amiga 500 motherboard has no built-in RTC. The Commodore A501
trapdoor memory expansion and compatible third-party expansions could add one.
An installed clock responds at `$DC0000–$DC003F`. The bus specification explains
how accesses reach the clock. The configuration specification lists the emulator
presets that include it.
[MemoryBus.md](MemoryBus.md), [Configuration.md](Configuration.md)

## Host time and battery data

To compute the clock time, the emulator adds a signed battery-backed offset to
the UTC seconds supplied by the host. The clock follows those samples regardless
of emulated CCKs, playback speed or pauses. Before the first pass of each GUI
update, the native or browser frontend supplies the current UTC. A headless
caller supplies UTC explicitly. The core answers clock reads using the most
recent sample and performs no wall-clock I/O.

At startup, HOLD and STOP are clear. This lets the seconds register follow the
supplied UTC before guest software initializes the controls. The technical
reference describes how `SetClock` detects an installed clock by checking whether
its seconds register ticks.
[A500/A2000 technical reference](../Reference/A500%20A2000%20Technical%20Reference%20Manual/13%20-%20Section%207.1%20Clock%20calendar%20registers.md)

Guest calendar writes and the 30-second adjustment change the offset. HOLD
keeps the values that guest software reads in the calendar latches while the
clock keeps running. Clearing HOLD refreshes those latches. STOP freezes the
clock itself. When software clears STOP, the clock excludes the host time that
elapsed while it was stopped. The clock uses odd-byte decoding, BCD digits, a
weekday latch, 12/24-hour encoding and control registers. Repeated samples of the
same second leave guest-written latches intact.

The frontend saves only the signed offset as independent battery data under a
separate storage key. At startup the emulator applies it to current host UTC.
Snapshots exclude RTC time, offset, latches and controls. When the emulator
restores a snapshot, the restored machine configuration determines whether the
machine has RTC hardware. Restoration and machine resets both retain the live
clock. [snapshots](Application/SaveState.md)

## Workbench 1.3 dates after 1999

Year registers store two BCD digits. The original Workbench 1.2 and 1.3
`SetClock` miscalculates dates after 1999. It can display 1978 even after it
detects the clock. Amiga's update replaces `C:SetClock` to correct the guest
software. Startup can still use `SetClock LOAD`.
[SetClock 34.3 update](https://www.amigaforever.com/classic/download/)

Amiga's disclosure documents the defect and identifies the affected operating
system versions.
[Amiga year 2000 disclosure](https://amiga.com/diary/developers/y2k.html)

## References

- The manual identifies the optional A500 clock in the A501 expansion and gives
  its `$DC0000` address.
  [A500/A2000 Technical Reference Manual, Section 7.1](../Reference/A500%20A2000%20Technical%20Reference%20Manual/13%20-%20Section%207.1%20Clock%20calendar%20registers.md)
- The emulator's RTC implementation is in [`crates/rtc`](../../../crates/rtc).
- Amiga explains the date defect in the original Workbench 1.2 and 1.3
  `SetClock` and describes the update.
  [Amiga year 2000 disclosure](https://amiga.com/diary/developers/y2k.html)
