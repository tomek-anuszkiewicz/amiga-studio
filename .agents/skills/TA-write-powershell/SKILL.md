---
name: TA-write-powershell
description: Implement assigned PowerShell tooling with its intended invocation, literal-path handling and focused validation.
---

# Implement the selected PowerShell change

The `TA-implementer` receives the approved behavior and test scenarios,
the selected script paths and the scope for exercising the script.
Return results using the [result contract](../../references/workflow-contract.md).

1. Use typed parameters and follow nearby script conventions. If the tool needs
   an action or input, a call with no arguments prints usage, succeeds and does
   no work. If the tool has a complete single-purpose default, that call runs
   the default. Keep usage at the no-argument invocation and omit extra help
   switches and aliases. For invalid input, report the problem and relevant
   usage, then exit with a nonzero status. Changes to internal APIs,
   configuration or formats follow the shared
   [internal migration contract](../../references/internal-migrations.md) and
   [specification compliance guidance](../../references/spec-compliance.md).
2. Alongside the implementation, explain what each parameter does, its default
   and constraints, and any changes to data. Include executable examples and keep
   output aligned and readable without color. Maintain one source for the
   manual. Have the script print usage before parameter binding can prompt.
   When the assignment includes companion documentation, apply the relevant
   [document/instruction ownership](../../references/information-hierarchy.md)
   and [prose criteria](../../references/narrative-criteria.md).
3. Validate inputs first. Infer an action from a modifier only when its
   documentation gives it one meaning. Use task-specific variables, quote and
   compose paths, and use `-LiteralPath`. Keep file operations in one PowerShell
   shell. Propagate failures from external tools and check resolved target paths
   before recursive moves or deletions.
4. Use `System.Management.Automation.Language.Parser.ParseFile` to parse the
   script without running it. Check that the declared parameters match the
   usage text. Then exercise the no-argument path, a representative valid
   invocation and invalid input where each is safe and within scope. Use
   temporary outputs. Keep help inspection read-only if a default outside the
   assignment can change data.
5. Return the changes, observed parsing and invocation results, and remaining
   work in text and JSON. Address assigned review findings using the criteria
   they cite.

The assignment ends after implementation and its checks. Keep script execution
within that scope and leave installation and commits outside the assignment.

Reviewers use the [PowerShell acceptance criteria](references/acceptance.md).
Authors consult the detailed criteria only when they need them to address an
applicable finding.
