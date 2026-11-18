---
name: TA-write-powershell
description: Implement assigned PowerShell tooling with its intended invocation, literal-path handling and focused validation.
---

# Implement the selected PowerShell change

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-implementer`; result kind: `implementation`.
Inputs: approved behavior/test scenarios, selected script paths and invocation scope.

1. Use typed parameters and nearby script conventions. An action/input tool prints usage and succeeds
   with no arguments without doing work; a complete single-purpose default runs.
   Keep usage at no-argument invocation, with no extra help switches or aliases.
   Invalid input reports its problem/usage and exits nonzero.
2. Explain parameter effects, defaults, constraints and data changes beside the
   implementation, with executable examples and aligned output independent of color.
   Avoid duplicate manuals or parameter binding that prompts before usage can print.
3. Validate inputs first. Infer an action from a modifier only where one documented
   meaning exists. Use task-specific variables, quoted/combined paths and
   `-LiteralPath`; keep file operations in one PowerShell shell. Propagate external
   failures and verify resolved targets before recursive moves/deletions.
4. Parse with `System.Management.Automation.Language.Parser.ParseFile` without
   executing. Inspect parameter/help agreement, then exercise the no-argument path,
   a representative valid invocation and invalid input only where safe within scope.
   Use temporary outputs; do not execute an unrelated mutating default to read help.
5. Return changes and actual parse/invocation evidence. Each edit/check pass is a
   round. Apply assigned review findings through their cited criteria.

Stop after implementation/checks. No unrelated execution, installation or commit.

Reviewers use the [PowerShell acceptance criteria](references/acceptance.md);
authors consult that detail for an applicable finding rather than loading it by default.

