---
title: "Physical Memory"
aliases: ["Amiga 500 Physical Memory", "PhysicalMemory", "Memory Bank Map"]
tags: ["amiga", "design", "physical_memory", "chip_ram", "memory_map"]
category: "Design"
subsystem: "physical_memory"
status: "active"
created: 2026-10-12
updated: 2026-11-17
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Configuration.md](Configuration.md)", "[Machine Loop.md](Machine%20Loop.md)", "[DMA.md](DMA.md)", "[RTC.md](RTC.md)", "[Platform Quirks and Invariants Catalog.md](Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/physical_memory"
---
# Physical Memory

The physical-memory component owns the Amiga 500's RAM and Kickstart ROM
buffers, the 24-bit bank table, the low-memory boot overlay, and the current
CPU memory-contention flag. It provides byte and big-endian word storage
operations. [`PhysicalMemory`](../../../crates/physical_memory/src/physical_memory.rs)

The machine loop owns the storage component and coordinates its use by the
CPU and Agnus. The motherboard router handles live CIA, RTC, and
custom-register access. Physical memory holds no direct device references.
[Motherboard router](MemoryBus.md) and [machine loop](Machine%20Loop.md)

## Installed storage and address decoding

Each configuration preset defines a 256-entry table of 64 KB banks covering
the 16 MB CPU address space. Each bank handler supplies byte and word reads
and writes, with separate operations for the debugger.
[`presets.rs`](../../../crates/physical_memory/src/presets.rs),
[configuration preset](Configuration.md), and
[`BankHandler`](../../../crates/physical_memory/src/map.rs)

Bank selection uses `(addr >> 16) & $FF`. For a CPU access, `MemoryBus`
first masks the address to 24 bits and routes device banks. For storage and
open bus, `PhysicalMemory` selects the bank handler. When a caller reaches
a CIA, RTC, or custom-chip bank directly through `PhysicalMemory`, the
handler uses the corresponding placeholder behavior and leaves the live
device untouched.

| Address | Installed behavior in the current presets |
| --- | --- |
| `$000000-$07FFFF` | 512 KB Chip RAM after overlay removal; Kickstart ROM while overlay is active. |
| `$080000-$1FFFFF` | Open bus; the current presets do not install extended Chip RAM. |
| `$200000-$5FFFFF` | 4 MB Fast RAM only in `ExpandedPowerUser`; open bus otherwise. No Chip RAM contention check. |
| `$600000-$9FFFFF` | Open bus in the current presets. |
| `$A00000-$BFFFFF` | CIA classification for direct physical-memory access; `MemoryBus` decodes the live CIA byte lanes. |
| `$C00000-$C7FFFF` | 512 KB Slow RAM in `Standard1Mb` and `ExpandedPowerUser`; open bus in `Bare512k`. |
| `$C80000-$DBFFFF` | Open bus. |
| `$DC0000-$DCFFFF` | RTC classification when installed, otherwise open bus. `MemoryBus` routes only `$DC0000-$DC003F` to the live [RTC](RTC.md). |
| `$DD0000-$DDFFFF` | Open bus. |
| `$DE0000-$DFFFFF` | Custom-chip classification; `MemoryBus` routes the live register bank. |
| `$E00000-$F7FFFF` | Open bus; expansion and diagnostic ROM regions are not installed by these presets. |
| `$F80000-$FFFFFF` | Kickstart ROM. The fixed 256 KiB image repeats across this 512 KiB window. CPU writes do not modify ROM. |

The table describes the handlers installed by the emulator. The manual
describes hardware address-space reservations. The layout of allocations
made by AmigaOS is outside the manual map's scope. The manual identifies an
8 MB primary Auto-config area (`$200000-$9FFFFF`) and places a 256 KB system
ROM at `$FC0000-$FFFFFF`.
The current expanded preset populates only the first 4 MB of that expansion
area. The emulator also decodes Kickstart from `$F80000` as a mirror of
the same 256 KiB image.
[Hardware Reference Manual's system map](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md)

## Boot overlay and reset

Construction and both reset paths engage the ROM overlay by switching banks
`$00-$07` from Chip RAM handlers to Kickstart handlers. The CPU then reads its
reset vectors at `$000000` and `$000004` from ROM.

Until a queued CIA-A `PRA` or `DDRA` write commits, the previous overlay
mapping stays in effect. On commitment, CIA-A reports a generic port change
and the machine loop passes it through the motherboard router. The router
applies the PA0 level in that same phase, with no extra propagation stage.
The pull-up keeps PA0 high when it is an input. A high OVL level selects
Kickstart ROM, and a low level selects Chip RAM.
[Machine loop](Machine%20Loop.md) and
[`MemoryBus`](MemoryBus.md#cia-and-boot-overlay)

Switching handlers leaves the contents of Chip RAM intact. Agnus bypasses
the CPU overlay for direct DMA reads and writes by using the counted
`ChipRamAccess` word methods.

`reset()` clears installed Chip, Slow, and Fast RAM to zero, clears
`chip_ram_blocked`, and restores the overlay. `reset_warm()` preserves RAM,
clears the contention flag, and restores the overlay. Both retain the ROM
image. When no ROM has been injected, construction supplies a fallback 256 KB
`$FF` image with synthetic initial stack-pointer and program-counter vectors
so the emulator can start headless.

## Runtime access admission

Physical memory keeps Chip RAM bytes and the access budget private. Agnus
and its Copper, Blitter, bitplane, sprite, audio, and disk engines use the
foundational `ChipRamAccess` word interface, which exposes word operations
without access to raw storage or a way to reset the budget. CPU storage
handlers use the same budget once the ordinary contention check grants access.
The budget allows one completed byte or word transfer in a machine CCK,
regardless of direction or producer. A second transfer in that CCK causes a
fatal emulation error before storage is read or changed. Register delivery and
refresh perform no RAM data transfer. Independent Fast RAM, ROM, and device
accesses are outside the shared RAM transfer budget.

The machine supplies its current CCK before subsystem and CPU work.
Physical memory compares that value with the CCK of the last actual access
and requires at least one CCK between transfers. The check stores only the
last access time. It needs no per-cycle clearing or history of producers
or addresses. If it rejects a transfer, the diagnostic reports the previous
and current CCKs. The CPU also checks its bus-cycle spacing independently.
[CPU cycle-spacing guard](CPU%20Micro-Step%20State%20Machine.md#bus-admission-guard)

A subsystem-only step also advances the master CCK. Cloning and serialization
preserve the access timestamp, so restoration still enforces the budget for
the saved CCK. Reset adjusts the timestamp with the clock origin, preserving
its wrapping distance from the current CCK and carrying the budget across
the reset. Standalone CPU and storage execution advances its CCK through
`AddressBus`. The production router uses the current machine CCK. Debug
access, injection, and reset or prefetch setup consume no runtime budget.

## Access semantics

Before the CPU step, the machine loop copies Agnus's arbitration result
into `chip_ram_blocked`. While the flag is set, the Chip RAM and installed
Slow RAM handlers return `BusResult::WaitState`. The CPU keeps its bus
micro-step pending until a later grant. Fast RAM, ROM, and open-bus handlers
work independently of the flag. `PhysicalMemory` enforces the result it
receives. The DMA specification defines who owns the slot, and
machine-coordinated DMA bypasses this CPU wait state through direct Chip RAM
word operations. [Machine loop](Machine%20Loop.md) and [DMA](DMA.md)

RAM word handlers store the high byte at the lower address and the low byte
at the next address. Unmapped byte reads return the fixed `OPEN_BUS_BYTE`
placeholder (`$FF`). Unmapped word reads return `OPEN_BUS_WORD` (`$FFFF`).
[Open-bus hardware limits](Platform%20Quirks%20and%20Invariants%20Catalog.md#open-bus-and-opposite-direction-register-access)

Unmapped writes complete without storing data. ROM writes through the normal
CPU path also complete and leave the image intact.

The debug byte and word handlers bypass contention and device side
effects. `write_bytes_debug()` uses those handlers to inject memory,
including loading ROM. If a block starts in or crosses into the high
ROM window, it must contain exactly 262144 bytes. For any other size
the operation returns an error before writing anything. An individual
debug byte or word write can patch the ROM without resizing it. Every
ROM access uses the fixed `$3FFFF` mask, including accesses through
the low-memory overlay. The private buffer remains fixed in length,
and deserialization rejects snapshots containing a different ROM size.

A direct debug access to `$A0-$BF`, `$DC`, or `$DE-$DF` uses the physical
bank's placeholder behavior. Use the motherboard router's debug interface
when inspection needs to decode live peripheral registers.
[Router's debug interface](MemoryBus.md)

## Reference Documentation & Upstream Ground Truth

- [Amiga Hardware Reference Manual: Appendix D](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md): Hardware address reservations and expansion ranges against which the implemented presets can be compared.
- [`physical_memory.rs`](../../../crates/physical_memory/src/physical_memory.rs): Storage ownership, overlay and reset transitions, DMA access, and debug entry points.
- [`presets.rs`](../../../crates/physical_memory/src/presets.rs) and [`map.rs`](../../../crates/physical_memory/src/map.rs): Exact installed bank ranges and byte/word handler behavior.
- [`memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs): Production CPU routing for CIA, RTC, and custom registers and the CIA-A overlay control.
- [`test_physical_memory.rs`](../../../crates/physical_memory/tests/test_physical_memory.rs) and [`test_map.rs`](../../../crates/physical_memory/tests/test_map.rs): Component checks for overlay, contention, RAM access, ROM mapping, and unmapped values.
