---
name: TA-create-agent
description: Create one repository TA agent with bounded responsibility, assigned skills and default reasoning settings.
---

# Create one TA agent

Apply the [invocation contract](../../references/workflow-contract.md) and
[instruction ownership](../../rules/information-hierarchy.md).
Executor: `main-session`. Result kind: `implementation`.
Before work, resolve and check both session settings under the
[runtime procedure](../../references/workflow-runtime.md#resolve-one-invocation).
Inputs are the requested responsibility, authorized paths, relevant existing
skills and the boundaries the role must follow.

1. Check existing role and skill ownership. Identify the responsibility, context
   or write boundary that requires a new role. Record read scope, allowed writes,
   tools or evidence specific to the role, and stopping rules. Use the shared
   invocation contract for rounds and matching text and JSON.
2. Add `.codex/agents/TA-<role>.toml` with the exact uppercase `TA-` prefix, a
   lowercase hyphenated role name, and matching filename and `name`. Keep
   instructions specific to the role. Describe inherited permissions and any
   behavioral restrictions accurately. Keep review independence where required.
3. Add the role's default model and effort to the
   [role defaults table](../../references/workflow-runtime.md#role-defaults).
   A TOML pin belongs only on a setting the user explicitly fixes for every
   invocation of the role.
   Bind the role and any authorized existing skill assignments in the
   [result schema](../../../tools/workflow/result.schema.json), including its
   result and guidance agent enums. Reconcile affected skill instructions and
   callers. After any reassignment, regenerate
   the [readable matrix](../../../docs/workflow-skill-matrix.md) with
   `python tools/workflow/result.py --matrix`.
4. Parse the TOML and check identity, skill ownership, runtime defaults,
   affected rows and local links. Check loading in a fresh project session
   where available. Report static validation and observed runtime evidence
   separately. Invoking the new role is a separate selected operation.

Return the role, scope, skill bindings, reasoning defaults and verification
evidence. If the role needs a new skill, report that as a separate operation for
the caller. Stop at the role definition and its registration.
