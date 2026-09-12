# Parallel Development with Git Worktrees

This document outlines the workflow for developing features, running long test suites, or hosting concurrent AI agent sessions in parallel using **Git Worktrees**.

---

## 1. Why Git Worktrees?

- **Zero Branch Switching Latency:** Switch contexts without stashing, clean rebuilds, or interrupting running test tasks.
- **Isolated Target Builds:** Each worktree can maintain its own compilation artifacts.
- **Shared Object Database:** Zero network traffic; new branches share the local `.git` repository and instant clean checkouts.

---

## 2. Standard Worktree Workflow

### A. Creating a Worktree
```powershell
# Create branch and check it out in a sibling directory
git worktree add ..\Amiga-<branch-name> -b <branch-name>

# Copy required local configuration (.env)
$target = "..\Amiga-<branch-name>"
@('.env') | ForEach-Object { if (Test-Path $_) { Copy-Item -Force $_ "$target\$_" } }
```

### B. Developing & Testing
```powershell
cd ..\Amiga-<branch-name>

# Run architecture quality gate
cargo test -p test_runner --test test_architecture_rules

# Commit changes using Conventional Commits
git commit -am "feat(subsystem): descriptive title"
```

### C. Syncing Latest Master Changes
Because worktrees share the local repository database, `master` changes are merged immediately without fetching:
```powershell
# Option A: Merge master into feature branch
git merge master

# Option B: Rebase feature branch onto master
git rebase master
```

### D. Safe Removal & Cleanup
Once the branch is merged into `master`:
```powershell
cd ..\Amiga
git worktree remove ..\Amiga-<branch-name>
git branch -d <branch-name>
```
