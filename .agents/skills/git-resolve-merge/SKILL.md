---
name: git-resolve-merge
description: Reintegrate branches, manually reconcile changed Markdown including clean Git merges, resolve conflicts, and verify the result.
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
4. **Markdown Review:** Git conflict detection is only a text merge. The agent reviews every changed tracked `.md` file before accepting the merged result, including files with no conflict markers.

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
- **Ancestor (exit 0):** Review the source's changed Markdown as described in Step 3 and verify its required results before `git merge --ff-only <source-branch>`; no new merge commit is created.
- **Diverged (exit 1):** Use `git merge <source-branch> --no-ff --no-commit`, then inspect and validate the merged tree before committing. Other exit codes indicate an error to resolve.
- **Merge conflicts:** Inspect conflicted files:
  ```powershell
  git status
  ```

### Step 3: Resolve Conflicts and Reconcile Markdown
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

Review Markdown independently of Git's conflict list. Before a fast-forward or before committing a divergent merge, identify the union of tracked `.md` paths changed from the common base to each branch. Use `--no-renames` to expose both sides of a move and `-z` to handle unusual path names:

```powershell
$base = git merge-base HEAD <source-branch>
git diff --name-status --no-renames -z $base HEAD -- '*.md'
git diff --name-status --no-renames -z $base <source-branch> -- '*.md'
```

For each path in that union, inspect the available base, target, and source versions and read the complete proposed file where it exists, not just conflict hunks. Resolve overlapping and separate edits in context; check section flow, terminology, links, referenced paths and commands, and claims against the merged implementation. Explicitly decide whether additions, deletions, and moves remain valid. Edit and stage the Markdown result when Git's clean automatic merge is incoherent. Follow [documentation maintenance](../../rules/docs-maintenance.md) for affected specifications and links.

Keep a per-path review checklist in `.agent/tmp/` during the merge. Do not finalize while any changed Markdown path is unreviewed or a substantive conflict remains unresolved. Summarize the reviewed scope and significant decisions in the merge commit and diary; do not describe automated checks as semantic approval.

### Step 4: Execute Pre-Commit Verification Gate
Never finalize a merge commit without running the full repository verification gate:
```powershell
python tools/verification/verify.py check formatting
python tools/verification/verify.py commit
python tools/verification/verify.py test rust workspace
```

Run any additional domain gates required by `AGENTS.md`. Append the merge's
own diary entry with the Markdown review scope, substantive decisions, conflict
resolutions, and observed verification before staging the completion commit.

### Step 5: Author Standardized Merge Commit
Commit using the mandatory structured merge message schema:
```powershell
git commit -m "merge(<source-branch>): integrate <source-branch> into master

Conflict Resolution:
- <file1>: <Concise description of resolution>
- <file2>: <Concise description of resolution>

Markdown Review:
- Reviewed <count> changed Markdown paths from both branches, including clean automatic merges.
- <Significant reconciliation decisions or "No corrections needed after review">.

Verification:
- Passed: python tools/verification/verify.py check formatting
- Passed: python tools/verification/verify.py check architecture
- Passed: python tools/verification/verify.py test rust workspace"
```

### Step 6: Worktree Teardown & Cleanup (If Using Worktrees)
If the branch was developed in an isolated worktree:
1. Teardown the worktree using the automated script:
   ```powershell
   .\tools\worktree.ps1 remove <branch-name>
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
- **Markdown Review:** <count> changed paths reviewed, including clean automatic merges; <significant decisions or none>.
- **Test Verification:**
  - `python tools/verification/verify.py test rust workspace`: PASS
  - `python tools/verification/verify.py check formatting`: PASS
  - `check_architecture.py`: PASS
- **Worktree Cleanup:** Worktree removed and pruned cleanly.
```
