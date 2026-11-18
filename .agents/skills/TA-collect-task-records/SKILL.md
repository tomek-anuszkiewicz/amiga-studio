---
name: TA-collect-task-records
description: Collect a selected execution into a new private archive revision in the surviving checkout.
---

# Collect a selected execution into a new private archive revision in the surviving checkout

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Result kind: `execution_archive`. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

Use the collect command on the closed registration. Identify starting, governing
and final commits, the reviewed candidate, approved input versions, selected
timing reports and the terminal outcome. Wait for relevant producers or mark their
tail provisional. Preserve selected bytes, lineage, supplementary stage evidence
and source ranges in a new immutable revision. Read the receipt status and gaps
even on exit zero. Return its manifest locator and digest with the observed gaps.
Stop before assessment.

For a completed operation, set `manifest_ref` to the repository-relative path and SHA-256 of the verified output. When the output lives in the primary private archive, preserve a byte-identical copy of the small receipt, manifest or report alongside the result in the calling checkout. Verify it against the original.
