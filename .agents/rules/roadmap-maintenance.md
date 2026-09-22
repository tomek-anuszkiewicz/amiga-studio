---
trigger: model_decision
description: Mandatory updating and pruning of ROADMAP.md whenever an active milestone or step is completed and verified.
---

# Roadmap Maintenance & Milestone Completion Rule (`ROADMAP.md`)

This rule governs the continuous synchronization, pruning, substrate-first ordering, and milestone completion workflow for [`ROADMAP.md`](../../ROADMAP.md).

---

## 1. Step Completion & Active Backlog Pruning

Whenever an agent is 100% certain that a roadmap milestone or step in [`ROADMAP.md`](../../ROADMAP.md) has been fully implemented and verified (all unit, integration, and architecture tests pass):
- As part of that **same task/commit**, you **must update [`ROADMAP.md`](../../ROADMAP.md)** following the [`roadmap-maintenance`](../skills/roadmap-maintenance/SKILL.md) skill:
  1. **Zero Retention of Completed Items:** Never mark tasks with `[COMPLETED]`, `[Completed: ...]`, `[x]`, or strikethrough. Do NOT retain completed tasks or verbose implementation breakdowns in Section 2.
  2. **Completely Delete Finished Tasks:** Delete the completed task, step, or sub-step text from Section 2 ("Core Implementation Strategy (Remaining Milestones)"). The active roadmap must strictly reflect only pending and in-progress work.
  3. **Update Concise Baseline Summary:** If the completed work represents an architectural milestone or major subsystem capability, update the concise completed baseline summary in Section 1 with high-level verified capabilities.
  4. **Renumber and Reorder Remaining Steps:** Keep remaining steps and sub-steps sequentially numbered and contiguous.
  5. **Historical Logging in DIARY.md:** Granular execution narratives, files touched, and technical rationales belong strictly in [`DIARY.md`](../../DIARY.md) (Section 10) and Git commit history, never in [`ROADMAP.md`](../../ROADMAP.md).

---

## 2. Minor Roadmap Point & Major Milestone Completion Gates

Whenever completing a minor roadmap step (e.g. Step 1.1, 1.2, 2.1) or major milestone block:

1. **Milestone Quality Gate (`pre_flight.py --milestone`):**
   - Run `python tools/harness/pre_flight.py --milestone` to verify all 11 quality gates (Clippy, Architecture Rules, Code Quality Pillars 1–4, Hardware Quality Pillars 1–5, and Docs Quality 10 pillars).
2. **Semantic Parity Audit (`audit-semantic-parity`):**
   - Run `/audit-semantic-parity` on modified subsystems to verify forward and reverse parity against living design specs.
3. **Design Documentation Synchronization:**
   - Update affected design documents under `Obsidian/Amiga/Design/` and stamp Git checkpoints (`audit_docs_quality.py --design-bump <doc>`) per [`docs-maintenance.md`](docs-maintenance.md).
4. **Milestone Engineering Diary Logging:**
   - Record the completed roadmap step in [`DIARY.md`](../../DIARY.md) (Section 10) using `python tools/harness/log_diary.py`.
5. **Major Milestone Compaction (`compact-diary`):**
   - Upon concluding full milestone phases, invoke the `compact-diary` skill ([`.agents/skills/compact-diary/`](../skills/compact-diary/SKILL.md)) to synthesize older completed milestone entries into high-level architectural digests.

---

## 3. The Substrate-First Invariant in Planning & Roadmaps

When planning new features, decomposing verification test suites, or structuring roadmap milestones:
- **Strict Prohibition of Folder-Tree Taxonomic Bias:** Never order tasks or test suites based on naive filesystem directory listings (e.g. alphabetical order of test folders in `ref_src/vAmigaTS/`).
- **Physical Electronic Causality (Substrate-First Invariant):** Tasks and milestones must strictly follow the physical hardware causality chain:
  1. **Layer 0: Bus Arbitration & Clock Synchronization:** CCK phase timing (`CCK1`/`CCK2`), even/odd slot allocation, DRAM refresh, DMA arbitration, wait-state assertions, and open bus.
  2. **Layer 1: Autonomous Coprocessors & DMA Masters:** Agnus Copper & Blitter engines (which request and consume bus slots).
  3. **Layer 2: Video Serializer & Display Pipeline:** Denise bitplane shifters, DIW/DDF window clipping, sprite multiplexing, and color DACs.
  4. **Layer 3: Peripheral Controllers & External I/O:** Paula audio/floppy/UART, CIAs (timers, ICR, TOD), keyboard, joystick/mouse ports.
  5. **Layer 4: System Integration & Firmware Exec:** Kickstart ROM bootblock, Exec library, autovectors, and OS intuition.
- **Rationale:** Debugging higher-level coprocessors (Layer 1/2) while the underlying bus arbiter (Layer 0) has cycle allocation defects creates phantom anomalies and circular debugging churn.

---

## 4. Verification Scorecard Synchronization Protocol

To maintain continuous ground-truth visibility without polluting `ROADMAP.md` or git history with transient pass rates:
- Ground-truth pass rates for regression test suites (vAmigaTS) are maintained in [`Obsidian/Amiga/Design/vAmigaTS Verification Scorecard.md`](../../Obsidian/Amiga/Design/vAmigaTS%20Verification%20Scorecard.md).
- **Mandatory Refresh Triggers:** The scorecard must be refreshed only upon:
  1. Completion of any roadmap sub-suite step (e.g., finishing Step 2.1, 2.2, etc.).
  2. Merging a major cross-subsystem timing fix that alters global pass rates.
  3. Explicit user instruction to audit current verification status.
- Never record transient, intermediate debug run pass rates into `ROADMAP.md`.
