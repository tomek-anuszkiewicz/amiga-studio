# Validation Execution

- Validate coherent changes with targeted tests for affected behavior, not after every edit. Repeat checks only after affected input changes, failures or unresolved concerns. Full pre-commit and milestone verification follows [AGENTS.md](../../AGENTS.md); ordinary task completion does not trigger those profiles.
- Run independent checks concurrently when they do not share mutable inputs or contend for one Cargo target directory. Background execution must not race edits to the source snapshot being validated.
- Preserve session identifiers for unfinished commands, continue independent work, and collect results before reporting success. Avoid busy polling and keep the user informed during long checks.
- Keep successful output concise while retaining failures and actionable diagnostics. Do not suppress exit codes or evidence needed to assess a result.
- Use [test-runner](../skills/test-runner/SKILL.md) for filters, full-suite commands, snapshots, and result handling. Delegate only when authorized by the user or applicable instructions; parallel shell checks do not require subagents.
