---
title: "Cross-Chip Signals and Action Dispatch Catalog"
aliases: ["Cross-Chip Signals Catalog", "Action Dispatch Catalog", "Inter-Chip Signal Matrix"]
tags: ["amiga", "design", "chips", "signals", "propagation", "action-dispatch"]
category: "Design"
subsystem: "general"
status: "draft"
created: 2026-09-14
updated: 2026-10-12
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Machine Loop.md](Machine%20Loop.md)", "[Agnus.md](Agnus.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Floppy.md](Floppy.md)", "[DMA.md](DMA.md)"]
---

# Cross-Chip Signals and Action Dispatch Catalog

The Amiga 500's chips communicate through fixed motherboard buses and electrical signal lines. In the emulator, the [machine loop](Machine%20Loop.md) and [memory bus](MemoryBus.md) route these connections. Individual chips do not hold references to one another or call one another's mutating methods.

## Signal flow

[Agnus](Agnus.md) generates Chip RAM DMA addresses and drives the register address bus (RGA). The memory bus places fetched data on the shared data bus. [Denise](Denise.md) and [Paula](Paula.md) latch data when the corresponding register strobe selects them; they do not read Chip RAM directly. The machine loop also samples chip outputs and delivers their signals to the receiving components.

Signals and register effects can take time to propagate. Their timing depends on the particular connection and the modeled clock phase; this document does not yet provide a verified delay for each signal. [DMA.md](DMA.md) owns slot scheduling and arbitration, while this catalog concerns connections between components.

## Signal table to add

This catalog still needs a table of the electrical signals that cross chip boundaries. Each entry should identify the source and destination, name the line or bus, and explain what the signal causes. Timing and active level should be recorded where supported by hardware evidence.

## Reference documentation and source

- [General Architecture.md](General%20Architecture.md) describes the machine's chip and bus topology.
- [Amiga Hardware Reference Manual, Chapter 1](../Reference/Hardware%20Reference%20Manual/01%20-%20Chapter%201%20-%20Introduction.md) introduces the custom chips and shared buses.
- [`machine_loop.rs`](../../../crates/machine_loop/src/machine_loop.rs) routes modeled signals between components.
- [`memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs) routes memory and custom register accesses.
