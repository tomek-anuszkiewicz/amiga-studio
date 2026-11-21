# Working with an AI agent

The engineer selects work through ordinary instructions or named repository skills.
Requests can cover one operation or a stated sequence, such as planning,
implementation, review and merge. Shared [instructions](../AGENTS.md) constrain
scope, authorization, preservation and reporting. The main session matches the
request and active task to a skill and coordinates only its authorized steps.

## Find the right guide

| Question | Start here |
| --- | --- |
| Who plans, implements, reviews or runs checks? How are models selected? | [Agent roles](../.codex/agents/README.md) |
| How does a task move from initialization to closure? Which skills does each main operation use? | [Task workflow and skills](../.agents/skills/README.md) |
| How do the three review rounds and corrective passes work? | [Review and correction flow](../.agents/skills/README.md#three-review-rounds-and-up-to-two-corrective-passes) |
| What should an operation report? | [Skill result contract](../.agents/references/workflow-contract.md) |
| How do I run tests, quality checks or coverage? | [Verification commands](../tools/verification/README.md) |
| How do I investigate CPU verification failures? | [CPU diagnostic guidance](../.agents/references/cpu-diagnostics.md) |
| How do graph navigation, RAG, hardware sources and screenshots help? | [Knowledge tools guide](../.agents/references/knowledge-tools.md) |
| Where are telemetry and execution archives documented? | [Capture guide](../tools/workflow/capture/README.md) |

Start with the workflow guide to choose the operation and follow its links to the
owning skill procedures. Consult the role guide for assignment responsibilities
and model selection. The engineer can select stages individually or authorize a
combined sequence; completing an operation does not authorize the next one.

Results distinguish actual changes, executed checks, findings and remaining work.
The [result schema](../tools/workflow/result.schema.json) describes their format;
it does not verify claims or grant permission for another action.
