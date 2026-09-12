---
name: sync-design-docs
description: Synchronize Obsidian design specifications with active codebase changes, prune draft code, and update crate graph.
---

# Recipe: Design Documentation & Codebase Synchronization

This skill provides the operational procedure for maintaining, pruning, and synchronizing architectural specifications under [Obsidian/Amiga/Design/](../../../Obsidian/Amiga/Design/) with active Rust codebase changes per [`.agents/rules/docs-maintenance.md`](../../rules/docs-maintenance.md).

---

## 1. When to Use This Skill

Activate this skill whenever:
- Implementing, refactoring, or completing any subsystem, opcode, or bus mechanism.
- Modifying crate dependencies in `Cargo.toml` (adding/removing crates or inter-crate dependencies).
- Concluding a roadmap milestone or step in [`ROADMAP.md`](../../../ROADMAP.md).
- Removing obsolete speculative code sketches or draft proposals from documentation.

---

## 2. Step-by-Step Execution Workflow

### Step 1: Identify Changed Subsystems from Git Diff
Inspect recent changes to determine which specifications are affected:
```powershell
git diff --name-only HEAD~1
```
Map modified crates to their corresponding design specifications:
- `crates/m68000/` $\to$ `CPU Motorola M68000.md`, `CPU Micro-Step State Machine.md`, `CPU Instructions Catalog.md`
- `crates/memory_bus/` $\to$ `MemoryBus.md`, `Gary.md`
- `crates/agnus/`, `crates/copper/`, `crates/blitter/`, `crates/dma/` $\to$ `Agnus.md`, `Copper.md`, `Blitter.md`, `DMA.md`
- `crates/denise/`, `crates/sprites/` $\to$ `Denise.md`, `Sprites.md`
- `crates/paula/`, `crates/audio/`, `crates/floppy/`, `crates/serial_port/` $\to$ `Paula.md`, `Audio.md`, `Floppy.md`
- `crates/cia/` $\to$ `CIA.md`
- `crates/gui/`, `crates/debugger/` $\to$ `GUI.md`, `Debugger.md`
- Workspace root / `Cargo.toml` $\to$ `General Architecture.md`

### Step 2: Synchronize Architectural Reality
For each affected design document:
1. Update register bitfields, timing constants, clock phase tables (`CCK1`/`CCK2`), and bus arbitration rules to match the working code.
2. Ensure hardware circuit realities, pinouts, and physical timings are clearly explained.
3. Keep technical specifications exhaustive—per rule, design specs have **no line count ceiling**.

### Step 3: Prune Draft Code & Eliminate Duplication
1. **Remove Speculative Snippets:** Delete tentative code proposals, hypothetical sketches, or superseded draft ideas written before implementation.
2. **Replace Raw Code Duplication:** Design documents must not duplicate full Rust structs or functions. Replace verbatim code blocks with concise hardware behavior descriptions, tables, and direct markdown links to the living Rust source files:
   ```markdown
   For the cycle-exact bus arbitration implementation, see [`MemoryBus::step_cck`](file:///d:/Programowanie/Amiga/crates/memory_bus/src/lib.rs).
   ```

### Step 4: Update Crate Dependency Mermaid Graph (If Dependencies Changed)
If crate dependencies were added or altered in any `Cargo.toml`:
1. Open [`Obsidian/Amiga/Design/General Architecture.md`](../../../Obsidian/Amiga/Design/General%20Architecture.md).
2. Locate Section 2 (`Workspace Crate Architecture & Dependencies`).
3. Update the Mermaid `graph TD` diagram to accurately depict current crate relationships and dependencies.

### Step 5: Validate Links & Graph Integrity
Verify YAML frontmatter properties and ensure zero broken links:
```powershell
cargo test -p test_runner --test test_architecture_rules -- test_obsidian_design_docs_links_integrity
```

---

## 5. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Medium`
- **Context Savings:** Isolates scanning across 28 Obsidian documents, git diff parsing, and Mermaid graph syntax verification from the main conversation.
- **Subagent Task Template:**
  - `TaskName`: "Syncing Design Specifications"
  - `TaskSummary`: "Audits recent git commits/diff, updates corresponding Obsidian design documents, prunes draft code, and validates links."
  - `Prompt`:
    ```markdown
    Synchronize Obsidian design documentation with recent code changes.
    Follow .agents/skills/sync-design-docs/SKILL.md:
    1. Inspect git diff: `git diff HEAD~1` (or staged changes).
    2. Identify affected design docs in `Obsidian/Amiga/Design/`.
    3. Update hardware registers, timings, and tables to match code.
    4. Prune speculative draft sketches and duplicate Rust code.
    5. Update Mermaid crate graph in `General Architecture.md` if dependencies changed.
    6. Verify link integrity: `cargo test -p test_runner --test test_architecture_rules -- test_obsidian_design_docs_links_integrity`.
    7. Return strictly the Design Docs Sync Report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### 📚 Design Documentation Sync Report
  - **Documents Updated:**
    | Design Document | Subsystem / Topic | Changes Applied |
    | :--- | :--- | :--- |
    | `Obsidian/Amiga/Design/<doc>.md` | `<subsystem>` | Synchronized registers, timing tables |
  - **Pruned Draft / Redundant Content:**
    - `<doc>.md`: Removed speculative code snippets (L<start>-L<end>).
  - **Mermaid Graph Status:** [UPDATED | UNCHANGED]
  - **Vault Link Integrity:** `test_obsidian_design_docs_links_integrity` passed (0 broken links).
  ```
