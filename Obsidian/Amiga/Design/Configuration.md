---
title: "Amiga 500 Configuration Specification (A500Config)"
aliases: ["A500Config", "Configuration", "Machine Configuration"]
tags: ["amiga", "design", "config", "presets"]
category: "Design"
subsystem: "config"
status: "active"
created: 2026-09-05
updated: 2026-11-16
related: ["[General Architecture.md](General%20Architecture.md)", "[Physical Memory.md](Physical%20Memory.md)", "[MemoryBus.md](MemoryBus.md)", "[RTC.md](RTC.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/config"
---
# Amiga 500 Configuration Specification (`A500Config`)

> [!NOTE]
> All ROM binaries, disk images, and memory configurations are injected into the core externally as raw byte slices (`&[u8]`), preserving WASM portability and system independence (see [AGENTS.md](../../../AGENTS.md)).
> Implementation resides in the dedicated foundational crate [`crates/config`](../../../crates/config).
> Physical memory layout is applied in [Physical Memory.md](Physical%20Memory.md), motherboard routing in [MemoryBus.md](MemoryBus.md), RTC models in [RTC.md](RTC.md), chipset variants in [Agnus.md](Agnus.md) and [Denise.md](Denise.md), and system coordinator setup in [Machine Loop.md](Machine%20Loop.md).

The shared [address bounds](../../../crates/config/src/address_map.rs) define
CIA banks `$A0-$BF` and custom banks `$DE-$DF` for all presets. Physical-memory
classification and the production router import these same definitions.
[MemoryBus](MemoryBus.md#cia-and-boot-overlay) owns register selection and
transaction acceptance; these constants do not instantiate devices.

Beam position remains in [Agnus](Agnus.md#motherboard-transport-boundary)
as its `hpos`/`vpos` counters and field/line state. Configuration retains hardware presets,
register definitions, the shared mutation queue and `big_array` Serde support.
The array helper is below subsystem crates and has no dependency on the
high-level [save format](Application/SaveState.md).

---

## 1. Focused Scope: 3 Canonical Hardware Presets

To achieve cycle-exact emulation and eliminate configuration explosion, the emulator core is configured exclusively through **three canonical hardware presets**:

| Preset | Installed memory | RTC |
| --- | --- | --- |
| `Bare512k` | 512 KB Chip RAM | None; open bus at `$DC0000` |
| `Standard1Mb` (default) | 512 KB Chip + 512 KB Slow RAM at `$C00000` | OKI MSM6242B at `$DC0000` |
| `ExpandedPowerUser` | 512 KB Chip + 512 KB Slow + 4 MB Auto-Config Fast RAM at `$200000` | OKI MSM6242B at `$DC0000` |

The standard PAL profiles use Agnus 8371 and Denise 8362, with a mouse on
port 1 and a joystick on port 2. Kickstart 1.2/1.3 is supplied externally.

---

## 2. Configuration Immutability & Data Structure

[`A500Config`](../../../crates/config/src/config.rs) permits atomic preset
changes rather than independently mutable memory and device settings. This keeps
the supported configurations within the three deliberate profiles above.

---

## 3. ROM & Image Injection Interfaces

Because the emulator core is system-agnostic and decoupled from the host filesystem, all ROMs and disk images are injected strictly as external byte slices (`&[u8]`):
- **Kickstart ROM Injection**: All presets support exactly 256 KiB (262144 bytes), matching the factory Kickstart 1.2/1.3 configuration. ROM injection through `PhysicalMemory::write_bytes_debug()` rejects every other image length before changing memory. The debugger and GUI propagate this error; larger upgraded ROMs are outside the supported configuration.
- **Floppy ADF Injection**: Disk drives accept ADF images as raw byte slices `&[u8]`, enabling seamless operation in both desktop environments and WebAssembly canvas contexts.
- **Zero Host I/O in Core**: No `std::fs` operations exist inside the core emulation engine.

---

## Named frontend configurations

The frontend composes these three presets with ROM selection, four drive
connection choices and two independent game-port devices. Peripheral attachment
is separate from memory/chipset selection. Each global configuration has a stable
ID, editable name and reusable keyboard/controller mapping references. Renaming
it preserves game associations. No per-game inheritance or hardware overrides
are provided. Desktop settings persist on the host; browser settings persist in
browser storage without caching ROM or disk contents. Reopening the browser
requires explicit reselection of unavailable assets.

The [GUI](Application/GUI.md) owns configuration editing and launch validation.
Saved machine hardware is authoritative on restore; current host bindings apply
only to compatible port devices. A mismatch requires configuration selection.

## 5. Deferred Peripheral Extension

Extended peripherals remain deferred; see the [deferred peripheral work](../../../ROADMAP.md#deferred-bus-and-peripherals).

---

## Shared propagation state

The shared [MutationQueue](../../../crates/config/src/mutation.rs) preserves
issue order. Pipeline writes coexist; OverwritePending replaces only a pending
write with the same destination and mode and places the replacement after
retained writes. Zero-delay writes are refused, and overflow stops the owner
before changing pending or due data rather than applying a write early. A
replacement at capacity remains valid. CIA retains its separate falling-E
capture boundary; [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips)
owns custom-register delays and routing.

All writes due at a tick are selected before chip application. Writes staged
during application cannot join that tick's due set. Snapshots retain pending
writes rather than the already committed observation; reset cancels both.
Loading rejects excessive capacity or zero remaining delay. Already committed
observations must not be delivered again after restoration or cloning.

## 6. Reference Documentation & Upstream Ground Truth

- [Amiga Hardware Reference Manual: Appendix D (System Memory Map)](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md): Memory map boundaries for 512 KB Chip, 512 KB Slow, and 8 MB Auto-Config address spaces.
- [A500/A2000 Technical Reference Manual: Section 1 (Summary of Differences)](../Reference/A500%20A2000%20Technical%20Reference%20Manual/01%20-%20Section%201%20Summary%20of%20Differences.md): Motherboard revision differences, video timing selection, and expansion bus options.
- [Configuration Subsystem Implementation Source](../../../crates/config/src/config.rs): Rust implementation of `A500Config`, hardware presets (`Bare512k`, `Standard1Mb`, `ExpandedPowerUser`), and video standards.
- [Mutation queue](../../../crates/config/src/mutation.rs): Fixed storage, issue ordering, due observations and pending-state serialization shared by custom chips and CIA.
