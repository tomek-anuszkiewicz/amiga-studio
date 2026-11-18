---
name: TA-verify-focused
description: Execute only an assigned focused verification command and return its actual evidence.
---

# Run the selected focused check

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist; result kind: `verification`;
profile: `focused`.
Inputs: exact focused command/scope, candidate, relevant environment/fixtures and
output location. Use the [verifier command reference](../../../tools/verification/README.md).

1. Confirm command selection and resources are available and no conflicting shared
   target writer is active. Use `--plan` when command expansion needs inspection.
2. Run the assigned focused command through the existing verifier or its documented
   skill-owned checker. Await exit status and inspect actual output/report identity.
3. Return selected scope, command, outcome, artifacts, candidate and limits.
   Missing/failed/interrupted execution is incomplete or failed, never a pass.

Stop after the check. Do not repair source, change acceptance, widen selection,
measure unrequested coverage or escalate to commit/milestone verification.

