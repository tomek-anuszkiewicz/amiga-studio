# Worktree commands

## Create a task worktree

Run this command directly from the surviving checkout.

```powershell
.\tools\worktree.ps1 add <branch> -Base <base>
```

The script handles local asset copying. If the command fails, report the error
and stop.

## Run detached diagnostics

Use `tools/worktree.ps1 test <name> -Base <commit>` for bounded diagnostics in a
detached checkout. Reproduce any intended uncommitted inputs explicitly in
addition to the baseline HEAD. Run acceptance checks in the real task checkout.
Before removing the disposable checkout, confirm ownership and check for active
commands and pending work. Run `tools/worktree.ps1 remove <name>` from a
surviving checkout and report any cleanup failure. This procedure serves
technical diagnostics only. It does not dispatch public lifecycle operations.

Before removing a real task worktree, preserve useful local outputs using
[TA-WF-Main-close-task](../../TA-WF-Main-close-task/SKILL.md). The existing worktree script
continues to own removal and its own safeguards. Never force removal, prune
unrelated registrations or recursively delete leftovers after Git failure.
Confirm the physical directory and Git registration separately.
