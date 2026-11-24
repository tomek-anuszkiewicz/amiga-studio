---
name: TA-WF-Step-register-execution
description: Register the selected thread against the primary checkout and confirm its live Collector record.
---

# Register the selected thread and confirm live telemetry

Apply the [result contract](../../references/workflow-contract.md).
The main session performs registration. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

For implementation startup or standalone registration, use the selected task
and execution IDs and observe the actual initiating thread ID. Follow
[Register and read the fresh event](../../../tools/workflow/capture/README.md#register-and-read-the-fresh-event)
against the primary checkout. Retain the receipt and fresh Collector proof
under the original deadline, including when the client sends buffered
records at turn end. Return the registration, confirmation and observed
service state to the caller. Implement Task uses confirmed proof before
checkout preparation or implementation handoff. A standalone request ends
at the registration result.

For separately selected historical or native-only evidence, give `start` the
observed time and turn ID when applicable. Record continuations explicitly. When
stopping, preserve the last included boundary and allow attributable records to
arrive late. Leave both shared Compose services running. Stop before collection.

Include the actual output location and observations in the common result.
Use existing archive files directly for the handoff.
