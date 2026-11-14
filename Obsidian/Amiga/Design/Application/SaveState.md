---
title: "Amiga 500 Save State Architecture & Serialization Specification"
aliases: ["SaveState", "State Serialization", "Snapshots"]
tags: ["amiga", "design", "savestate", "serialization", "snapshots"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-08-31
updated: 2026-11-14
related: ["[General Architecture.md](../General%20Architecture.md)", "[MemoryBus.md](../MemoryBus.md)", "[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[Agnus.md](../Agnus.md)"]
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
version, `SaveStateHeader`, JSON conversion, file I/O and PNG preview encoding.
A decoded file must have the supported schema version before its machine value
is restored. The envelope contains `header` and a boxed `state` value, keeping large
serialized payloads off the call stack. JSON contains an ordinary state object;
old layouts have no
compatibility parser. Frontends and the debugger use this crate for persistence;
in-memory quick slots retain only the machine value.

Schema version 5 stores the [frame buffer](../Frame%20Buffer.md) with a fixed
row stride and retains destination addresses for pending line output. PNG
previews contain only the visible pixels of each row. Chipset engines remain
inside their owning crates; DMACON, BPLCON0, DDFSTRT and DDFSTOP have their sole
values in `state.agnus.dma`. Earlier files are unsupported; no
migration or default-state fallback is provided. This does not remove files
from the host. Copper still stores instruction progress and the pending SKIP
decision. A restored partial instruction must consume its remaining word only
after the next granted Copper DMA opportunity.

Subsystems retain their own Serde implementations. The shared fixed-array
serializer in `config::big_array` remains below them; it must not depend on
this high-level save format. Agnus beam counters and field/line state, receiver
timing state and RGA events are serialized at their respective owners.

## 2. Full snapshot contents

`A500State` includes the CPU's micro-execution state, physical memory and ROM,
RTC, Agnus, Denise, Paula, both CIAs, floppy, keyboard and game ports. Custom
chips are already value containers, so no duplicate per-chip snapshot wrappers
are introduced. Their registers, input levels and pending changes are retained.
Each receiver stores its incoming inputs alongside sampled levels and local
synchronization state. This includes IRQ/TOD, disk control/data, game-port drive
and RGA events. CIA TOD/FLAG history and Paula interrupt levels remain local.
The machine retains a single pending `keyboard_reset_asserted` boolean;
CPU IPL is stored by the CPU. There is no shared pending input frame or parallel
connector model. Pending delivery is state, not a register
mutation or a list of the last four synchronization values.

A snapshot at CCK T must resume with the same subsequent state as uninterrupted
execution, including a signal captured but not delivered and an unaccepted
register word. Construction/reset defines released levels. Current snapshots
require their complete state fields; there are no legacy defaults or host handles.

---

## 3. Differential Save States (`A500DiffState`)

Differential full-machine rewind remains a design proposal; its pending work
and proposed format are collected in the
[debugger work](../../../../ROADMAP.md#debugger-execution-control).

---

## 4. File loading and restoration

The save_state crate decodes the envelope and rejects an unsupported version.
Physical-memory deserialization also rejects any Kickstart buffer whose length
is not exactly 256 KiB (262144 bytes), before restoring the machine.
The frontend then passes its pure `state` value to `A500Machine::load_state`.
Restoration adopts the saved configuration, RAM/ROM, CPU micro-execution state,
chip values, receiver-owned incoming inputs and board reset state. It performs no routing during
loading. Current files embed physical memory, including ROM; alternate ROM
reference modes and differential rewind remain proposals rather than active APIs.

---

## 5. Reference Documentation & Upstream Ground Truth

- [vAmiga Snapshot Component Implementation](../../../../ref_src/vAmiga/Core/Media/Snapshot.cpp): Snapshot serializer for Amiga hardware state, block headers, and uncompressed RAM payloads.
- [CPU State Snapshot Implementation Source](../../../../crates/cpu/src/state.rs): Living Rust `CpuState` data structures implementing Serde serialization and deserialization.

- [Pure state](../../../../crates/machine_loop/src/state.rs): Defines the core snapshot schema.
- [Save format](../../../../crates/save_state/src/save_state.rs): Implements the file envelope, JSON and PNG persistence.
