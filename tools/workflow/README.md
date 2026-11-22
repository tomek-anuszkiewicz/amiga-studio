# Skill results

Repository skills return a written result and a JSON object. Read the shared
[result contract](../../.agents/references/workflow-contract.md) for the contents
of the result and [result.schema.json](result.schema.json) for the JSON format.
The repository provides no result CLI, automatic renderer, skill transition
matrix, review router or delivery coordinator.

The user can select one operation or a sequence. Use ordinary Git commands for
selected commits and merges. Run the selected checks with the existing
[verification commands](../verification/README.md). The
[capture tools](capture/README.md) support telemetry and, in a separate operation,
preserve selected files in task archives.
