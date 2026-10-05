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

1. **Ancestry Permits Fast-Forward:** If the target is an ancestor of the source, prefer `git merge --ff-only`. A conflict-free merge of diverged branches still needs a merge commit.
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
Check whether the target is an ancestor of the source:
```powershell
git merge-base --is-ancestor HEAD <source-branch>
```
- **Ancestor (exit 0):** Verify the source's required results before `git merge --ff-only <source-branch>`; no new merge commit is created.
- **Diverged (exit 1):** Use `git merge <source-branch> --no-ff --no-commit`, then inspect and validate the merged tree before committing. Other exit codes indicate an error to resolve.
- **Merge conflicts:** Inspect conflicted files:
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
python tools/harness/pre_flight.py --quick
cargo test -p test_runner --test test_architecture_rules
cargo test --workspace --exclude test_runner
```

Run any additional domain gates required by `AGENTS.md`. Append the merge's
own diary entry with conflict decisions and observed verification before staging
the completion commit.

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
- Passed: cargo test --workspace --exclude test_runner"
```

### Step 6: Worktree Teardown & Cleanup (If Using Worktrees)
If the branch was developed in an isolated worktree:
1. Teardown the worktree using the automated script:
   ```powershell
   .\tools\git\worktree.ps1 remove <branch-name>
   ```
2. Delete the merged feature branch (if not deleted by worktree remove):
   ```powershell
   git branch -d <source-branch>
   ```

---

## 5. Standard Merge Report Format

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
