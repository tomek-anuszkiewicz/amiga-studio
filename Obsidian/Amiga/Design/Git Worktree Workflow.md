# Git Worktree Workflow & Subsystem Integration

This document outlines the standard operational procedure for working with **Git Worktrees** in this repository. Git worktrees enable simultaneous work on multiple branches in isolated working directories without switching branches or interrupting long-running tests in the primary workspace.

---

## 1. Overview & Mechanical Advantages

Working with git worktrees in this project provides several specific benefits:
1. **Isolated Cargo Build Cache (`target/`)**: Each worktree maintains its own `target/` directory. You can compile, run benchmarks, or execute `SINGLESTEP_FULL=1` in one tree without locking `target/` in another.
2. **Concurrent AI Agent Sessions**: Run separate agent tasks on dedicated branches without branch switching conflicts (`git checkout` / `git stash`).
3. **Instant Zero-Cost Clean Checkouts**: SingleStepTests JSON files (`ref_src/SingleStepTests-m68000/v1/*.json` and `ref_src/SingleStepTests-680x0/68000/v1/*.json`) are tracked in Git and checked out automatically.

---

## 2. Quick Command Reference

### Step 1: Create Worktree & Branch
Run from the root of the primary repository:

```powershell
# Create a new branch and checkout into a sibling directory
git worktree add ..\Amiga-<branch-name> -b <branch-name>
```

### Step 2: Copy Ignored Configuration Files
Copy necessary untracked configuration and baseline data from the primary repository to the worktree using this PowerShell one-liner:

```powershell
$target = "..\Amiga-<branch-name>"
@('.env', '.test_results') | ForEach-Object { if (Test-Path $_) { Copy-Item -Recurse -Force $_ "$target\$_" } }
```

### Step 3: Work & Test in Worktree
```powershell
cd ..\Amiga-<branch-name>

# Verify architecture rules and subsystem tests
cargo test -p test_runner --test test_architecture_rules

# Stage and commit your work
git add -A
git commit -m "feat(subsystem): your descriptive commit message"
```

### Step 4: Mid-Development Synchronization (Pulling `master` into Worktree)
Because the worktree shares the same `.git` object store with the primary repository, any commit added to `master` in the main directory is immediately accessible inside your worktree without needing network pushes or pulls.

If you are in the middle of development in your worktree and want to incorporate the latest changes from `master`:

#### Option A: Merge `master` into your Feature Branch (Standard Merge)
Preserves branch history and creates an explicit merge commit:
```powershell
cd ..\Amiga-<branch-name>

# 1. Ensure your local work is cleanly committed or stashed
git status
# If you have uncommitted changes: git stash

# 2. Merge local master into your current branch
git merge master

# 3. Restore stashed changes (if stashed above)
# git stash pop
```

#### Option B: Rebase on `master` (Clean Linear History)
Replays your worktree commits directly on top of `master`:
```powershell
cd ..\Amiga-<branch-name>
git rebase master
```

#### Option C: Pull Remote Changes (if `master` was pushed to remote repository)
```powershell
cd ..\Amiga-<branch-name>
git fetch origin
git merge origin/master
# or: git rebase origin/master
```

> [!NOTE]
> If a merge conflict occurs during synchronization:
> 1. Run `git status` to see conflicting files.
> 2. Resolve the conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`).
> 3. Stage resolved files: `git add <file>`.
> 4. Conclude sync: `git merge --continue` (or `git rebase --continue`).
> 5. To cancel and restore previous state: `git merge --abort` (or `git rebase --abort`).

### Step 5: Final Reintegration (Merge Back to Primary Repository)
When feature work is finished and all tests pass, reintegrate changes into `master`:

#### Scenario A: Clean Reintegration (Zero Conflicts)
If `master` has not diverged or changes merge cleanly without conflicts, a fast-forward merge is permitted:
```powershell
cd ..\Amiga
git checkout master
git merge --ff-only <branch-name>
```

#### Scenario B: Divergent Work or Merge Conflicts (Mandatory Merge Commit)
Per the repository rule ([`git-merge-commits.md`](../../.agents/rules/git-merge-commits.md)), if `master` has diverged or merge conflicts occur:
1. **Never squash or rebase away the merge point:** An explicit merge commit must be created to preserve the branch history and document the conflict resolution audit trail.
2. **Perform standard merge:**
   ```powershell
   cd ..\Amiga
   git checkout master
   git merge <branch-name>
   ```
3. **Resolve all conflicts holistically:** Ensure no working features, tests, or docs from either branch are dropped.
4. **Pass verification gate:**
   ```powershell
   cargo fmt --all -- --check
   cargo test -p test_runner --test test_architecture_rules
   ```
5. **Commit the formal merge commit:**
   ```powershell
   git commit -m "merge(<branch-name>): integrate <feature> into master"
   ```

### Step 6: Teardown & Clean Up
Once merged, remove the worktree and clean up references:

```powershell
# Remove worktree directory and Git link
git worktree remove ..\Amiga-<branch-name>

# Delete feature branch
git branch -d <branch-name>

# Prune any stale metadata
git worktree prune
```

To view all active worktrees at any time:
```powershell
git worktree list
```

---

## 3. Analysis of `.gitignore` & Untracked Files

Review of repository ignored paths and whether to replicate them in worktrees:

| Path | Category | Action for Worktree | Rationale |
| :--- | :--- | :--- | :--- |
| `.env` | Environment Config | **Copy Mandatory** | Contains `RAG_CACHE_FILE` and optional API keys. Required for Python tools and RAG integration. |
| `.test_results/` | Regression Baselines | **Copy Recommended** | Stores baseline test metrics (`test_results.json`). Copying allows `cargo run -p test_runner -- --diff` to compare against master immediately. |
| `target/` | Cargo Build Artifacts | **Do NOT Copy** | Keep separate. Isolated `target/` avoids compiler file locks between parallel workspaces. |
| `graphify-out/cache/` | AST Parser Cache | **Do NOT Copy** | Re-generated on demand by `graphify`. Code graph metadata (`graph.json`, `wiki/`) is tracked in Git. |
| `Obsidian/Amiga/.obsidian/` | Editor State | **Do NOT Copy** | Local workspace state and cursor positions. |
| `.tmp.drive*` | Temporary Sync | **Do NOT Copy** | Ephemeral drive upload/download buffers. |
| `__pycache__/` | Python Bytecode | **Do NOT Copy** | Generated automatically when Python scripts run. |

> [!NOTE]
> All SingleStepTests JSON files (`ref_src/SingleStepTests-m68000/v1/*.json` and `ref_src/SingleStepTests-680x0/68000/v1/*.json`) are tracked in Git. They are automatically checked out in newly created worktrees without needing `python decode.py`.

---

## 4. Subsystem & Tooling Caveats in Worktrees

### 4.1 RAG Tooling (`tools/rag`, FastMCP & Qdrant)
- **Centralized Vector Store:** The local Qdrant vector database (`http://localhost:6333`, collection `amiga`) stores document chunks and embeddings referencing file paths from the primary repository.
- **MCP Server Registration:** Agent IDE configurations register the FastMCP server (`rag_mcp_server.py`) against the primary repository.
- **Recommendation:** Keep RAG ingestion and indexing (`amiga_rag.ps1 . --source amiga`) centralized in the primary repository. Semantic search (`rag_search`) queries Qdrant directly and works seamlessly from any worktree as long as `.env` is present.

### 4.2 Obsidian Vault (`Obsidian/Amiga/`)
- **Vault Folder Anchor:** The Obsidian desktop application opens a specific filesystem folder as a vault (typically `<repo_path>/Obsidian/Amiga`).
- **Isolation Effect:** Markdown design notes or reference files edited in `..\Amiga-<branch-name>\Obsidian\Amiga\` will **not** appear in your active Obsidian vault window until the branch is merged back into the primary repository.
- **Workaround:** If extensive note editing or review is needed inside a worktree before merging, open `..\Amiga-<branch-name>\Obsidian\Amiga\` as a separate vault in Obsidian (`Open folder as vault`).

### 4.3 AST & Knowledge Graph (`graphify`)
- The knowledge graph file (`graphify-out/graph.json`) and wiki (`graphify-out/wiki/`) are tracked in Git.
- If you refactor or add new modules in a worktree, you can run `graphify update .` directly within the worktree to update the local AST and relationships before committing.

### 4.4 IDE & VS Code / Antigravity GUI Worktree Integration
- **Automatic Discovery:** VS Code and Antigravity IDE natively detect all linked worktrees by inspecting the central `.git/worktrees/` directory.
- **Source Control View (SCM):** In the **Source Control** sidebar (`Ctrl+Shift+G`), the **Source Control Repositories** sub-view automatically displays each worktree, its checked-out branch, and live pending change counts.
- **Quick Switching & New Windows:**
  - In the SCM Repositories list, right-click any worktree to select **"Open in New Window"** to operate multiple development branches simultaneously in parallel IDE instances.
  - You can also add worktrees to your current workspace via **File $\to$ Add Folder to Workspace...** for multi-root editing.
- **Command Palette:** Run `Git: Open Repository...` or `Git: Check out to...` (`Ctrl+Shift+P`) to quickly focus or switch between worktree branches without manual terminal navigation.

