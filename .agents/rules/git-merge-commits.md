# Merge and Worktree Rules

- Prefer fast-forward when the target is an ancestor of the source. Diverged branches require a merge even when there are no conflicts.
- Conflict resolution requires an explicit merge commit. Do not silently rebase or squash away the merge decision; reconcile features, tests, and documentation from both branches.
- For every reintegration, the agent must review every tracked Markdown file changed on either branch, including files Git merged without a conflict and files added, deleted, or moved. For a divergent merge, compare the common-base version, both branch versions, and the proposed result; manually reconcile meaning, structure, links, and claims against the merged code. For a fast-forward, review changed Markdown before advancing the target. A clean Git merge or passing automated checks does not establish that the documentation makes sense. Follow the inventory and review procedure in [git-resolve-merge](../skills/git-resolve-merge/SKILL.md).
- Before finalizing a merge, run formatting, the [per-commit gates](git-commits.md), and tests covering the merged components. Record the Markdown review scope, substantive resolutions, conflict decisions, and actual verification in the commit and diary.
- Create worktrees as sibling directories `../<repo_name>-<branch_name>`, never inside the checkout. Follow this convention without requesting a placement choice.
- Worktrees use independent physical copies of ignored references, test assets, and `.env`. Do not use junctions or symbolic links across checkouts.
- Each worktree owns its Cargo cache and Graphify graph. A copied graph is only a seed and must be refreshed for the target checkout.
- Use the repository worktree script through [git-worktree](../skills/git-worktree/SKILL.md) for creation, synchronization, and removal. Use [git-resolve-merge](../skills/git-resolve-merge/SKILL.md) for reintegration, conflict resolution, and merge-message templates.
