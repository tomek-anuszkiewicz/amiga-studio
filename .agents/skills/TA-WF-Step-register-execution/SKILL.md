---
name: TA-WF-Step-register-execution
description: Register the selected thread and verify its saved registration in the primary checkout.
---

# Register the selected thread

Apply the [result contract](../../references/workflow-contract.md).
The main session performs registration. Use the
[capture guide](../../../tools/workflow/capture/README.md) for commands and formats.

For implementation startup, resume or standalone registration, use the selected
task and execution IDs and observe the actual initiating thread ID. Follow
[Register or resume an execution](../../../tools/workflow/capture/README.md#register-or-resume-an-execution)
against the primary checkout. Return the receipt, verified `registration.json`
path and any service state observed by the registration command. Implement Task
continues to checkout preparation or implementation as soon as the archive
registration is verified. A standalone request ends at the registration result.

For separately selected historical or native-only evidence, give `start` the
observed time and turn ID when applicable. Record continuations explicitly. When
stopping, preserve the last included boundary and allow attributable records to
arrive late. Leave both shared Compose services running. Stop before collection.

Include the actual output location and observations in the common result.
Use existing archive files directly for the handoff.
