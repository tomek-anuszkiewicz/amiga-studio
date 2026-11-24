# Skill results

Agents send a written account and JSON to the main session. The main session
reports the outcome, checks and outstanding work to the engineer in plain
language. The reply can link an existing saved result when useful.
[Result contract](../../.agents/references/workflow-contract.md#inputs-and-outputs) ·
[Handoff schema](result.schema.json)

The user can select one operation or a sequence. Use ordinary Git commands for
selected commits and merges. Run the selected checks with the existing
[verification commands](../verification/README.md). The
[capture tools](capture/README.md) support telemetry and, in a separate operation,
preserve selected files in task archives.
