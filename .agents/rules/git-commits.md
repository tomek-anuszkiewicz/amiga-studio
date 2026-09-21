---
trigger: always_on
description: Mandatory immediate atomic Git commit after every completed task, zero accumulated uncommitted changes, Conventional Commits, and pre-commit test gates.
---

# Git Commits & Immediate Atomic History Protocol

This rule governs standard Git commit creation, mandatory immediate commits after every discrete task, atomic commit decomposition, context reconstruction, and Conventional Commit standards across the repository.

---

## 1. Rule Mandate & Immediate Commit Protocol

1. **Mandatory Post-Task Commit Rule (Zero Dirty Working Trees Across Turns)**:
   - Every discrete change, refactoring, bug fix, new capability, or documentation update **must conclude with an immediate, verified Git commit before completing the turn**.
   - The agent **must never leave uncommitted changes sitting in the working tree** across conversational turns or accumulate multiple unrelated modifications into delayed bulk commits.
   - Leaving uncommitted changes leads to context loss across compaction, broken intermediate states, and brittle merge conflicts. Commit early, commit atomically.

2. **No Blind Bulk Staging Across Disparate Concerns**:
   - The agent **must never default to blind bulk staging (`git add -A; git commit -m "..."`)** if changes span multiple distinct architectural concerns.
   - Follow the 4-step pipeline: **Inspect $\to$ Decompose $\to$ Format $\to$ Verify**.

---

## 2. Context Reconstruction (When Changes Accumulate)

When multiple changes have accumulated across files or over an extended pair-programming session:

1. **Inspect Working Tree Status:**
   - Execute `git status` and `git diff --stat` to review all modified, staged, and untracked files.
2. **Reconstruct Context from DIARY.md & Session History:**
   - Review recent entries in [DIARY.md](../../DIARY.md) (Section 10) and the conversation transcript.
   - `DIARY.md` records chronological modifications, technical rationales, and affected subsystems. Use these records to group files into logical, self-contained units.

---

## 3. Atomic Commit Decomposition Strategy

Separate accumulated changes into distinct, focused commits using targeted staging (`git add <file1> <file2>`):

### A. Typical Independent Commit Categories:
1. **Design Documentation & Knowledge Base:**
   - Changes to `Obsidian/Amiga/Design/*.md` (e.g. frontmatter properties, architectural specs, diagrams).
   - Commit type: `docs(obsidian): ...` or `docs(design): ...`
2. **Agent Rules, Skills & Workflows:**
   - Changes to `.agents/rules/*.md`, `.agents/skills/`, `.agents/workflows/`, or `AGENTS.md`.
   - Commit type: `chore(rules): ...` or `docs(rules): ...`
3. **Core Subsystem Implementation & Bug Fixes:**
   - Changes to `crates/cpu/`, `crates/memory_bus/`, `crates/agnus/`, etc.
   - Commit type: `feat(<subsystem>): ...` or `fix(<subsystem>): ...` or `refactor(<subsystem>): ...`
4. **Architecture Guardrails & Test Harnesses:**
   - Changes to `crates/test_runner/tests/test_architecture_rules.rs` or shared testing fixtures.
   - Commit type: `test(arch): ...` or `test(<subsystem>): ...`

### B. The Cohesive Unit Exception (Keep Together):
- When an implementation change, its dedicated unit test, and its corresponding design specification update belong to the **exact same discrete feature or bug fix**, stage and commit them together in a single atomic commit to maintain repository integrity and bisectability.

---

## 4. Conventional Commits Standard

All commit messages must be written in **strict English** per [`.agents/rules/language-policy.md`](language-policy.md) and adhere to Conventional Commits:

### Format Schema:
```text
<type>(<scope>): <concise imperative summary in English>

- <Concrete change 1: what was added, modified, or removed>
- <Concrete change 2: technical rationale or trade-off>
- <Verification: passed test suites or compliance checks>
```

### Types & Scopes:
- **Types:** `feat` (new capability), `fix` (bug fix), `refactor` (code restructuring with identical behavior), `perf` (cycle or memory optimization), `docs` (specifications and guides), `test` (test suites), `chore` (maintenance, CI, rule updates).
- **Scopes:** Concrete subsystem or domain: `m68000`, `memory_bus`, `agnus`, `denise`, `paula`, `cia`, `gui`, `debugger`, `obsidian`, `arch`, `rules`.

---

## 5. Pre-Commit Quality Verification Gate

Never commit to any branch (especially `master`) without running the repository quality gate:

1. **Pre-Flight Quick Gate:**
   ```powershell
   python tools/harness/pre_flight.py --quick
   ```
   *Verifies:* code formatting (`cargo fmt`), `AGENTS.md` size ceiling, test coupling in `tests/`, public API coverage, Clippy invariants, Hardware Quality (Pillars 1 & 2), and Code Quality (Pillars 1 & 2).
2. **Automated Architecture Tests:**
   ```powershell
   cargo test -p test_runner --test test_architecture_rules
   ```
3. **Subsystem Test Suites:**
   - Execute relevant unit/integration tests for the modified crates before committing.

> [!NOTE]
> Routine commits do not require diary logging, design documentation updates, or roadmap pruning. Those occur strictly upon completing minor roadmap points or milestones.

---

## 6. On-Demand Review Workflow: `/code-review`

Run [`/code-review`](../workflows/code-review.md) only when explicitly requested by the user or before a major architectural branch merge. **Not required** for routine commits or minor roadmap milestones.
