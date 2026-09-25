---
trigger: model_decision
description: Log completed minor roadmap points and major milestones in DIARY.md; never log routine micro-commits.
---

# Engineering Diary Maintenance Rule (`DIARY.md`)

This rule governs milestone entries in the engineering chronicle in [`DIARY.md`](../../DIARY.md).

---

## 1. Milestone-Only Engineering Log (Section 10)

Append an entry to [`DIARY.md`](../../DIARY.md) under Section 10 only after a minor roadmap point (for example, Step 1.1 or 1.2) or a major milestone has been completed and verified. Summarize the completed point or milestone, including the relevant work since its previous diary entry.

Routine micro-commits, including intermediate fixes, refactors, tests, rule changes, and documentation edits, **must not create DIARY.md entries**. A completed task or Git commit alone is not a diary trigger.

Format each milestone entry with timestamp: `### [YYYY-MM-DD HH:MM CEST] — <Title>`.

---

## 2. Deterministic Logging Tool (`tools/harness/log_diary.py`)

To prevent context bloat and eliminate reading the 90+ KB `DIARY.md` file into context:
- **Recommended Workflow:** Always use the deterministic CLI tool:
  ```powershell
  python tools/harness/log_diary.py \
    --title "<Title>" \
    --subsystems "<crates/..., rules/...>" \
    --changes "<bullet 1>; <bullet 2>" \
    --rationale "<rationale>" \
    --results "<test verification>"
  ```
- The tool automatically computes the current timestamp (`### [YYYY-MM-DD HH:MM CEST] — <Title>`), formats all 4 required sections, and appends the entry cleanly in 0.05s without prompt overhead.
- **Milestone Lifecycle & Git Staging Order:**
  - Execute `log_diary.py` only after the minor roadmap point or major milestone passes its completion gates, and before the milestone-completion commit.
  - Stage `DIARY.md` with the milestone completion changes under the **Cohesive Unit** protocol in [`.agents/rules/git-commits.md`](git-commits.md). Earlier routine micro-commits must not include diary entries.
  - Do not leave the required milestone entry for an orphan trailing `docs(diary): ...` commit.

---

## 3. Standard Entry Structure

Every log entry under Section 10 must systematically document:

1. **Affected Subsystems:** Crates, modules, rules, or design notes modified.
2. **What Was Changed (The Concrete Reality):** Specific code modifications, data structures, algorithms, or mechanics introduced or refactored.
3. **Why It Was Done & Architectural Rationale:** The problem statement, edge cases discovered, user directives, and trade-offs behind the solution.
4. **Verification & Test Results:** Specific test suites executed and verified (e.g. `cargo test -p test_runner --test test_architecture_rules`, SingleStepTests, formatting checks).

---

## 4. Rationale: The Narrative History

Git commits record routine changes. `DIARY.md` records the verified outcome, technical decisions, and evidence for each completed roadmap point or milestone, so the engineering history remains readable without duplicating every commit.

---

## 5. Milestone Diary Compaction Gate

Upon completing a major roadmap milestone in [`ROADMAP.md`](../../ROADMAP.md):
- Invoke the `compact-diary` skill ([`.agents/skills/compact-diary/`](../skills/compact-diary/SKILL.md)) to synthesize older completed milestone entries into high-level architectural digests, preserving evolutionary rationale and key decisions while keeping recent entries granular.
