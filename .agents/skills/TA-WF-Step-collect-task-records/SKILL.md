---
name: TA-WF-Step-collect-task-records
description: Preserve selected whole files in an existing execution archive.
---

# Preserve selected task records

Apply the [result contract](../../references/workflow-contract.md). Use the
[capture guide](../../../tools/workflow/capture/README.md#preserve-selected-files-at-task-closure)
for the existing commands and formats.

For a selected copy, identify the known files and registered execution directory.
Run `records.py preserve` with its explicit input list. Keep unfinished producers
and source files available; use archived paths printed by the command on a retry.
Return actual copied locations and blocked paths. There is no publication record
or graph-refresh prerequisite. Stop after copying.

For ignored test outputs before worktree removal, use the independent
[artifact tool](../../../tools/artifacts/README.md) through the cleanup skill.
