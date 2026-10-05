# Merge and Worktree Rules

- Prefer fast-forward when the target is an ancestor of the source. Diverged branches require a merge even when there are no conflicts.
- Conflict resolution requires an explicit merge commit. Do not silently rebase or squash away the merge decision; reconcile features, tests, and documentation from both branches.
- Before finalizing a merge, run formatting, the [per-commit gates](git-commits.md), and tests covering the merged components. Record conflict decisions and actual verification in the commit and diary.
- Create worktrees as sibling directories `../<repo_name>-<branch_name>`, never inside the checkout. Follow this convention without requesting a placement choice.
- Worktrees use independent physical copies of ignored references, test assets, and `.env`. Do not use junctions or symbolic links across checkouts.
- Each worktree owns its Cargo cache and Graphify graph. A copied graph is only a seed and must be refreshed for the target checkout.
- Use the repository worktree script through [git-worktree](../skills/git-worktree/SKILL.md) for creation, synchronization, and removal. Use [git-resolve-merge](../skills/git-resolve-merge/SKILL.md) for reintegration, conflict resolution, and merge-message templates.
