# Validation Execution

- During iteration, run targeted tests for the changed behavior. Run required full domain verification before task completion, as specified by [AGENTS.md](../../AGENTS.md).
- Run independent checks concurrently when they do not share mutable inputs or contend for one Cargo target directory. Background execution must not race edits to the source snapshot being validated.
- Preserve session identifiers for unfinished commands, continue independent work, and collect results before reporting success. Avoid busy polling and keep the user informed during long checks.
- Keep successful output concise while retaining failures and actionable diagnostics. Do not suppress exit codes or evidence needed to assess a result.
- Use [test-runner](../skills/test-runner/SKILL.md) for filters, full-suite commands, snapshots, and result handling. Delegate only when authorized by the user or applicable instructions; parallel shell checks do not require subagents.
