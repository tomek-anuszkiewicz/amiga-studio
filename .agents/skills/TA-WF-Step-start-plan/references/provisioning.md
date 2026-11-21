# Worktree commands

## Create a task worktree

Run this command directly from the surviving checkout.

```powershell
.\tools\worktree.ps1 add <branch> -Base <base>
```

The script handles local asset copying. If the command fails, report the error
and stop.

## Run detached diagnostics

Bounded detached diagnostics use the selected tool procedure:
`tools/worktree.ps1 test <name> -Base <commit>`.
Reproduce intended dirty inputs explicitly; baseline HEAD is insufficient.
Canonical acceptance stays in the real task checkout. Remove owned disposable
checkouts from a surviving checkout through `tools/worktree.ps1 remove <name>`
after confirming active commands, ownership and pending work. Report failed cleanup.
This is a technical diagnostic procedure, not a public lifecycle dispatcher.

Before removing a real task worktree, preserve useful local outputs using
[TA-WF-Main-close-task](../../TA-WF-Main-close-task/SKILL.md). The existing worktree script
continues to own removal and its own safeguards. Never force removal, prune
unrelated registrations or recursively delete leftovers after Git failure.
Confirm the physical directory and Git registration separately.
