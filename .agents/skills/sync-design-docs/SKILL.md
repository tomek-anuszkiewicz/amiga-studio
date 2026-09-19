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

### Step 1: Detect Drift & Staleness via Audit Engine
Run the automated design synchronization auditor to identify all specifications whose tracked Rust crates have evolved:
```powershell
python tools/harness/audit_docs_quality.py --design-sync
```
- **Within Active Tolerance ($\le 100$ commits, $\le 30$ days):** Specifications undergoing routine development without architectural changes are marked `[PASS / TOLERATED]`. Do NOT bump checkpoints or create empty commits for these.
- **Stale ($> 100$ commits or $> 30$ days without audit):** Specifications exceeding the threshold are flagged as `[STALE]`, requiring inspection and reconciliation.

Tracked specifications and their corresponding paths include:
- `crates/m68000/` $\to$ `CPU Motorola M68000.md`, `CPU Micro-Step State Machine.md`
- `crates/memory_bus/` $\to$ `MemoryBus.md`
- `crates/agnus/` $\to$ `Agnus.md`
- `crates/denise/` $\to$ `Denise.md`
- `crates/paula/` $\to$ `Paula.md`
- `crates/floppy/` $\to$ `Floppy.md`
- `crates/cia/` $\to$ `CIA.md`
- `crates/rtc/` $\to$ `RTC.md`
- `crates/joystick/`, `crates/mouse/`, `crates/keyboard/` $\to$ `Joystick.md`, `Mouse.md`, `Keyboard.md`
- `crates/machine_loop/` $\to$ `Main loop A500.md`
- `crates/gui/`, `crates/debugger/` $\to$ `GUI.md`, `GUI Specification.md`, `Debugger.md`
- `crates/config/` $\to$ `Configuration.md`
- `crates/save_state/` $\to$ `SaveState.md`

### Step 2: Inspect Crate Git Diff Since Checkpoint
For any stale or updating specification, inspect the exact code changes made to its tracked paths since the last recorded synchronization checkpoint:
```powershell
python tools/harness/audit_docs_quality.py --design-diff Denise.md
```

### Step 3: Synchronize Architectural Reality
For each affected design document:
1. Update register bitfields, timing constants, clock phase tables (`CCK1`/`CCK2`), and bus arbitration rules to match the working code.
2. Ensure hardware circuit realities, pinouts, and physical timings are clearly explained.
3. Keep technical specifications exhaustive—per rule, design specs have **no line count ceiling**.

### Step 4: Prune Draft Code & Eliminate Duplication
1. **Remove Speculative Snippets:** Delete tentative code proposals, hypothetical sketches, or superseded draft ideas written before implementation.
2. **Replace Raw Code Duplication:** Design documents must not duplicate full Rust structs or functions. Replace verbatim code blocks with concise hardware behavior descriptions, tables, and direct markdown links to the living Rust source files:
   ```markdown
   For the cycle-exact bus arbitration implementation, see [`MemoryBus::step_cck`](../../../crates/memory_bus/src/memory_bus.rs).
   ```

### Step 5: Update Crate Dependency Mermaid Graph (If Dependencies Changed)
If crate dependencies were added or altered in any `Cargo.toml`:
1. Open [`Obsidian/Amiga/Design/General Architecture.md`](../../../Obsidian/Amiga/Design/General%20Architecture.md).
2. Locate Section 2 (`Workspace Crate Architecture & Dependencies`).
3. Update the Mermaid `graph TD` diagram to accurately depict current crate relationships and dependencies.

### Step 6: Validate Links & Graph Integrity
Verify YAML frontmatter properties and ensure zero broken links:
```powershell
cargo test -p test_runner --test test_architecture_rules -- test_obsidian_design_docs_links_integrity
```

### Step 7: Stamp the Git Checkpoint & Commit Atomically
Once the specification is substantively updated (or formally verified after exceeding the 100-commit / 30-day threshold), stamp its frontmatter checkpoint to HEAD:
```powershell
python tools/harness/audit_docs_quality.py --design-bump Denise.md
```
**Strict Prohibition of Empty Sync Commits:**
- Stage and commit the bumped document **together with its substantive documentation improvements**.
- Never create standalone commits that only modify `last_synced_commit`.

---

## 3. Standard Sync Report Format

```markdown
### 📚 Design Documentation Sync Report
- **Documents Updated & Bumped:**
  | Design Document | Subsystem / Topic | Changes Applied | Checkpoint Bumped |
  | :--- | :--- | :--- | :--- |
  | `Obsidian/Amiga/Design/<doc>.md` | `<subsystem>` | Synchronized registers, timing tables | `HEAD` (`<hash>`) |
- **Pruned Draft / Redundant Content:**
  - `<doc>.md`: Removed speculative code snippets.
- **Mermaid Graph Status:** [UPDATED | UNCHANGED]
- **Vault Link Integrity:** `test_obsidian_design_docs_links_integrity` passed (0 broken links).
- **Design Specs Sync Status:** All tracked specifications in sync with HEAD.
```

