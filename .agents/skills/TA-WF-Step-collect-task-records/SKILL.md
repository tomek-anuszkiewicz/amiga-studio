---
name: TA-WF-Step-collect-task-records
description: Preserve selected whole files in an existing execution archive.
---

# Preserve selected task records

Apply the [result contract](../../references/workflow-contract.md). Use the
[capture guide](../../../tools/workflow/capture/README.md#preserve-selected-files-at-task-closure)
for the existing commands and formats.

Identify the files selected for copying and the registered execution directory
where the copies belong. Run `records.py preserve` with the explicit input list.
Keep the source files and any processes still producing them available. On a
retry, use the archived paths the command printed. Report where the copies were
saved and which paths were blocked. Copying requires neither a publication record nor
a graph refresh. Stop after copying.

For ignored test outputs before worktree removal, use the independent
[artifact tool](../../../tools/artifacts/README.md) through the cleanup skill.
