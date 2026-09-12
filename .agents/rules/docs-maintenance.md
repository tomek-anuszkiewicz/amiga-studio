---
trigger: model_decision
description: Maintaining and synchronizing technical design documentation under Obsidian/Amiga/Design/ with active emulator code and architecture.
---

# Design Documentation Maintenance & Code Synchronization Rule

This rule governs the continuous synchronization, cleanup, and maintenance of technical design documentation under [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/).

---

## 1. Living Design Synchronization

Whenever implementing, refactoring, or modifying any subsystem in this repository:
- You **must update the corresponding design document in [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/)** whenever any architectural decision, timing model, data structure, register bitfield, or hardware quirk has changed or was clarified.
- The design specifications in `Obsidian/Amiga/Design/` are living, permanent specifications and must always stay synchronized with the active code.

---

## 2. Post-Implementation Cleanup & Code Duplication Removal

Design specifications often contain tentative draft snippets, forward-looking proposals, or hypothetical code sketches written prior to implementation:
- Whenever completing a roadmap step or implementing a feature, you **must review and clean up the relevant design documents**:
  1. Remove obsolete speculative code and draft proposals.
  2. Ensure the document reflects the finalized, living architectural reality.
  3. **Design documents must never duplicate code that has already been written**: replace duplicate Rust code blocks with concise architectural descriptions, tables, and direct markdown links to living Rust source files.

---

## 3. No Line Limits on Technical Documentation

Design specifications and reference manuals in [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design/) have **no line count limits**. They should be as long, exhaustive, and detailed as necessary to serve as complete, living single-source-of-truth specifications. Never artificially split, truncate, or omit architectural details from markdown documentation.

---

## 4. Crate Dependency Graph Maintenance

Whenever crates or dependencies in `Cargo.toml` (new crates, added/removed inter-crate dependencies, or key external dependencies) are modified:
- You **must update the Crate Dependency Mermaid Graph in [`Obsidian/Amiga/Design/General Architecture.md`](../../Obsidian/Amiga/Design/General%20Architecture.md#2-workspace-crate-architecture--dependencies)**.

---

## 5. Knowledge Graph & Linking Compliance

All modifications to design specifications must adhere strictly to the YAML frontmatter properties, dual-layer linking, and inverted pyramid structure codified in [`.agents/rules/vault-linking-and-graph-integrity.md`](vault-linking-and-graph-integrity.md).
