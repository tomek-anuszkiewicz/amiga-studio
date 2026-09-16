---
name: roadmap-maintenance
description: Audit, prune completed steps, and synchronize ROADMAP.md with zero retention of completed items.
---

# Recipe: Roadmap Maintenance & Backlog Pruning

This skill provides the mandatory procedure for maintaining and pruning [`ROADMAP.md`](../../../ROADMAP.md) per [`.agents/rules/roadmap-maintenance.md`](../../rules/roadmap-maintenance.md).

---

## 1. When to Use This Skill

Activate this skill whenever:
- Finishing any task, milestone, or sub-step documented in [`ROADMAP.md`](../../../ROADMAP.md).
- Running the `/code-review` workflow to verify that [`ROADMAP.md`](../../../ROADMAP.md) contains zero completed task descriptions.
- Re-aligning project priorities and renumbering remaining backlog steps.

---

## 2. Core Principle: Zero Retention of Completed Backlog Items

[`ROADMAP.md`](../../../ROADMAP.md) is strictly a **forward-looking backlog** of pending and active work. It is NOT a historical changelog or execution archive:

- **Strict Prohibition of Completed Tags:**
  - Never mark tasks with `[COMPLETED]`, `[Completed: ...]`, `[x]`, or strikethrough (`~~...~~`).
  - Leaving completed tasks or detailed implementation checklists sitting in Section 2 clutters the backlog, wastes context tokens, and obscures active priorities.
- **Mandatory Complete Deletion:**
  - When an item, step, or milestone is 100% completed and verified (all tests pass), **completely remove and delete its text** from Section 2 ("Core Implementation Strategy (Remaining Milestones)").
- **Separation of Concerns:**
  - **Granular Historical Detail:** Belongs in [`DIARY.md`](../../../DIARY.md) (Section 10) and Git commit history.
  - **Active Backlog:** Belongs in Section 2 of [`ROADMAP.md`](../../../ROADMAP.md) (strictly uncompleted items).
  - **Verified Capabilities:** When a major milestone is completed, summarize its capabilities into a concise 1-paragraph or high-level bullet under Section 1 ("Hardware Roadmap & Milestones / Baseline Deliverables").

---

## 3. Step-by-Step Execution Workflow

### Step 1: Verify Full Implementation & Testing
Ensure the task or milestone is genuinely complete:
1. All relevant unit, integration, and architecture tests pass.
2. Code formatting passes (`cargo fmt --all -- --check`).
3. Relevant design specifications under `Obsidian/Amiga/Design/` are updated.

### Step 2: Delete Completed Steps from Section 2
1. Open [`ROADMAP.md`](../../../ROADMAP.md).
2. Locate the completed step or sub-step under Section 2 ("Core Implementation Strategy (Remaining Milestones)").
3. **Delete the entire block** (title, description, sub-bullets, and test references).

### Step 3: Update Baseline Summary in Section 1 (If Major Milestone)
If the completed work represents an architectural milestone or major subsystem capability:
1. Locate Section 1 ("Hardware Roadmap & Milestones") under the active phase (e.g. "Phase 1: Baseline Amiga 500").
2. Add or update a concise baseline bullet summarizing the deliverable and key verified capabilities (e.g., supported modes, test coverage).
3. Keep baseline entries concise and high-level—avoid pasting verbose task breakdowns.

### Step 4: Renumber & Reorder Remaining Steps (Substrate-First Invariant)
1. Renumber remaining steps and sub-steps in Section 2 so that numbering remains contiguous.
2. **Apply the Substrate-First Invariant:** Ensure remaining tasks strictly follow physical hardware causality rather than folder taxonomy:
   - Layer 0: Bus Arbitration, Clock Phases, Refresh, Contention & Stalls
   - Layer 1: Autonomous Coprocessors & DMA Channels (Copper, Blitter)
   - Layer 2: Video Serializer & Display Pipeline (Denise, DIW/DDF, Sprites)
   - Layer 3: Peripherals & External I/O (Paula Audio/Floppy, CIAs, Ports)
   - Layer 4: System Integration & Firmware Exec
3. Ensure the step marked `[Active Focus]` accurately reflects the immediate next task.

### Step 5: Synchronize Verification Scorecard (If Applicable)
If a sub-suite milestone was finished or global timing changed:
1. Update [`Obsidian/Amiga/Design/vAmigaTS Verification Scorecard.md`](../../../Obsidian/Amiga/Design/vAmigaTS%20Verification%20Scorecard.md) with updated test counts and pass rates.
2. Ensure `ROADMAP.md` only links to the scorecard rather than containing transient test run percentages.

### Step 6: Verify Cleanliness & Attractor Discipline
1. Search [`ROADMAP.md`](../../../ROADMAP.md) for any stray occurrences of `COMPLETED` or `Completed:` in Section 2:
   Ensure zero matches in the active backlog.
2. Run the attractor linter:
   ```powershell
   python .agents/skills/attractor-discipline/scripts/lint_attractors.py
   ```
