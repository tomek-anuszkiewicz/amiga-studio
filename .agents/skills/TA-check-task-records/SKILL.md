---
name: TA-check-task-records
description: Check archive integrity and assess the eight required evidence rows against independent observations.
---

# Check archive integrity and assess the eight required evidence rows against independent observations

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Result kind: `capture_assessment`. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

Select a manifest revision and the independent observer evidence for its trial. Use the check command. Report the eight evidence rows with their status, source locators and OTLP contributions. Explain gaps and any effect on the requested outcome. Preserve the input archive and earlier reports. Stop after the assessment.

For a completed operation, set `report_ref` to the repository-relative path and SHA-256 of the verified output. When the output lives in the primary private archive, preserve a byte-identical copy of the small receipt, manifest or report alongside the result in the calling checkout. Verify it against the original.
