---
name: TA-collect-task-records
description: Collect a selected execution into a new private archive revision in the surviving checkout.
---

# Collect a selected execution into a new private archive revision in the surviving checkout

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Result kind: `execution_archive`. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

Use the collect command on the closed selection. Identify Git inputs, outcomes, reports and any independently produced CLI stream. Wait for relevant producers or mark their tail provisional. Preserve selected bytes, lineage, supplementary stage evidence and source ranges in a new immutable revision. Return its manifest locator and digest, with partial sources. Stop before assessment.

For a completed operation, set `manifest_ref` to the repository-relative path and SHA-256 of the verified output. When the output lives in the primary private archive, preserve a byte-identical copy of the small receipt, manifest or report alongside the result in the calling checkout. Verify it against the original.
