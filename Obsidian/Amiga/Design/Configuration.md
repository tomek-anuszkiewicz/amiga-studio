---
title: "Amiga 500 Configuration Specification (A500Config)"
aliases: ["A500Config", "Configuration", "Machine Configuration"]
tags: ["amiga", "design", "config", "presets"]
category: "Design"
subsystem: "config"
status: "active"
created: 2026-09-05
updated: 2026-11-17
related: ["[General Architecture.md](General%20Architecture.md)", "[Physical Memory.md](Physical%20Memory.md)", "[MemoryBus.md](MemoryBus.md)", "[RTC.md](RTC.md)", "[Machine Loop.md](Machine%20Loop.md)"]
tracked_paths:
  - "crates/config"
---
# Amiga 500 Configuration Specification (`A500Config`)

> [!NOTE]
> The core receives all ROM binaries, disk images and memory configurations from outside the core as raw byte slices (`&[u8]`). This keeps the core portable to WASM and independent of the host system. See [AGENTS.md](../../../AGENTS.md).
> The dedicated foundational crate [`crates/config`](../../../crates/config) implements configuration.
> The components that apply the configuration handle physical memory layout in [Physical Memory.md](Physical%20Memory.md), motherboard routing in [MemoryBus.md](MemoryBus.md), RTC models in [RTC.md](RTC.md), chipset variants in [Agnus.md](Agnus.md) and [Denise.md](Denise.md), and system coordinator setup in [Machine Loop.md](Machine%20Loop.md).

All presets use the same CIA banks `$A0-$BF` and custom banks `$DE-$DF`.
The physical memory code and the production router both import the same
[address bounds](../../../crates/config/src/address_map.rs) to classify
addresses. These constants define address ranges only. During a transaction,
[MemoryBus](MemoryBus.md#cia-and-boot-overlay) selects the register and decides
whether to accept the access.

Configuration provides hardware presets, register definitions, the shared
mutation queue and `big_array` Serde support. The array helper sits below the
subsystem crates and works independently of the high-level [save format](Application/SaveState.md).
The `hpos`/`vpos` counters and field/line state that track the beam belong to
[Agnus](Agnus.md#motherboard-transport-boundary).

---

## 1. Focused Scope: 3 Canonical Hardware Presets

The core supports exactly **three canonical hardware presets**. Limiting the
set of hardware combinations supports the goal of cycle-exact emulation.

| Preset | Installed memory | RTC |
| --- | --- | --- |
| `Bare512k` | 512 KB Chip RAM | None; open bus at `$DC0000` |
| `Standard1Mb` (default) | 512 KB Chip + 512 KB Slow RAM at `$C00000` | OKI MSM6242B at `$DC0000` |
| `ExpandedPowerUser` | 512 KB Chip + 512 KB Slow + 4 MB Auto-Config Fast RAM at `$200000` | OKI MSM6242B at `$DC0000` |

The standard PAL profiles use Agnus 8371 and Denise 8362, with a mouse on
port 1 and a joystick on port 2. Kickstart 1.2/1.3 is supplied externally.

---

## 2. Configuration Immutability & Data Structure

To keep memory and device settings within the three supported profiles,
[`A500Config`](../../../crates/config/src/config.rs) allows changes to those
settings only by switching the entire preset atomically. Changes to internal
configurations follow the
[internal migration contract](../../../.agents/references/internal-migrations.md).

---

## 3. ROM & Image Injection Interfaces

The emulator core is independent of the host system and its filesystem. The
host must supply ROMs and disk images as external byte slices (`&[u8]`).

- **Kickstart ROM Injection** accepts exactly 256 KiB (262144 bytes) in every preset, matching the factory Kickstart 1.2/1.3 configuration. `PhysicalMemory::write_bytes_debug()` rejects every other image length before it changes memory. The debugger and GUI pass along that error. Larger upgraded ROMs are outside the supported configuration.
- **Floppy ADF Injection** supplies raw byte slices `&[u8]` to the disk drives. The same interface works in desktop environments and WebAssembly canvas contexts.
- **Zero Host I/O in Core** keeps `std::fs` operations outside the core emulation engine.

---

## Named frontend configurations

The frontend combines one of the three presets with a ROM selection, four drive
connection choices and two independent game-port devices. The user chooses
peripheral attachments separately from memory and chipset. Each named
configuration has a stable ID, an editable name and references to reusable
keyboard and controller mappings. Renaming the configuration preserves its game
associations. The scope excludes per-game inheritance and hardware overrides.

On desktop, each library's root contains its catalog of named configurations
and mappings. A library configuration refers to a Kickstart file by its path
relative to that root. A library copy carries those profiles and managed assets
with it. The current user's application storage holds the selected root,
active editor selection and other host preferences. An accessible library
without a catalog starts with the built-in profile and mapping. If a catalog
exists and cannot be read or validated, the desktop reports the problem and
protects its contents. On first upgrade, the desktop transfers the previous
native catalog into its previously selected library, preserving all IDs. It
keeps an exact backup of the old settings and reports any conflict or
unavailable Kickstart that prevents the transfer.

The browser frontend saves named settings in browser storage. When the user
reopens the browser, they must select any unavailable ROM or disk bytes again.

The user edits configurations in the [GUI](Application/GUI.md), which also
validates their launch choices. The saved machine hardware, ROM and drive
progress govern restoration. Launch choices are independent of game-port
attachments and host input settings. The GUI applies edits to launch choices
only on an explicit launch. The user can still continue the existing machine.
The active session keeps its library and named configuration identity when the
user selects another profile or library for a future launch. When loading into
that session, the player uses its committed controls and mappings to determine
port attachments and host input, even if the captured device types differ.
Host paths, controller identities, mappings and presentation preferences stay
outside the machine snapshot.

## 5. Deferred Peripheral Extension

Extended peripherals remain deferred. See the [deferred peripheral work](../../../ROADMAP.md#deferred-bus-and-peripherals).

---

## Shared propagation state

The shared [MutationQueue](../../../crates/config/src/mutation.rs) preserves
issue order. Pipeline writes can share the queue. OverwritePending replaces
only a pending write with the same destination and mode. The queue puts the
replacement after the retained writes. It refuses zero-delay writes. If an
insertion would overflow, the owner stops before changing pending or due data,
and the write stays unapplied. An entry can be replaced even at full capacity.
CIA captures writes at its own falling-E boundary. Custom-register delays and
routing belong to [MemoryBus](MemoryBus.md#custom-registers-and-specialized-chips).

At each tick the queue selects all due writes before the chip applies any of
them. Any writes staged while the chip applies them must wait for a later
tick. Snapshots save pending writes only. Reset cancels pending writes and
the observation of an already committed write. The loader rejects stored
state with too many entries or a write with zero remaining delay. Restoration
and cloning must avoid delivering committed observations again.

## 6. Reference Documentation & Upstream Ground Truth

- The address boundaries for 512 KB Chip, 512 KB Slow and the 8 MB Auto-Config address space are documented in the [Amiga Hardware Reference Manual: Appendix D (System Memory Map)](../Reference/Hardware%20Reference%20Manual/12%20-%20Appendix%20D%20-%20System%20Memory%20Map.md).
- Motherboard revision differences, video timing selection and expansion bus options are described in the [A500/A2000 Technical Reference Manual: Section 1 (Summary of Differences)](../Reference/A500%20A2000%20Technical%20Reference%20Manual/01%20-%20Section%201%20Summary%20of%20Differences.md).
- The Rust source implements `A500Config`, the hardware presets `Bare512k`, `Standard1Mb` and `ExpandedPowerUser`, and the video standards. [Configuration Subsystem Implementation Source](../../../crates/config/src/config.rs)
- The queue used by custom chips and CIA provides fixed storage, issue ordering, due observations and pending-state serialization. [Mutation queue](../../../crates/config/src/mutation.rs)
