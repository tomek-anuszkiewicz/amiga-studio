---
name: TA-plan-change
description: Plan one requested change with scope, acceptance criteria, risks and decisions; no implementation.
---

# Plan the selected change

Use [the invocation contract](../../references/workflow-contract.md). Inputs are
the requested outcome, relevant requirements, read scope, authorized plan paths,
candidate and round budget. This operation uses `TA-planner` and result kind `plan`.

1. Read the supplied requirements and relevant architecture. Identify the starting
   state, constraints, acceptance criteria and unresolved decisions.
2. Write the bounded implementation plan in the assigned `.agent/tasks/` path.
   Distinguish necessary changes from optional ideas and preserve unfinished work.
3. Return the identified plan, decisions, risks and acceptance criteria as text and
   JSON. Stop without test design, implementation, staging or a next-step invocation.

If a material requirement is missing, return `needs_input` with the decision; do
not invent approval. A substantive revision consumes another declared round.
