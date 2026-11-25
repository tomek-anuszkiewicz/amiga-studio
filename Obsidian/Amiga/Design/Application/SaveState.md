---
title: "Amiga 500 Save State Architecture & Serialization Specification"
aliases: ["SaveState", "State Serialization", "Snapshots"]
tags: ["amiga", "design", "savestate", "serialization", "snapshots"]
category: "Design"
subsystem: "general"
status: "active"
created: 2026-08-31
updated: 2026-11-25
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

The JSON envelope has a `header` and a `state` object. The loader checks
schema version 17 before interpreting the state layout, regardless of member
order. Earlier files remain intact and report an unsupported version. Frontends
and the debugger use the same file codec. An in-memory quicksave contains a
complete machine snapshot with the prepared drives and actual ROM bytes.
[Internal migration contract](../../../../.agents/references/internal-migrations.md)

The state uses access timestamps in place of CPU admission countdowns
and shared Chip RAM transfer descriptions. It retains the active CPU sequence
identity, CPU bus-cycle start, pending refused operation and last shared Chip
RAM access. Cloning or restoring preserves an occupied cycle without creating
another transfer opportunity. Pending keyboard releases survive a full
type-ahead queue, preserving release delivery through another capture and exact
restoration.

The snapshot records which drives are attached. The chipset activity observations preserve drive attribution across the
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

Player save files carry the host creation time in UTC, a full-resolution PNG
preview, an optional description, and game and mounted-disk IDs when available.
The catalog checks the complete document and supports previews and description
edits while source media are offline. Editing a description preserves the
creation time, preview, associations and machine snapshot.

A desktop library game designates one save as its quicksave. Each Quick save
writes a new file before replacing the designation. The previous file remains
ordinary progress. On first opening legacy data, the app can designate the
existing save referenced by legacy slot 1. An empty recorded designation keeps
it cleared across restarts. Manual sessions retain one in-memory quicksave
alongside file import and export on desktop and in the browser.

A save file identifies the source Kickstart and each ADF inserted at capture.
Guest recordings that exist only on the emulated disks survive in the current
session and in full in-memory slots. A file load restores the disk contents
from the verified source ADFs. The files can move or have new names. Desktop
can resolve the stable disk IDs in the opened game library and find exact ROM
bytes among managed Kickstarts or authorized configuration paths. The frontend
also accepts matching images already supplied in the session or selected by the
user. The recorded SHA-256 governs the match independently of the next boot
selection. On a fresh browser page, add the matching Kickstart in
**Configurations** and select disks in **Manual setup**. Choose **Return to
session**, then **Import state**. The frontend uses content matches to bind
mounted drives to the current page's selections.

On a quick load within a session the player keeps available disks absent from
the quicksave, including disks displaced from a drive by the load. When an
existing copy uniquely matches a restored copy, the quicksave's contents take
precedence. The drive assignment identifies independent copies with the same disk ID when
placement resolves the match. A quicksave from an earlier manual session opens
with its own mounted disks.

Each native library game owns a collection of named start states outside its
`saves/` directory. Each manifest entry has a stable file identity and a
player-visible name. To add one, the frontend validates the selected stored
save or current quicksave and copies its unchanged snapshot bytes into the
game's collection before publishing the entry. Ordinary saves and quicksave
replacement leave these copies intact. Rename changes only the display name.
Delete affects only the chosen entry and its owned file after the Undo period
or on application close. The game's legacy initial-state reference and asset
remain independent.

Named start states load only when selected explicitly. They use the same
restore and play behavior as an ordinary save file. The frontend prepares the
disks in their saved drives from matching source ADFs. It validates the
version, game/media association and committed input
settings before replacing a live session. A successful restore resumes the
saved machine with its captured hardware. A game with a legacy initial-state
reference can still launch it or use explicit normal boot.

After restoration, the frontend releases saved controls through the normal
keyboard protocol, discards obsolete host events and reconciles the endpoints
with the active session's current committed controls. The captured device
discriminator identifies the device in the emulated state. The frontend uses
the current host bindings and allows a different current attachment. Unchanged
endpoints retain their emulated phase, and changed endpoints start neutral.
The receivers retain their chip state and sampling phases. Initial states,
library saves, imported files and quicksaves share this policy.

## 3. Differential Save States (`A500DiffState`)

Differential full-machine rewind remains a design proposal. The proposed
format and remaining work are listed under [debugger work](../../../../ROADMAP.md#debugger-execution-control).

---

## 4. File loading and restoration

The file codec stores the preview PNG, each populated RAM bank and the full
Denise frame storage as individual strings in standard padded Base64. Optional
RAM banks use explicit nulls. The frame stores ARGB pixels as little-endian
32-bit words in row order, including the backing rows outside the visible
NTSC image. The reader checks bank lengths and presence against the saved
hardware and verifies the frame geometry. The preview remains an opaque PNG
payload for the frontend.

The ROM and each inserted disk store a SHA-256 digest of the exact source
bytes in lowercase hexadecimal. They can carry a display name. The frontend
uses its own authorized media sources and compares the digest after checking
the image size. A Kickstart image has 262,144 bytes. A standard ADF has
901,120 bytes. The built-in debugger ROM can be regenerated from its
deterministic boot vectors and verified against its saved digest. Exporting
a patched ROM requires a supplied source matching its current bytes.

A media reference is a JSON object with the digest (`sha256`) and display
name (`name`, or null). The ROM reference adds a `synthetic` flag. The reader
requires these properties and rejects extra properties. Each inserted drive
stores its reference in `track_data`. An attached empty
drive and a disconnected drive both carry a null disk reference and retain
their attachment state. Capture checks ADF size and hashes the source bytes
without processing the mounted tracks. After verifying the source ADF on load,
the drive prepares all its circular tracks from those bytes. The saved drive
mechanics and in-flight controller state take effect against those tracks.
Data already transferred into guest memory remains part of the saved state.
[Floppy](../Floppy.md)

The frontend validates the file, required images and committed controls before
it replaces the live machine and publishes the updated input generation.
Restoration adopts the saved configuration, RAM, ROM, CPU micro-execution
state, chip values, pending deliveries and drive mechanics. It resumes from
the saved CCK without a boot or an insertion pulse. If a source is missing
or its contents differ, the current machine and its running state stay intact.

## 5. Reference Documentation & Upstream Ground Truth

- [vAmiga Snapshot Component Implementation](../../../../ref_src/vAmiga/Core/Media/Snapshot.cpp): Snapshot serializer for Amiga hardware state, block headers, and uncompressed RAM payloads.
- [CPU State Snapshot Implementation Source](../../../../crates/cpu/src/state.rs): Living Rust `CpuState` data structures implementing Serde serialization and deserialization.

- [Pure state](../../../../crates/machine_loop/src/state.rs): Defines the core snapshot schema.
- [Save format](../../../../crates/save_state/src/save_state.rs): Implements the file envelope, JSON and PNG persistence.
