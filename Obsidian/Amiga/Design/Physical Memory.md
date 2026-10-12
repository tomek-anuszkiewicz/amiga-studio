---
title: "Physical Memory"
aliases: ["Amiga 500 Physical Memory", "PhysicalMemory", "Memory Bank Map"]
tags: ["amiga", "design", "physical_memory", "chip_ram", "memory_map"]
category: "Design"
subsystem: "physical_memory"
status: "active"
created: 2026-10-12
updated: 2026-10-12
related: ["[General Architecture.md](General%20Architecture.md)", "[MemoryBus.md](MemoryBus.md)", "[Configuration.md](Configuration.md)", "[Machine Loop.md](Machine%20Loop.md)", "[DMA.md](DMA.md)", "[RTC.md](RTC.md)"]
tracked_paths:
  - "crates/physical_memory"
---
# Physical Memory

[`PhysicalMemory`](../../../crates/physical_memory/src/physical_memory.rs) owns the Amiga 500's RAM and Kickstart ROM buffers, the 24-bit bank table, the low-memory boot overlay, and the current CPU memory-contention flag. It supplies byte and big-endian word storage operations. The [motherboard router](MemoryBus.md) handles live CIA, RTC, and custom-register access; the [machine loop](Machine%20Loop.md) owns the storage and coordinates its use with the CPU and Agnus. Physical memory has no direct references to those devices.

## Installed storage and address decoding

The 16 MB CPU address space is classified by a 256-entry table of 64 KB banks. [`presets.rs`](../../../crates/physical_memory/src/presets.rs) builds a table for each [configuration preset](Configuration.md); each [`BankHandler`](../../../crates/physical_memory/src/map.rs) supplies byte and word read/write functions, including separate debugger operations. The selected bank is `(addr >> 16) & $FF`. `MemoryBus` first masks CPU addresses to 24 bits and routes device banks; `PhysicalMemory` selects handlers for storage and open bus. A bank's CIA, RTC, or custom-chip classification does not make a direct `PhysicalMemory` access reach the live device.

| Address | Installed behavior in the current presets |
| --- | --- |
| `$000000-$07FFFF` | 512 KB Chip RAM after overlay removal; Kickstart ROM while overlay is active. |
| `$080000-$1FFFFF` | Open bus; the current presets do not install extended Chip RAM. |
| `$200000-$5FFFFF` | 4 MB Fast RAM only in `ExpandedPowerUser`; open bus otherwise. No Chip RAM contention check. |
| `$600000-$BEFFFF` | Open bus in the current presets. |
| `$BF0000-$BFFFFF` | CIA classification for direct physical-memory access; `MemoryBus` decodes the live CIA byte lanes. |
| `$C00000-$C7FFFF` | 512 KB Slow RAM in `Standard1Mb` and `ExpandedPowerUser`; open bus in `Bare512k`. |
| `$C80000-$DBFFFF` | Open bus. |
| `$DC0000-$DCFFFF` | RTC classification when installed, otherwise open bus. `MemoryBus` routes only `$DC0000-$DC003F` to the live [RTC](RTC.md). |
| `$DD0000-$DEFFFF` | Open bus. |
| `$DF0000-$DFFFFF` | Custom-chip classification; `MemoryBus` routes the live register bank. |
| `$E00000-$F7FFFF` | Open bus; expansion and diagnostic ROM regions are not installed by these presets. |
| `$F80000-$FFFFFF` | Kickstart ROM. A 256 KB image repeats across this 512 KB window; a 512 KB image occupies it once. CPU writes do not modify ROM. |

This table describes the emulator's installed handlers, not a promise that every hardware-reserved address is usable RAM. The [Hardware Reference Manual's system map](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md) identifies an 8 MB primary Auto-config area (`$200000-$9FFFFF`) and locates a 256 KB system ROM at `$FC0000-$FFFFFF`. The current expanded preset populates only the first 4 MB of that expansion area; the emulator also decodes Kickstart from `$F80000` to support its 256/512 KB image window. The manual's map is a hardware address-space guide, not the layout of allocations made by AmigaOS.

## Boot overlay and reset

Construction and both reset paths engage the ROM overlay by changing banks `$00-$07` from Chip RAM handlers to Kickstart handlers. CPU reads of the reset vectors at `$000000` and `$000004` therefore see ROM. When CIA-A Port A bit 0 selects Chip RAM, [`MemoryBus`](MemoryBus.md) calls `map_chip_ram_to_low_memory()`; the opposite value restores the ROM handlers. The [machine loop](Machine%20Loop.md) also applies the CIA's reported overlay transition. Switching handlers does not copy or erase Chip RAM. Direct Agnus DMA reads and writes use `read_chip_word()` and `write_chip_word()` on the Chip RAM buffer, so they bypass the CPU overlay.

`reset()` clears installed Chip, Slow, and Fast RAM to zero, clears `chip_ram_blocked`, and restores the overlay. `reset_warm()` preserves RAM while clearing the contention flag and restoring the overlay. Both retain the ROM image. When no ROM has been injected, construction supplies a 256 KB `$FF` image with synthetic initial stack-pointer and program-counter vectors for headless startup; this is an emulator fallback, not a Kickstart ROM.

## Access semantics

Chip RAM and installed Slow RAM handlers return `BusResult::WaitState` while `chip_ram_blocked` is set. The [machine loop](Machine%20Loop.md) copies Agnus's arbitration result into that flag before the CPU step; the CPU keeps its pending bus micro-step until a later grant. Fast RAM, ROM, and open-bus handlers do not test that flag. [DMA](DMA.md) defines the slot ownership; `PhysicalMemory` only enforces the supplied contention result. Direct Chip RAM word operations bypass this CPU wait state for machine-coordinated DMA.

RAM word handlers store the high byte at the lower address and the low byte at the next address. Unmapped reads return the configured `unmapped_byte` (default `$FF`); word reads repeat it in both byte lanes (`$FFFF` by default). Unmapped writes complete without storage. This is a fixed open-bus approximation, not a history-dependent model of floating data lines. ROM writes through the normal CPU path likewise complete without modifying the image.

The debug byte/word handlers bypass contention and device side effects. `write_bytes_debug()` uses them for memory injection, including ROM loading; unlike CPU ROM writes, debug writes to the ROM address range can update the ROM buffer. A direct debug access to `$BF`, `$DC`, or `$DF` sees the physical bank's placeholder behavior, not live peripheral registers. Use the [router's debug interface](MemoryBus.md) when inspection needs device-aware decoding.

## Reference Documentation & Upstream Ground Truth

- [Amiga Hardware Reference Manual: Appendix D](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md): Hardware address reservations and expansion ranges against which the implemented presets can be compared.
- [`physical_memory.rs`](../../../crates/physical_memory/src/physical_memory.rs): Storage ownership, overlay and reset transitions, DMA access, and debug entry points.
- [`presets.rs`](../../../crates/physical_memory/src/presets.rs) and [`map.rs`](../../../crates/physical_memory/src/map.rs): Exact installed bank ranges and byte/word handler behavior.
- [`memory_bus.rs`](../../../crates/memory_bus/src/memory_bus.rs): Production CPU routing for CIA, RTC, and custom registers and the CIA-A overlay control.
- [`test_physical_memory.rs`](../../../crates/physical_memory/tests/test_physical_memory.rs) and [`test_map.rs`](../../../crates/physical_memory/tests/test_map.rs): Component checks for overlay, contention, RAM access, ROM mapping, and unmapped values.
