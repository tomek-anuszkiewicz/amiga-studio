---
name: roadmap-maintenance
description: Prune completed steps from ROADMAP.md, apply substrate-first renumbering, and update baseline deliverables
---

# Workflow: Roadmap Maintenance & Backlog Pruning

Use this workflow to maintain `ROADMAP.md` upon completing development tasks or milestones, strictly enforcing zero retention of completed items in Section 2 and substrate-first causal ordering.

---

## 1. Zero-Parameter Run (`/roadmap-maintenance`)
When invoked without parameters:
1. **Verify Implementation & Test Gates:**
   - Confirms that the target milestone or sub-step is 100% complete and all relevant test suites pass:
     ```powershell
     python tools/harness/pre_flight.py
     python tools/harness/run_tests.py --unit
     python tools/harness/run_tests.py --integration
     ```
2. **Purge Completed Tasks from Section 2:**
   - Completely removes completed step descriptions, bullets, and checklists from Section 2 ("Core Implementation Strategy (Remaining Milestones)").
   - Never leaves completed tags (`[COMPLETED]`, `[x]`, strikethroughs).
3. **Update Baseline Summary in Section 1 (If Major Milestone):**
   - Summarizes verified subsystem capabilities into a concise bullet in Section 1 ("Hardware Roadmap & Milestones").
4. **Renumber Remaining Steps with Substrate-First Invariant:**
   - Renumbers remaining backlog items contiguously.
   - Enforces physical hardware causality: Layer 0 (Bus/Contention) $\to$ Layer 1 (DMA Coprocessors) $\to$ Layer 2 (Display Pipeline) $\to$ Layer 3 (Peripherals) $\to$ Layer 4 (Firmware/OS).

---

## 2. Execution Runbook
Follow the operational procedure in [`.agents/skills/roadmap-maintenance/SKILL.md`](../skills/roadmap-maintenance/SKILL.md):
1. **Audit:** Identify completed steps in `ROADMAP.md`.
2. **Prune:** Delete completed blocks from Section 2.
3. **Deliverable Sync:** If milestone-level, update Section 1 baseline deliverables.
4. **Renumber:** Contiguously renumber remaining steps.
5. **Causality Check:** Validate ordering against the 5 physical substrate layers.

---

## 3. Output Contract
Conclude with the standardized summary report:
```markdown
### 🗺️ Roadmap Maintenance Summary
- **Completed Steps Purged:** <list of purged step titles>
- **Section 1 Baseline Deliverable Added/Updated:** <deliverable title or N/A>
- **Remaining Backlog Items Renumbered:** <count> active steps
- **Substrate Causality Verified:** [PASS (Layer 0 -> Layer 4)]
- **Zero Completed Tasks In Section 2:** [PASS]
```
