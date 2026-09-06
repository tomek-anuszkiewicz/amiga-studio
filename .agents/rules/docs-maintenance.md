# Documentation Maintenance Rule

Whenever implementing, refactoring, or modifying any subsystem in this repository:

- **Mandatory Final Step (Definition of Done):** You must update the corresponding design document in [Obsidian/Amiga/Design](../../Obsidian/Amiga/Design) whenever any architectural decision, timing model, data structure, or hardware quirk has changed or was clarified.
- **Crate Dependency Graph Maintenance:** Whenever crates or dependencies in `Cargo.toml` (new crates, added/removed inter-crate dependencies, or key external dependencies) are modified, you **must update the Crate Dependency Mermaid Graph in [Obsidian/Amiga/Design/General Architecture.md](../../Obsidian/Amiga/Design/General%20Architecture.md#2-workspace-crate-architecture--dependencies)**.
- The design specifications in `Obsidian/Amiga/Design/` are living, permanent references and must always stay synchronized with the active code.
