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

Supply the governing commit and any client evidence explicitly selected for
capture. Preserve tool probe results, installed and declared dependencies,
checkout and Git object identities, and the limits of those observations.
Readers can use the recorded `governing_commit` to look up the repository's role
settings for that revision. This requires no separate role tables or per-role
comparisons. To report which model and reasoning effort a session actually used,
supply the selected runtime evidence. Keep the inventory beside the
task's session logs, telemetry and test reports. The capture command can run
without the Receiver or Collector. Return the inventory location and digest.
Stop after capture.

Include the actual output location and observations in the common result.
Refer to existing archive files directly without making a duplicate handoff
copy.
