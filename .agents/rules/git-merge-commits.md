---
trigger: model_decision
description: Git branch merging, conflict resolution with mandatory merge commits, and Git worktree lifecycle management.
---

# Git Branch Merging, Worktrees & Conflict Resolution Protocol

This rule governs Git branch reintegration, Git worktree lifecycles, and conflict resolution standards across the repository.

---

## 1. Core Branch Reintegration Principles

When reintegrating a development branch or Git worktree back into `master` (or between long-lived branches):

### A. Clean Merges (Zero Conflicts) $\to$ Fast-Forward Allowed
- When `master` has not diverged or when changes merge cleanly without conflicts, **fast-forward merges (`git merge --ff-only` or standard fast-forward)** are permitted and preferred.
- Preserves a clean, linear commit graph without unnecessary empty merge bubbles.

### B. Conflicting Merges $\to$ Mandatory Explicit Merge Commit
- **Mandatory Merge Point:** Whenever merge conflicts occur during reintegration, **an explicit merge commit must be created**.
- **Prohibition of Linearizing Rebases on Divergent Conflicts:** Silently rebasing away or squashing conflicting merges is forbidden. A distinct merge point ensures that conflict resolution decisions are version-controlled, auditable, and attributable.
- **Holistic Resolution:** The agent must resolve conflicts by unifying features, test coverage, and documentation from both branches. Never silently discard working features, tests, or documentation from either side.

---

## 2. Pre-Commit Verification Gate

A conflict-resolving merge commit must **never** be finalized on `master` until the merged working tree passes all repository quality gates:

1. **Code Formatting:**
   ```powershell
   cargo fmt --all -- --check
   ```
2. **Automated Architectural Rules:**
   ```powershell
   cargo test -p test_runner --test test_architecture_rules
   ```
   (Validates source file sizes $\le 800$ lines, rule file sizes $\le 23\ \text{KB}$, zero runtime panics, canonical idle micro-steps, and golden hash anti-tamper contracts).
3. **Subsystem Unit & Integration Tests:**
   Execute all test suites covering the modified or merged components (e.g. `cargo test -p debugger`, `cargo test -p gui`, `cargo test -p test_runner --test test_benchmark_smoke`).

---

## 3. Standardized Merge Commit Message Format

When completing a conflict-resolving merge, use the following structured commit message:

```text
merge(<source-branch>): integrate <source-branch> into <target-branch>

Conflict Resolution:
- <relative/path/to/file1>: <Explanation of resolution strategy>
- <relative/path/to/file2>: <Explanation of resolution strategy>

Verification:
- Passed: cargo fmt --all -- --check
- Passed: cargo test -p test_runner --test test_architecture_rules
- Passed: <subsystem-specific test suites>
```

---

## 4. Git Worktree Placement & Lifecycle Management

Whenever creating isolated worktrees for feature branches or subagent parallel tasks:

1. **Sibling Placement Invariant:**
   - Worktrees must **always** be placed as sibling directories in the parent directory (`../<repo_name>-<branch_name>`).
   - Never nest worktrees inside the repository (e.g. `.worktrees/` is prohibited).
   - Never prompt the user asking where to place the worktree; follow the sibling convention automatically.
2. **Automated Ignored Assets Linking:**
   - Large ignored test suites (`ref_src/` at 6.5 GB, `tools/AmigaTestKit/`, `Obsidian/Amiga/Reference/`, `tests/singlestep/`, `tests/benchmarks/`) and `.env` must be linked via NTFS directory junctions.
   - Always invoke the automated tool:
     ```powershell
     .\tools\git\worktree.ps1 add <branch-name>
     ```
3. **Safe Teardown:**
   - Clean up worktrees safely using `worktree.ps1` (which unlinks junctions before deletion, ensuring target files are never deleted):
     ```powershell
     .\tools\git\worktree.ps1 remove <branch-name>
     ```

---

## 5. Execution Skills

- **Worktree Management:** Follow [`git-worktree`](../skills/git-worktree/SKILL.md) for creating, linking, and managing isolated worktrees.
- **Branch Reintegration & Conflicts:** Follow [`git-resolve-merge`](../skills/git-resolve-merge/SKILL.md) for step-by-step branch reintegration, holistic conflict resolution, and verification gates.

