---
name: TA-register-execution
description: Register the selected thread and confirm its live Collector record before work.
---

# Register the selected thread and confirm live telemetry

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Result kind: `execution_registration`. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

For ordinary task startup, establish stable task and execution IDs and the
observed initiating thread ID. Follow
[Confirm telemetry before work](../../../tools/workflow/capture/README.md#confirm-telemetry-before-work).
Register the initiating thread against the primary checkout, then retain the
fresh Collector record from its active client in the same deadline. A pending
turn-end flush stays within the original deadline. Start planning or plan
execution only after confirmation. Return both the registration and proof
with the observed shared service state.

For separately selected historical or native-only evidence, record the
requested boundary. For a historical next-turn start, register the control turn
and arm the saved selection after its observed completion. Record continuations
explicitly. On stop, preserve the last included boundary and allow attributable
delayed records. Leave both shared Compose services running. Stop before
collection.

For a completed operation, set `receipt_ref` to the repository-relative path and SHA-256 of the verified output. When the output lives in the primary private archive, preserve a byte-identical copy of the small receipt, manifest or report alongside the result in the calling checkout. Verify it against the original.
