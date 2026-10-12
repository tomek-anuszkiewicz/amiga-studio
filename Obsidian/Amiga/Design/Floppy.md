---
title: "Amiga 500 Floppy Subsystem"
aliases: ["Floppy", "Floppy Drive"]
tags: ["amiga", "design", "floppy", "disk"]
category: "Design"
subsystem: "floppy"
status: "active"
created: 2026-09-06
updated: 2026-10-12
related: ["[CIA.md](CIA.md)", "[Paula.md](Paula.md)", "[Agnus.md](Agnus.md)", "[DMA.md](DMA.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/floppy"
---

# Amiga 500 Floppy Subsystem

A floppy drive reads and writes removable magnetic disks. In the emulator, the [floppy crate](../../../crates/floppy/src/floppy.rs) represents the drives and integrates their behavior with disk-related registers owned by several chips. It does not duplicate those registers.

[CIA-B](CIA.md) provides drive control, [CIA-A](CIA.md) receives drive status, and [Paula](Paula.md) handles disk data and control registers. [Agnus](Agnus.md) owns disk DMA addresses and bus arbitration. Register accesses and signals that cross component boundaries are routed through the [memory bus](MemoryBus.md) and [machine loop](Machine%20Loop.md); the floppy component does not directly mutate peer chips.

## From a disk track to Chip RAM

The drive supplies a serial stream of magnetic transitions from the selected
track. In MFM encoding, each data bit is accompanied by a clock bit chosen
from the preceding data bit, so the raw track stream is larger than the sector
data it represents. The disk circuitry in Paula recovers the timed bitstream,
groups it into words, and can recognize a chosen synchronization pattern. In the usual
read path, the hardware transfers the **raw MFM-encoded words** into a Chip
RAM buffer. It does not turn the stream into AmigaDOS sectors. Software on
the CPU interprets the buffered encoding, locates sectors, and checks their
contents after the transfer; software may use the Blitter to help process
the encoded data.

For each eligible disk DMA transfer, Paula supplies a ready word and
[Agnus](Agnus.md) grants a bus opportunity, generates the Chip RAM address,
and advances the transfer position after the word is written. The configured
word count ends the block and can raise a completion interrupt. A matching
sync pattern can separately signal the CPU and, when synchronization is
enabled, determine when read DMA begins. For writing a track, the direction
reverses: Agnus fetches raw encoded words from Chip RAM and Paula sends them
to the drive as a serial stream. [DMA](DMA.md) describes the common grant and
transfer path; [CIA](CIA.md) describes the drive-control and status pins.

## References

- [CIA.md](CIA.md): Drive control and status register ownership.
- [Paula.md](Paula.md): Disk data and control register ownership.
- [Agnus.md](Agnus.md): Disk DMA address generation and arbitration.
- [MemoryBus.md](MemoryBus.md) and [Machine Loop.md](Machine%20Loop.md): Register access and signal routing.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Describes drive controls, raw MFM, synchronization, bidirectional disk DMA, and completion.
- [Floppy implementation](../../../crates/floppy/src/floppy.rs): Drive state and integration behavior.
