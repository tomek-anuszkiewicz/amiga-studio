---
name: TA-WF-Step-collect-task-records
description: Copy finished task outputs and logs to the selected task archive.
---

# Copy task outputs and logs

Apply the [result contract](../../references/workflow-contract.md). After the
tests and other producers finish, use the
[artifact tool](../../../tools/artifacts/README.md#preserve-task-outputs) to
copy their results and logs. For a dedicated checkout, use the known output
locations and include any custom paths. For the shared primary checkout, supply
the paths produced by the task. Use task and execution names to locate the
archive. Copying works independently of telemetry registration and time windows.

For selected whole files outside the checkout, such as completed session logs,
use the [capture guide](../../../tools/workflow/capture/README.md#preserve-selected-files-at-task-closure).
Keep source files and earlier copies if a copy fails. On retry, use any archive
paths already reported. Return the actual destinations and any blocked paths.
Stop after copying.
