---
title: "Agnus DMA Scheduling"
aliases: ["DMA", "DMA Scheduler", "DMA Arbitration"]
tags: ["amiga", "design", "dma", "agnus", "scheduling"]
category: "Design"
subsystem: "dma"
status: "draft"
created: 2026-09-19
updated: 2026-10-12
related: ["[Agnus.md](Agnus.md)", "[MemoryBus.md](MemoryBus.md)", "[Cross-Chip Signals and Action Dispatch Catalog.md](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)"]
tracked_paths:
  - "crates/dma"
---
# Agnus DMA Scheduling

DMA is part of [Agnus](Agnus.md). It schedules access to the Chip RAM bus shared
with the CPU, deciding which requester may use each available memory slot.

For a DMA transfer, Agnus drives the memory address on the Chip RAM address bus
and the destination register address on its internal register address (RGA) bus.
DMA selection signals and register strobes travel over the modeled electrical
connections. The selected chip recognizes its signal and latches or supplies the
word on the shared data bus. [MemoryBus.md](MemoryBus.md) describes CPU contention;
the [cross-chip signal catalog](Cross-Chip%20Signals%20and%20Action%20Dispatch%20Catalog.md)
describes the connections between chips.

## From an opportunity to a transfer

Some DMA activity starts with a request from the device that needs data, such as
an audio or disk channel. Other opportunities follow the beam position and an
enabled display function: bitplane and sprite fetching have scheduled places in
the line, while memory refresh occurs independently of a device data request.
An allocated opportunity does not by itself mean that a word is transferred.
If the relevant channel has no work, its opportunity can be used by another
eligible bus user.

When a channel is granted a memory cycle, Agnus selects its Chip RAM address and
the destination on the shared custom-chip bus. For a read, Chip RAM supplies a
word and the selected display or audio circuit latches it. For a disk write or
other memory-bound operation, the data flows in the opposite direction under
the same Agnus-controlled address and bus grant. The transfer then advances
the channel's position in its data stream; completion or a need for more data
is signaled by the responsible component. Refresh is different: it occupies a
memory opportunity but does not deliver a word to a peripheral.

## Reference Documentation & Upstream Ground Truth

- [Fat Agnus technical reference](../Reference/A500%20A2000%20Technical%20Reference%20Manual/11%20-%20Section%206%20-%20Fat%20Agnus%20Chip.md): Describes Agnus address generation and DMA signals.
- [Amiga Hardware Reference Manual, Chapter 6](../Reference/Hardware%20Reference%20Manual/06%20-%20Chapter%206%20-%20Blitter%20Hardware.md): Explains allocated memory opportunities and bus priority.
- [DMA scheduler source](../../../crates/dma/src/dma.rs): Implements the current arbitration model.
