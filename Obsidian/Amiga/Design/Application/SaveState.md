---
title: "Amiga 500 Save State Architecture & Serialization Specification"
aliases: ["SaveState", "State Serialization", "Snapshots"]
tags: ["amiga", "design", "savestate", "serialization", "snapshots"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-08-31
updated: 2026-11-23
related: ["[General Architecture.md](../General%20Architecture.md)", "[MemoryBus.md](../MemoryBus.md)", "[CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[Agnus.md](../Agnus.md)", "[RTC.md](../RTC.md)"]
tracked_paths:
  - "crates/machine_loop/src/state.rs"
  - "crates/save_state"
---
# Amiga 500 Save State Architecture & Serialization Specification

> [!NOTE]
> The system's ownership and component isolation rules are defined in [AGENTS.md](../../../../AGENTS.md) and [General Architecture.md](../General%20Architecture.md).
> The master clock synchronization rules are described in [Machine Loop.md](../Machine%20Loop.md).
> The memory layout and low-memory overlay rules are defined in [MemoryBus.md](../MemoryBus.md). The machine loop drives machine state serialization. [Machine Loop.md](../Machine%20Loop.md)
> The subsystem notes define their state schemas. [CPU Motorola M68000.md](../CPU%20Motorola%20M68000.md), [Agnus.md](../Agnus.md), [Denise.md](../Denise.md), [Paula.md](../Paula.md), [CIA.md](../CIA.md), [RTC.md](../RTC.md)

---

## 1. State ownership and persistence

The machine owns the data-only, serializable `A500State` that it captures and
restores on both native and WASM. Capture and restoration operate directly on
machine data, without file access, JSON encoding or PNG conversion. The
machine and the original subsystems retain ownership of chip values, CPU
state, memory, configuration, master CCK and motherboard deliveries.
Restoration preserves pending events without creating signal edges.
[state.rs](../../../../crates/machine_loop/src/state.rs)

The higher-level save_state crate depends on `machine_loop`. The dependency
is one-way. The save_state crate owns the `SaveState` file envelope, the
schema version, `SaveStateHeader`, JSON conversion and PNG preview encoding.
[save_state crate](../../../../crates/save_state)

The machine owner encodes or decodes JSON. The frontend owns file I/O, so
the GUI reads and writes those documents. The file content and version
validation rules still apply after the removal of direct file helpers.
[frontend](GUI.md#input-and-output)

The loader checks that the file uses schema version 15 before restoring it.
The envelope contains `header` and a boxed `state` value, which keeps large
serialized payloads off the call stack. The JSON holds an ordinary state
object. Earlier layouts have no compatibility parser. Frontends and the
debugger use the save_state crate for persistence. In-memory quick slots
keep only the machine value. The schema changes and user-data preservation
rules are defined in the [internal migration contract](../../../../.agents/references/internal-migrations.md).

The state uses access timestamps in place of CPU admission countdowns
and shared Chip RAM transfer descriptions. It retains the active CPU sequence
identity, CPU bus-cycle start, pending refused operation and last shared Chip
RAM access. Cloning or restoring preserves an occupied cycle without creating
another transfer opportunity. Pending keyboard releases survive a full
type-ahead queue, preserving release delivery through another capture and exact
restoration.

The snapshot records which drives are attached and the player association.
It embeds the current contents of mounted disks, including guest writes. The
frontend can restore those disks without the original ADF files.
The chipset activity observations preserve drive attribution across the
captured control boundary, so they still identify which drives a transfer
belonged to. When the frontend restores the machine, it starts drive-activity
feedback from a new baseline. The snapshot omits the wall-clock hold times
for that feedback. [GUI](GUI.md)

The snapshot excludes the live RTC and its independent battery offset. It
stores Denise's frame cursor with the frame buffer, preserving the write
position across visible and blanking boundaries. The buffer uses a fixed
720-sample row stride and the selected PAL/NTSC presentation profile. It also
keeps destination addresses for pending line output. PNG previews use exactly
the same 720 x 568 PAL or 720 x 480 NTSC rectangle as the host image.
[frame buffer](../Frame%20Buffer.md)

Chipset engines stay in their owning crates. The only values for DMACON,
BPLCON0, DDFSTRT and DDFSTOP are in `state.agnus.dma`. The snapshot groups
sprite coordinates and vertical DMA latches under `state.agnus.sprite_dma`,
and restoration rebuilds the derived comparison results. Copper stores its
instruction progress and pending SKIP decision. A restored partial instruction
must wait for the next granted Copper DMA opportunity before consuming the
remaining word.

Earlier files are unsupported. The loader provides neither migration nor a
default-state fallback and leaves those files on the host.

## 2. Full snapshot contents

`A500State` includes the CPU's micro-execution state, physical memory and ROM,
Agnus, Denise, Paula, both CIAs, floppy, keyboard and game ports. The custom
chips already hold their own state values, so snapshots capture their
registers, input levels and pending changes directly without separate per-chip
wrappers.

Each receiver stores incoming inputs alongside the sampled levels and its own
synchronization state. The inputs include IRQ/TOD, disk control/data, game-port
drive and RGA events. CIA TOD/FLAG history and Paula interrupt levels stay
with their respective chips. The machine keeps one pending
`keyboard_reset_asserted` boolean, and the CPU keeps its IPL. Receivers own
pending delivery as state without a shared pending input frame or a parallel
connector model. A pending delivery has yet to change a register and is
distinct from the last four synchronization values.

Resuming a snapshot at CCK T must produce the same subsequent state as
uninterrupted execution. This includes signals captured but awaiting delivery
and register words awaiting acceptance. The CPU preserves redirected sequences
and partial bus transfers independently of the current instruction register.
[CPU execution identity](../CPU%20Micro-Step%20State%20Machine.md#internal-registers-and-execution-state)

Construction and reset establish released levels. Current snapshots require
all state fields and contain no host handles. There are no legacy defaults.
The saved configuration determines whether the RTC is present. Its live clock
continues from current host UTC and its independently persisted offset.
Capture and restore omit its time, battery offset, calendar latches and
controls. [RTC](../RTC.md)

---

## Player progress

Save files carry the actual host creation time and a PNG preview, with optional
game ID and four stable mounted-disk IDs. ROM and media bytes stay embedded in
the machine. Host handles and library paths stay outside machine state.
Desktop games have durable save catalogs and per-game quick slots. Manual
browser mode retains file import/export and session slots.

A save file contains the disks mounted when the player saves. Opening it as a
new session starts with those disks and any available source ADFs. During a
session, a quick load restores the mounted disks from its slot. The player
keeps disks absent from the slot, including disks displaced from a drive by
the load. When an existing copy uniquely matches a restored copy, the slot's
contents take precedence. The drive assignment identifies independent copies
with the same disk ID when placement resolves the match. Loading a slot from an
earlier manual session starts with that slot's mounted disks.

Initial states are immutable launch assets stored separately from user saves.
If an initial state is invalid, the frontend offers normal boot as an explicit
choice. Loading validates the version, game/media association and committed
input settings before replacing a live session. When the player continues a
saved game, the frontend restores the saved machine and resumes it directly,
without booting or applying the current hardware preset.

After restoration, the frontend releases saved controls through the normal
keyboard protocol, discards obsolete host events and reconciles the endpoints
with the active session's current committed controls. The captured device
discriminator identifies the device in the emulated state. The frontend uses
the current host bindings and allows a different current attachment. Unchanged
endpoints retain their emulated phase, and changed endpoints start neutral.
The receivers retain their chip state and sampling phases. Initial states,
library saves, imported files and quick slots share this policy.

## 3. Differential Save States (`A500DiffState`)

Differential full-machine rewind remains a design proposal. The proposed
format and remaining work are listed under [debugger work](../../../../ROADMAP.md#debugger-execution-control).

---

## 4. File loading and restoration

The save_state crate decodes the envelope and rejects an unsupported version.
When decoding physical memory, it also rejects a Kickstart buffer of any
length other than exactly 256 KiB (262144 bytes). Both checks must pass before
the machine is restored.

The frontend then passes the pure `state` value to `A500Machine::load_state`.
The machine adopts the saved configuration, RAM/ROM, CPU micro-execution
state, chip values, receiver-owned incoming inputs and board reset state.
There is no routing during loading. The core restores input and device state
exactly. The frontend applies its release and attachment policy before guest
execution resumes. Capturing a state leaves the running machine's inputs as
they are. Current files embed physical memory, including ROM. Alternate ROM
reference modes and differential rewind remain proposals.

---

## 5. Reference Documentation & Upstream Ground Truth

- [vAmiga Snapshot Component Implementation](../../../../ref_src/vAmiga/Core/Media/Snapshot.cpp): Snapshot serializer for Amiga hardware state, block headers, and uncompressed RAM payloads.
- [CPU State Snapshot Implementation Source](../../../../crates/cpu/src/state.rs): Living Rust `CpuState` data structures implementing Serde serialization and deserialization.

- [Pure state](../../../../crates/machine_loop/src/state.rs): Defines the core snapshot schema.
- [Save format](../../../../crates/save_state/src/save_state.rs): Implements the file envelope, JSON and PNG persistence.
