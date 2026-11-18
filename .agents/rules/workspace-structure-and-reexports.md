# Workspace boundaries and re-exports

Load before creating/reorganizing crates, public exports or CPU instruction modules.
Read affected ownership requirements through
[General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md) and
[Configuration](../../Obsidian/Amiga/Design/Configuration.md).

- Crates remain flat under `crates/*`. Express ownership through APIs and explicit
  re-exports. Each library root is `src/<crate_directory>.rs`, declared in Cargo.toml;
  generic `lib.rs` files are prohibited in workspace crates and helper tools.
- The machine facade exposes peer subsystems side by side for frontends. Peers do
  not re-export each other merely because they collaborate. Shared `config` is
  foundational, not a peer's owned child.
- Contained engines are modules of their owner: Copper, Blitter and DMA in Agnus;
  sprites in Denise; audio/interrupts in Paula; mouse/joystick in game_ports.
  Keep source files flat with subsystem prefixes for supporting files. Expose
  subsystem namespaces and useful primary types explicitly, without glob exports.
  Retained child crates such as frame_builder remain re-exported by their owner.
- CPU instruction files remain flat in `crates/cpu/src/instructions/`, one mnemonic
  per file; no subdirectories or umbrellas combining distinct mnemonics. Oversized
  instruction exceptions remain flat. Exact grouping exceptions are
  `move_sr_ccr.rs`, `logic_sr_ccr.rs`, size-based `move_b.rs` / `move_w.rs` /
  `move_l.rs`, and condition families `bcc.rs` / `dbcc.rs` / `scc.rs`.

## Compatibility and Internal Refactoring

Internal APIs, schemas, configuration and generated formats have no default
backward-compatibility requirement. Workspace consumers are maintained together;
update affected callers, imports, fixtures and tests, then remove replaced code,
aliases, forwarders, re-exports, duplicate namespaces and legacy fallbacks in the
same change. Do not add migrations, adapters, deprecated interfaces or parallel
implementations for hypothetical consumers. Until first release, snapshots need
no backward compatibility. Preserve compatibility only for an explicit request or
a documented external contract.

Preserve external specifications and user data. Explain impact and obtain direction
before removing external-format support or deleting user data that cannot be
recreated. Before completion, search active code/tests for replaced symbols and
mechanisms, assess each remaining reference and run relevant checks. Before an
authorized commit, also reconcile documentation, instructions and executable
examples: history must identify unavailable capabilities; future work must state
prerequisites. An explicit documentation-preservation request takes precedence;
report that debt. Detailed cohesion/visibility assessment belongs to
[architecture criteria](../references/architecture-criteria.md).
