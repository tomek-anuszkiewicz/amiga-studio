---
name: TA-capture-environment
description: Capture a portable inventory of the selected tools, dependencies and governing workflow.
---

# Capture a portable inventory of the selected tools, dependencies and governing workflow

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Result kind: `environment_inventory`. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

Use the capture command with the selected checkout and a fresh output path. Supply the governing commit and explicitly selected client evidence. Preserve tool probe results, installed and declared dependencies, checkout and Git object identities, configured roles and observation limits. Return the inventory location and digest. Stop after capture.

For a completed operation, set `inventory_ref` to the repository-relative path and SHA-256 of the verified output. When the output lives in the primary private archive, preserve a byte-identical copy of the small receipt, manifest or report alongside the result in the calling checkout. Verify it against the original.
