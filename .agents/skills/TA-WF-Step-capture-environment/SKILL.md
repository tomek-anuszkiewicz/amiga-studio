---
name: TA-WF-Step-capture-environment
description: Capture a portable environment inventory and save it with the task's archived logs.
---

# Capture a portable inventory of the selected tools, dependencies and governing workflow

Apply the [result contract](../../references/workflow-contract.md).
Executor: main session. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

Run the capture command from the selected task checkout. Save its JSON inventory
in the primary checkout under
`.agent/task-archives/<task>/<execution>/logs/environment.json`, using the existing
task and execution IDs. If that name already exists, choose a fresh name in the
same directory and preserve the earlier inventory. Create the parent directory
before capture. Without task context, use an explicitly selected archive location.

Supply the governing commit and explicitly selected client evidence. Preserve
tool probe results, installed and declared dependencies, checkout and Git object
identities and observation limits. The recorded `governing_commit` is sufficient
to recover repository-defined role settings from that revision. Separate role
tables and per-role comparisons are unnecessary. Claims about the actual session
model and effort require selected runtime evidence. Keep the inventory beside
the task's session logs, telemetry and test reports; capture needs no running
Receiver or Collector. Return the inventory location and digest. Stop after capture.

Include the actual output location and observations in the common result. Existing archive files need no duplicate handoff copy.
