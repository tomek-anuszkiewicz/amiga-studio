---
name: compact-diary
description: Milestone synthesis and compaction of historical chronological entries in DIARY.md
---

# Workflow: Compact Engineering Diary

Use this workflow to synthesize and compact historical chronological entries in `DIARY.md` (Section 10) upon major milestone completion, preserving architectural insights while preventing token bloat.

---

## 1. Zero-Parameter Run (`/compact-diary`)
When invoked without parameters:
1. **Assess Diary Size & Scope:**
   - Evaluates current line count and file size of `DIARY.md`.
   - Identifies entries belonging to the currently active or most recently completed milestone vs. older settled milestones.
2. **Synthesize Older Milestones into Milestone Digests:**
   - Consolidates older, settled entries into cohesive digests covering:
     - Scope & Date Range (e.g. `[Phase 1 Baseline CPU: Batches 1.1–1.11]`).
     - Architectural Breakthroughs (e.g. 2-clock micro-step slicing, contention modeling).
     - Key Decisions & Non-Obvious Trade-Offs (preserving the *why*).
     - Agent Collaboration & Root-Cause Lessons (subtle traps and permanent safeguards).
     - Final Verification Gates.
3. **Preserve Active Milestone Detail:**
   - Retains granular chronological records for the active milestone.
4. **Verify Formatting & Invariants:**
   - Runs pre-flight checks and architecture rules.

---

## 2. Execution Runbook
Follow the operational procedure in [`.agents/skills/compact-diary/SKILL.md`](../skills/compact-diary/SKILL.md):
1. **Extraction:** Read older entries targeted for compaction.
2. **Distillation:** Extract structural breakthroughs, non-obvious lessons, and architectural trade-offs while discarding micro-diffs.
3. **Restructuring:** Replace verbose individual logs with the synthesized Milestone Digest block.
4. **Verification:** Confirm zero loss of non-obvious domain knowledge.

---

## 3. Output Contract
Conclude with the standardized summary report:
```markdown
### 📔 Diary Compaction Summary
- **Target Milestones Compacted:** `<milestone_name_and_dates>`
- **Active Entries Retained:** <count> granular entries
- **Original Size:** <lines_before> lines (<kb_before> KB)
- **Compacted Size:** <lines_after> lines (<kb_after> KB)
- **Net Token Reduction:** <percent>% reduction
- **Key Architectural Digests Preserved:** <bullet list of digest sections>
- **Verification:** `pre_flight.py` (PASS)
```
