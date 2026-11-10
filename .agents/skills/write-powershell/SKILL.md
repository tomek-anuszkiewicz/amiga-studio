---
name: write-powershell
description: Create or revise repository PowerShell scripts, including typed parameters, detailed console usage, no-argument behavior, and focused validation.
---

# Write PowerShell Tools

Use this skill when creating or changing PowerShell tooling in this repository.
Project scope and path requirements remain in [AGENTS.md](../../../AGENTS.md).

## Choose the Invocation Contract

- A parameterized tool that needs an action or input prints its usage when invoked
  without arguments and returns successfully without executing an action.
- A simple, single-purpose script with a complete default invocation runs without
  arguments. Do not add a usage-only barrier to that workflow.
- Keep usage at no-argument invocation; add no help switches or aliases.
- Invalid input reports the specific problem and relevant usage, with a nonzero
  exit code. Do not silently run a different action.

## Design Parameters and Explain Their Effects

Follow the existing script conventions for parameters and console usage.

Explain parameter effects, defaults, constraints and data changes in console usage.

Include executable examples for the ordinary invocation and meaningful parameter
combinations. Keep the display aligned and readable without relying on color.
Choose column widths from the actual option names rather than a fixed project-wide
width. Avoid descriptions such as "force mode" that omit the concrete effect.

Keep console usage beside the script in one implementation; optional comment help must not duplicate the manual.

## Keep Execution Explicit

- Validate inputs before performing work. Required parameter binding must not
  prevent the intended no-argument usage display by prompting for missing values.
- Infer a parent action from a modifier only when there is one clear, documented
  action. Do not let a modifier such as `-Force` silently expand execution scope.
- Quote paths, use `Join-Path` for composition, and use `-LiteralPath` when processing
  supplied filesystem paths. Keep filesystem operations within PowerShell rather
  than constructing deletion commands for another shell.
- Use task-specific variables, never automatic or environment variable names.
- Propagate external tool failures; keep tooling outside emulation.

## Validate the Changed Behavior

Parse the script without executing it:

```powershell
$ScriptPath = 'tools/example.ps1'
$Tokens = $null
$ParseErrors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path -LiteralPath $ScriptPath).Path, [ref]$Tokens, [ref]$ParseErrors
) | Out-Null
if ($ParseErrors.Count -gt 0) {
    $ParseErrors | ForEach-Object { Write-Error $_.Message }
    exit 1
}
```

Check the parameter declarations against the displayed descriptions. Exercise the
no-argument path, one representative valid invocation, and invalid input when
these runs are safe within the authorized task. Use temporary output paths for
mutating checks. Do not execute a simple script merely to inspect its usage if its
default action would perform unrelated work.

Use existing tool contracts when they cover the changed behavior. Completion and
commit verification follow [git-commits.md](../../rules/git-commits.md); this skill
does not authorize a commit or prescribe extra completion suites.
