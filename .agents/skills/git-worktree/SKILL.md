---
name: git-worktree
description: Create, synchronize, and tear down isolated Git worktrees with automatic physical copying of ignored test assets and .env (Zero NTFS Junctions).
---

# Recipe: Isolated Git Worktree Lifecycle & Physical Asset Isolation

This skill defines the operational procedure for managing Git worktrees, ensuring that large ignored test suites, reference documentation, and configuration files are physically copied to maintain complete repository isolation (zero NTFS junctions).

---

## 1. When to Use This Skill

Activate this skill whenever:
- The user requests a new worktree (e.g., *"create a worktree for branch X"*, *"set up an isolated worktree for Y"*).
- Running parallel subagents or background tasks that require an isolated checkout without disrupting the active working tree.
- Repairing or synchronizing an existing worktree that is missing ignored test vectors, reference documentation, or `.env`.

---

## 2. Constitutional Invariants

1. **Sibling Directory Placement Invariant:**
   - Worktrees must **always** be placed as sibling directories in the repository's parent folder (`../<repo_name>-<branch_name>`).
   - **Zero Prompts to User:** Never ask the user where to place the worktree. Sibling placement in the parent folder is the standard invariant.
   - **No Internal Worktree Nesting:** Never create worktrees inside the repository (e.g., `.worktrees/` is deprecated).

2. **Strict Worktree Isolation & Zero-Junction Invariant:**
   - **Strict Prohibition of NTFS Junctions and Directory Links:** Never use NTFS directory junctions (`New-Item -ItemType Junction`, `mklink /J`) or symbolic links across worktrees or repositories. Every worktree must remain 100% self-contained with independent physical storage.
   - **Independent Physical Copies:** The test suite and documentation rely on files excluded by `.gitignore`:
     - `ref_src/` (CPU test vectors and reference emulators)
     - `Obsidian/Amiga/Reference/` (Commodore hardware manuals)
     - `tools/AmigaTestKit/` (test floppy ADFs)
     - `tests/singlestep/` & `tests/benchmarks/` (test baseline outputs)
     - `.env` (environment configuration, `RAG_CACHE_FILE`)
   - **Local Knowledge Graphs:** `graphify-out` is strictly local to each repository and must never be linked or copied across worktrees.
   - **Mandatory Script Use:** Always use `.\tools\git\worktree.ps1` to create and tear down worktrees. The script automatically executes multi-threaded physical copying of necessary test assets into standalone directories.

3. **Isolated Cargo Build Cache:**
   - `target/` is deliberately **not** linked or copied. Each worktree maintains an independent Cargo build cache to prevent concurrent compiler database lock contention.

---

## 3. Operational Workflow

### Step 1: Create an Isolated Worktree
When the user asks to create a worktree for a branch or feature:
```powershell
.\tools\git\worktree.ps1 add <branch_name>
```
To branch from a specific base branch:
```powershell
.\tools\git\worktree.ps1 add <branch_name> -Base master
```

The script will:
1. Determine the main repository root.
2. Create the worktree at the sibling path `../<repo_name>-<branch_name>`.
3. Copy test fixtures into independent physical directories.
4. Copy `.env`.

### Step 2: Working Within the Worktree
Inform the user of the created path. When executing commands in the worktree, pass the worktree path as the working directory (`Cwd`):
```powershell
cargo test -p <crate>
```

### Step 3: Repairing / Synchronizing an Active Worktree
If an existing worktree was created without the script and is missing `.env` or `ref_src/`:
```powershell
.\tools\git\worktree.ps1 sync
```

### Step 4: Teardown & Cleanup
Once the worktree branch has been merged into `master` or is no longer needed:
```powershell
.\tools\git\worktree.ps1 remove <branch_name>
```
The script removes the worktree via `git worktree remove --force` and prunes stale worktree registrations.

---

## 4. Subagent Delegation Contract

When delegating an isolated task to a subagent:
1. The parent agent creates the worktree via `.\tools\git\worktree.ps1 add <branch>`.
2. The subagent executes all commands strictly within the worktree directory.
3. Upon completion and verification, the subagent returns its report to the parent.
4. The parent merges the branch (using `git-resolve-merge` if conflicts arise) and runs `.\tools\git\worktree.ps1 remove <branch>`.
