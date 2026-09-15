---
trigger: model_decision
description: Mandatory chronological engineering narrative logging in DIARY.md under Section 10 whenever implementing, refactoring, or modifying code or subsystems.
---

# Engineering Diary Maintenance Rule (`DIARY.md`)

This rule governs the continuous maintenance of the engineering chronicle in [`DIARY.md`](../../DIARY.md).

---

## 1. Mandatory Living Engineering Log (Section 10)

Whenever an agent implements, refactors, fixes, or modifies any code, subsystem, rule, or architectural document in this repository:
- You **must append a detailed narrative entry to [`DIARY.md`](../../DIARY.md) under Section 10 (Living Chronological Engineering Log)**.
- Format each entry with timestamp: `### [YYYY-MM-DD HH:MM CEST] — <Title>`

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

---

## 3. Standard Entry Structure

Every log entry under Section 10 must systematically document:

1. **Affected Subsystems:** Crates, modules, rules, or design notes modified.
2. **What Was Changed (The Concrete Reality):** Specific code modifications, data structures, algorithms, or mechanics introduced or refactored.
3. **Why It Was Done & Architectural Rationale:** The problem statement, edge cases discovered, user directives, and trade-offs behind the solution.
4. **Verification & Test Results:** Specific test suites executed and verified (e.g. `cargo test -p test_runner --test test_architecture_rules`, SingleStepTests, formatting checks).

---

## 3. Rationale: The Narrative History

Git commits in this project are frequently squashed, batched, or merged into higher-level commits. Relying solely on commit messages causes granular design evolution and technical decisions to be lost. `DIARY.md` serves as the permanent, living chronological chronicle and narrative history of what was actually built.

---

## 4. Milestone Diary Compaction Gate

Upon completing a major roadmap milestone in [`ROADMAP.md`](../../ROADMAP.md):
- Invoke the `compact-diary` skill ([`.agents/skills/compact-diary/`](../skills/compact-diary/SKILL.md)) to synthesize older completed milestone entries into high-level architectural digests, preserving evolutionary rationale and key decisions while keeping recent entries granular.
