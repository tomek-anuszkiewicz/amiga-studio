# Working with an AI agent

Ask the agent in plain language or name a repository skill. You can request a
single operation or specify a sequence, such as planning, implementation, review
and merge. The main session selects a skill based on your request and the active
task. It coordinates the steps you authorized and follows the shared rules on
scope, authorization, preservation and reporting.
[Shared instructions](../AGENTS.md)

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

Use the workflow guide to choose an operation and find its skill procedures.
The role guide explains who does each assignment and how the main session
selects models. Authorize stages one at a time or as a sequence. If your
authorization covers the next stage, the agent can proceed when the current
stage ends.

The agent's result describes what changed, which checks ran, what the agent found
and what remains. The result schema defines the format. The agent needs evidence
for its claims and your permission for further work.
[Result schema](../tools/workflow/result.schema.json)
