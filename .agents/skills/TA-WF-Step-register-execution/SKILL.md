---
name: TA-WF-Step-register-execution
description: Register the selected thread and confirm its live Collector record before work.
---

# Register the selected thread and confirm live telemetry

Apply the [result contract](../../references/workflow-contract.md).
Executor: main session. Use the
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

Include the actual output location and observations in the common result. Existing archive files need no duplicate handoff copy.
