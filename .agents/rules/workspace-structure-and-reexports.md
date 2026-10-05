# Workspace Boundaries and Re-Exports

- Keep crates flat under `crates/*`; express ownership through Rust APIs and re-exports, not nested crate directories.
- The top-level machine facade exposes peers for host frontends. Shared `config` is foundational and is not re-exported as a peer's owned child.
- Peer subsystems are owned side by side by the machine and never re-export one another merely because they collaborate.
- A parent subsystem re-exports its contained child crate and useful primary types: RTC belongs to memory_bus; Copper/Blitter to Agnus; audio components to Paula. Use explicit namespaces/types rather than glob re-exports.
- Each library root is `src/<crate_directory>.rs`, configured in Cargo.toml. Generic `lib.rs` files are prohibited in workspace crates and helper tools.
- Curate the root API; worker modules remain private or crate-visible. Minimum item visibility belongs to [rust-best-practices.md](rust-best-practices.md).
- Structural refactors update all consumers without stale aliases or compatibility wrappers per [clean-break-refactoring.md](clean-break-refactoring.md).

Read [General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md) and [Configuration](../../Obsidian/Amiga/Design/Configuration.md) for ownership and dependencies. The [refactor-split-module](../skills/refactor-split-module/SKILL.md) procedure links [workspace examples](../skills/refactor-split-module/references/workspace-layout.md).
