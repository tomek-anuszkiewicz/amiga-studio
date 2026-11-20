---
name: TA-plan-change
description: Plan one requested change with scope, acceptance criteria, risks and decisions; no implementation.
---

# Plan the selected change

Use [the invocation contract](../../references/workflow-contract.md). Inputs are
the requested outcome, relevant requirements, read scope, authorized plan paths,
candidate and round budget. This operation uses `TA-planner` and result kind `plan`.

1. For a new task, check the caller's [telemetry startup confirmation](../../references/workflow-contract.md#telemetry-startup-prerequisite).
   Read the supplied requirements and relevant architecture. Identify the starting
   state, constraints, acceptance criteria and unresolved decisions.
2. Write the bounded implementation plan in English in the assigned
   `.agent/tasks/` path, regardless of the conversation language, including Polish.
   Follow the shared [language policy](../../rules/language-policy.md).
   Distinguish necessary changes from optional ideas and preserve unfinished work.
   Record agreed observable outcomes, the applicable design destination and
   [concrete approval decisions](../../references/workflow-guidance.md#record-individual-approvals).
   For an amendment retain unaffected requirements and identify what changed.
3. Return the identified plan, decisions, risks and acceptance criteria as text and
   JSON. Report agreed testable behavior or other acceptance evidence for the
   [main session's planning handoff](../../references/workflow-guidance.md#save-a-formal-plan).

If a material requirement is missing, return `needs_input` with the decision; do
not invent approval. A substantive revision consumes another declared round.
This operation ends with the plan and its result.
