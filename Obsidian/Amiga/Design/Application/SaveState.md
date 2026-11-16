---
title: "Amiga 500 Save State Architecture & Serialization Specification"
aliases: ["SaveState", "State Serialization", "Snapshots"]
tags: ["amiga", "design", "savestate", "serialization", "snapshots"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-08-31
updated: 2026-11-16
related: ["[General Architecture.md](../General%20Architecture.md)", "[MemoryBus.md](../MemoryBus.md)", "[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[Agnus.md](../Agnus.md)", "[RTC.md](../RTC.md)"]
tracked_paths:
  - "crates/machine_loop/src/state.rs"
  - "crates/save_state"
---
# Amiga 500 Save State Architecture & Serialization Specification

> [!NOTE]
> System ownership principles and decoupling constraints are defined in [AGENTS.md](../../../../AGENTS.md) and [General Architecture.md](../General%20Architecture.md).
> Master clock synchronization is detailed in [Machine Loop.md](../Machine%20Loop.md).
> Memory layout and low-memory overlay rules are specified in [MemoryBus.md](../MemoryBus.md). Machine state serialization is driven by [Machine Loop.md](../Machine%20Loop.md). Subsystem state schemas are defined in [CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md), [Agnus.md](../Agnus.md), [Denise.md](../Denise.md), [Paula.md](../Paula.md), [CIA.md](../CIA.md), and [RTC.md](../RTC.md).

---

## 1. State ownership and persistence

The machine owns a pure serializable `A500State` in
[state.rs](../../../../crates/machine_loop/src/state.rs). Capturing and restoring
it performs no file access, JSON encoding or PNG conversion. Chip values,
CPU state, memory, configuration, master CCK and motherboard deliveries remain
owned by the machine and their original subsystems. Restoring does not consume
pending events or synthesize a signal edge. Native and WASM use the same state.

The higher-level [save_state crate](../../../../crates/save_state) depends on
`machine_loop`, not the reverse. It owns the `SaveState` file envelope, schema
version, `SaveStateHeader`, JSON conversion and PNG preview encoding. The
[frontend](GUI.md#input-and-output) owns file I/O: the machine owner encodes or
decodes JSON, while the GUI reads and writes those owned documents. Retired
direct file helpers are unavailable; file contents and version validation retain
the same contract.
A decoded file must have the supported schema version before its machine value
is restored. The envelope contains `header` and a boxed `state` value, keeping large
serialized payloads off the call stack. JSON contains an ordinary state object;
old layouts have no
compatibility parser. Frontends and the debugger use this crate for persistence;
in-memory quick slots retain only the machine value.

Schema version 9 embeds usable floppy raw tracks and spindle phase, including
mid-transfer connector state; restoring requires no original ADF file. It also
retains the CPU bus-admission phases and active shared Chip RAM budget. Clone
and restore preserve an occupied cycle; neither creates another transfer
opportunity. Sprite coordinates and vertical DMA latches are grouped under
