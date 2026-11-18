---
name: TA-compare-environments
description: Compare two saved execution inventories and identify changed, missing and unknown values.
---

# Compare two saved execution inventories and identify changed, missing and unknown values

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Result kind: `environment_comparison`. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

Identify both immutable input inventories. Use the compare command and a new report path. Summarize meaningful changes with their source and distinguish absent values from unavailable probes. Return the report and uncertainties, preserving both original inventories. Stop after comparison.

For a completed operation, set `comparison_ref` to the repository-relative path and SHA-256 of the verified output. When the output lives in the primary private archive, preserve a byte-identical copy of the small receipt, manifest or report alongside the result in the calling checkout. Verify it against the original.
