---
name: git-resolve-merge
description: Resolve 3-way Git merge conflicts holistically in isolated worktrees with regression verification.
---

# Recipe: Git Branch Reintegration & Conflict Resolution

This skill provides the operational procedure for managing Git worktrees, merging divergent branches, resolving merge conflicts, and formatting standardized merge commits per [`.agents/rules/git-merge-commits.md`](../../rules/git-merge-commits.md).

---

## 1. When to Use This Skill

Activate this skill whenever:
- Reintegrating a feature branch or isolated Git worktree back into `master`.
- Resolving Git merge conflicts between divergent branches.
- Cleaning up and pruning Git worktrees after successful branch reintegration.

---

## 2. Core Reintegration Principles

1. **Clean Merges $\to$ Fast-Forward:** If branches have not diverged, fast-forward merges (`git merge --ff-only`) are preferred to maintain a clean linear history.
2. **Conflicting Merges $\to$ Explicit Merge Commit:** Never silently rebase away conflicts. Create an explicit merge commit documenting the conflict resolution strategy.
3. **Holistic Resolution:** Never blindly accept `--ours` or `--theirs`. Merge conflicting code, unit tests, and documentation from both branches.

---

## 3. Step-by-Step Execution Workflow

### Step 1: Prepare Target Branch
Ensure your target branch (`master`) is clean and up-to-date:
```powershell
git checkout master
git status
```

### Step 2: Attempt Merge
Attempt the merge from the source branch:
```powershell
git merge <source-branch> --no-commit
```
- **Scenario A (Clean Merge):** If there are zero conflicts, finalize the merge or use fast-forward.
- **Scenario B (Merge Conflicts):** Inspect conflicted files:
  ```powershell
  git status
  ```

### Step 3: Resolve Conflicts Holistically
For each conflicted file:
1. Open the file and inspect the conflict markers (`<<<<<<< HEAD`, `=======`, `>>>>>>> <source-branch>`).
2. Unify architectural changes:
   - **Rust Code:** Retain new structs/methods from both branches. Verify disjoint borrowing and zero allocation invariants.
   - **Unit Tests:** Retain and merge test cases from both branches.
   - **Design Docs & DIARY.md:** Combine chronological entries and design notes without losing historical entries.
3. Remove all conflict markers and stage resolved files:
   ```powershell
   git add <resolved-file>
   ```

### Step 4: Execute Pre-Commit Verification Gate
Never finalize a merge commit without running the full repository verification gate:
```powershell
cargo fmt --all -- --check
cargo test -p test_runner --test test_architecture_rules
cargo test --workspace --exclude test_runner
```

### Step 5: Author Standardized Merge Commit
Commit using the mandatory structured merge message schema:
```powershell
git commit -m "merge(<source-branch>): integrate <source-branch> into master

Conflict Resolution:
- <file1>: <Concise description of resolution>
- <file2>: <Concise description of resolution>

Verification:
- Passed: cargo fmt --all -- --check
- Passed: cargo test -p test_runner --test test_architecture_rules
- Passed: cargo test --workspace"
```

### Step 6: Worktree Teardown & Cleanup (If Using Worktrees)
If the branch was developed in an external worktree:
1. Remove the secondary worktree:
   ```powershell
   git worktree remove <worktree-path> --force
   ```
2. Prune stale worktree metadata:
   ```powershell
   git worktree prune
   ```
3. Delete the merged feature branch:
   ```powershell
   git branch -d <source-branch>
   ```

---

## 5. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Medium`
- **Context Savings:** Isolates raw `diff3` conflict markers, multi-file conflict churn, and worktree git status commands from the main pair-programming context.
- **Subagent Task Template:**
  - `TaskName`: "Git Merge & Conflict Resolution: <branch_name>"
  - `TaskSummary`: "Creates isolated worktree, resolves 3-way conflicts holistically, verifies tests, and creates structured merge commit."
  - `Prompt`:
    ```markdown
    Merge branch `<SOURCE_BRANCH>` into `<TARGET_BRANCH>` using isolated worktree.
    Follow .agents/skills/git-resolve-merge/SKILL.md:
    1. Create isolated worktree at `.worktrees/merge-<branch>`.
    2. Attempt merge: `git merge --no-ff <source_branch>`.
    3. If conflicts occur, analyze intent of both sides and preserve all non-conflicting features.
    4. Run `cargo test` and `test_architecture_rules`.
    5. Commit with standardized merge commit message.
    6. Clean up worktree.
    7. Return strictly the Conflict Resolution Report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### 🔀 Git Merge & Conflict Resolution Report
  - **Source Branch:** `<source_branch>` $\to$ **Target Branch:** `<target_branch>`
  - **Merge Status:** [MERGED CLEANLY | CONFLICTS RESOLVED | ABORTED]
  - **Merge Commit SHA:** `<commit_sha>`
  - **Conflict Decision Log:**
    | Conflicted File | Conflicting Aspects | Resolution Rationale |
    | :--- | :--- | :--- |
    | `crates/.../<crate>.rs` | Both branches added imports | Merged both import blocks, eliminated duplicate entries |
  - **Test Verification:**
    - `cargo test --all`: PASS
    - `cargo fmt --all -- --check`: PASS
    - `test_architecture_rules`: PASS
  - **Worktree Cleanup:** Worktree removed and pruned cleanly.
  ```
