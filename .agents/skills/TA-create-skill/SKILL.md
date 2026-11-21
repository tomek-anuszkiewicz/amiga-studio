---
name: TA-create-skill
description: Create one atomic repository TA skill and register its executor, result, recommended transitions and reasoning effort.
---

# Create one TA skill

Apply the [invocation contract](../../references/workflow-contract.md) and
[instruction ownership](../../rules/information-hierarchy.md).
Executor: `main-session`. Result kind: `implementation`.
Before work, resolve and check both session settings under the
[runtime procedure](../../references/workflow-runtime.md#resolve-one-invocation).
Inputs are the requested operation, its intended executor, authorized paths and
the conditions under which the user would select it.

1. Check the current skills for an existing owner. Define one operation with
   required inputs, one procedure, allowed writes, evidence, output and a
   stopping point. A parameter may identify the target or scope. If it would
   select a different operation, return the decision to the caller. Preconditions
   and results never start a second skill.
2. Add `.agents/skills/TA-<action>/SKILL.md` with the exact uppercase `TA-`
   prefix, a lowercase hyphenated action, and matching directory and frontmatter
   name. Describe the activation clearly and link existing requirements and
   reusable procedures. Use the invocation contract for matching text and JSON.
3. Bind the skill to an existing executor and suitable result kind in the
   [result schema](../../../tools/workflow/result.schema.json), including its
   result and guidance skill enums. Update the
   [transition matrix](../../../tools/workflow/skill-matrix.json) with the new
   row and any justified incoming and outgoing cells. Give each recommendation
   an evidence-based condition. Leave other cells empty. Report the inherited
   settings or add a row to the [skill overrides table](../../references/workflow-runtime.md#skill-overrides)
   when the operation needs its own default or session prerequisite.
4. Update affected callers and criteria under instruction ownership. Run
   `python tools/workflow/result.py --describe <skill>`, inspect the affected
   rows with `--next-skills`, and regenerate the
   [readable matrix](../../../docs/workflow-skill-matrix.md) with `--matrix`.
   Check local links, the project naming convention and a representative result
   for the selected executor through the result validator. Report any unverified
   loading or invocation separately from static checks.

Return the definition, executor, effort, transitions and verification evidence.
If a needed executor is missing, return that prerequisite to the caller.
Stop at the skill definition and its registration.
