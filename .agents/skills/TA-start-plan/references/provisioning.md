# Physical worktree provisioning

The caller has already selected creation or repair. Use `tools/worktree.ps1`; do
not create junctions, directory links, shared build targets or substitute a custom
worktree wrapper. New task worktrees are siblings `../<repo_name>-<branch_name>`.
Run commands with explicit working directories; changing a tool's cwd does not
prove that an agent has loaded a different instruction context.

Before creation or full asset repair, validate the source checkout's pinned FPGA
sources and then verify the target after copying:

```powershell
.\tools\bootstrap\bootstrap_sources.ps1 -MiniMig -AmigaReplacementProject
```

Creation uses `tools/worktree.ps1 add <branch> -Base <base>`. Full explicitly
selected asset repair uses `tools/worktree.ps1 sync` from the target. Both use
independent physical copies of `ref_src/`, `Obsidian/Amiga/Reference/`,
`tools/AmigaTestKit/`, `.agent/tmp/singlestep/`, `.agent/tmp/benchmarks/`,
`.env` and graph seeds. The reference graph remains inside `ref_src/`.
Preserve the target's tracked `ref_src/.graphifyignore`. Project graph refresh uses
the invoking checkout's `tools/graphify/refresh.py` with the target root.
`target/` is neither shared nor copied.

For an explicitly scoped missing-FPGA repair, physically copy only the identified
missing source directories and their `_bootstrap_source.json` manifests from a
validated checkout, then verify target hashes with the command above. Do not turn
that selected repair into a full synchronization or master merge.

Bounded detached diagnostics retain their rule/tool exception:
`tools/worktree.ps1 test <name> -Base <commit>`. They create no task branch.
Reproduce intended dirty inputs explicitly; baseline HEAD is insufficient.
Canonical acceptance stays in the real task checkout. Remove owned disposable
checkouts from a surviving checkout through `tools/worktree.ps1 remove <name>`
after confirming active commands, ownership and pending work. Report failed cleanup.
This is a technical diagnostic procedure, not a public lifecycle dispatcher.

Removal discards ignored configuration, references, caches and diagnostics. Do not
inventory, hash, compare, copy or archive ignored contents during teardown. Never
force removal, prune unrelated registrations or recursively delete leftovers after
Git failure. Confirm physical directory and Git registration separately. Closed-task
branch removal and publication checks belong to the existing integration tool.

