# Worktree commands

## Create a task worktree

Run this command directly from the surviving checkout.

```powershell
.\tools\worktree.ps1 add <branch> -Base <base>
```

The script handles local asset copying. If the command fails, report the error
and stop.

## Run detached diagnostics

Bounded detached diagnostics retain their rule/tool exception:
`tools/worktree.ps1 test <name> -Base <commit>`.
Reproduce intended dirty inputs explicitly; baseline HEAD is insufficient.
Canonical acceptance stays in the real task checkout. Remove owned disposable
checkouts from a surviving checkout through `tools/worktree.ps1 remove <name>`
after confirming active commands, ownership and pending work. Report failed cleanup.
This is a technical diagnostic procedure, not a public lifecycle dispatcher.

Do not inventory, hash, compare, copy or archive ignored contents during teardown.
Never force removal, prune unrelated registrations or recursively delete leftovers
after Git failure. Confirm physical directory and Git registration separately.
Closed-task branch removal and publication checks belong to the existing integration tool.
