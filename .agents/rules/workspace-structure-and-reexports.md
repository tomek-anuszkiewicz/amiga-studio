# Workspace Boundaries and Re-Exports

- Keep crates flat under `crates/*`; express ownership through Rust APIs and re-exports, not nested crate directories.
- The top-level machine facade exposes peers for host frontends. Shared `config` is foundational and is not re-exported as a peer's owned child.
- Peer subsystems are owned side by side by the machine and never re-export one another merely because they collaborate.
- Contained chipset engines are modules of their owning crate: Copper, Blitter and DMA belong to Agnus; sprites to Denise; audio and interrupts to Paula; mouse and joystick to game_ports. Keep source files flat, with a subsystem prefix for supporting files (for example `blitter_area.rs`). Expose the subsystem namespace and useful primary types explicitly, without glob re-exports. Retained child crates such as frame_builder remain re-exported by their owner.
- Each library root is `src/<crate_directory>.rs`, configured in Cargo.toml. Generic `lib.rs` files are prohibited in workspace crates and helper tools.
- Follow the neighboring facade and module export pattern.
- Structural refactors update all consumers without stale aliases or compatibility wrappers per [Compatibility and Internal Refactoring](../../AGENTS.md#compatibility-and-internal-refactoring).

Read [General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md) and [Configuration](../../Obsidian/Amiga/Design/Configuration.md) for ownership and dependencies. The [refactor-split-module](../skills/refactor-split-module/SKILL.md) procedure links [workspace examples](../skills/refactor-split-module/references/workspace-layout.md).
