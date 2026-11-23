---
name: TA-WF-Step-register-execution
description: Register the selected thread and confirm its live Collector record before work.
---

# Register the selected thread and confirm live telemetry

Apply the [result contract](../../references/workflow-contract.md).
The main session performs registration. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

For ordinary task startup, establish stable task and execution IDs and the
observed initiating thread ID. Follow
[Confirm telemetry before work](../../../tools/workflow/capture/README.md#confirm-telemetry-before-work).
Register the initiating thread against the primary checkout. Within the same
deadline, retain the fresh record that the thread's active client sends to
Collector. The original deadline applies even if the client sends buffered
records at the end of the turn. Once the record confirms registration, start
planning or executing the plan. Return the registration, the confirming record
and the observed state of the shared services.

For separately selected historical or native-only evidence, give `start` the
observed time and turn ID when applicable. Record continuations explicitly. When
stopping, preserve the last included boundary and allow attributable records to
arrive late. Leave both shared Compose services running. Stop before collection.

Include the actual output location and observations in the common result.
Use existing archive files directly for the handoff.
