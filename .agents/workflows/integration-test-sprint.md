---
name: integration-test-sprint
description: Execute 4-iteration cascading integration verification, cluster triage, and anti-patchwork synthesis
---

# Workflow: Integration Test Sprint & Anti-Patchwork Protocol

Use this workflow to drive multi-chip integration test suites (Tier 2 `machine_loop` tests and Tier 4 `vAmigaTS` system captures) through the 4-iteration cascading protocol and the post-discovery architecture cleanup.

---

## 1. Zero-Parameter Sprint Assessment
When invoked without parameters (typing `/integration-test-sprint`):
1. **Inspect Working Tree & Git Status:**
   ```powershell
   git status
   git log -n 5 --oneline
   ```
2. **Determine Sprint Phase:**
   - **Clean working tree with new test baseline needed:** Proceed to **Iteration 1 (Discovery Sweep)**.
   - **Fixes applied to central bottlenecks:** Proceed to **Iteration 2 (Systemic Cascades)**.
   - **Targeting near-match clusters (< 1% diff):** Proceed to **Iteration 3 (Geometry & Video Convergence)**.
   - **Exploratory branches / dirty working tree:** Proceed to **Section 4: Anti-Patchwork Synthesis**.

---

## 2. Execution Runbook
Follow the operational procedure defined in [`integration-test-sprint` skill](../skills/integration-test-sprint/SKILL.md):

1. **Iteration 1 (Global Discovery Sweep):** Run baseline, filter non-runnable tests (AGA, 68020+, analog audio), group failure clusters.
2. **Iteration 2 (Low-Hanging Cascades):** Resolve central bus, interrupt mask, or beam lead bottlenecks.
3. **Iteration 3 (Convergence):** Fine-tune display/bitplane parameters with a strict **2–3 attempt cap** per cluster.
4. **Iteration 4 (Validation & Regression Lock):** Re-verify the baseline and record pass totals in [`DIARY.md`](../../DIARY.md).
5. **Anti-Patchwork Synthesis:**
   - Extract sprint diff: `git diff <baseline_commit>..HEAD > sprint_diff.patch`
   - Group changes by subsystem.
   - Find the single physical hardware law (common denominator).
   - Eradicate ad-hoc `if` conditions and unify the physical hardware model.

---

## 3. Output Contract
Conclude every sprint round or synthesis step with an executive summary:
- **Active Iteration:** `Iteration [1 | 2 | 3 | 4 | Anti-Patchwork Synthesis]`
- **Subsystem Evaluated:** `<subsystem_or_category>`
- **Pass Rate Delta:** `+X passed, -Y regressed (Total: Z / N, P%)`
- **Common Denominator Identified:** `<concise physical hardware explanation>`
- **Next Action Proposed:** `<concrete next step or commit readiness>`
