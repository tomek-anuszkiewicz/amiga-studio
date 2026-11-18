---
name: TA-register-execution
description: Register the selected thread and capture boundary, and start or reuse the optional local receiver.
---

# Register the selected thread and capture boundary, and start or reuse the optional local receiver

Apply the [invocation contract](../../references/workflow-contract.md).
Executor: main session. Result kind: `execution_registration`. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

Run the register command from the selected thread with its execution ID, primary checkout and chosen boundary. For OTLP, confirm the project service identity, loopback intake and effective admission before covered work starts. For a next-turn start, register the control turn and arm the saved selection after its observed completion. Record continuations explicitly. On stop, preserve the last included boundary and allow attributable delayed records. Shut down only when selected and idle. Return the registration and actual service state. Stop before collection.

For a completed operation, set `receipt_ref` to the repository-relative path and SHA-256 of the verified output. When the output lives in the primary private archive, preserve a byte-identical copy of the small receipt, manifest or report alongside the result in the calling checkout. Verify it against the original.
