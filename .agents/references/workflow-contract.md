# Skill result contract

The user selects outcomes, operations and any combined sequence through ordinary
language or named skills. The main session follows the routing and subagent
settings in [AGENTS.md](../../AGENTS.md#user-selected-operations). Perform only the
assigned scope and respect existing authorization. A result does not launch another
skill, require a lifecycle stage or grant permission to commit, merge, push or delete.

## Inputs and outputs

Each repository skill returns a concise explanation in the user's language and a
JSON object following [the result schema](../../tools/workflow/result.schema.json).
Use English in the JSON and any saved repository artifacts. The two forms describe
the same outcome, including failures and unfinished work.

Required fields:

- `skill`: the selected skill name; this is not an executable registry.
- `status`: `completed`, `partial`, `blocked` or `failed`.
- `summary`: what actually happened.
- `changed_paths`: repository-relative files changed by this operation.
- `checks`: actual commands and their `passed`, `failed`, `not_run` or
  `interrupted` status, summary and optional exit code.
- `findings`: concrete problems found within the assignment.
- `remaining_work`: unfinished work, missing inputs and decisions still needed.

Use empty arrays where appropriate. An assessment can complete with findings;
a completed assessment does not mean its subject passed. Never describe an unrun
check as passing. State the relevant file, behavior and consequence in findings.
Include useful output paths in check summaries without inventing file hashes,
candidate seals or execution identities.

Return JSON directly in the conversation unless the user or selected tool needs a
saved result. A separate matching Markdown file, automatic renderer, validation
command, round ledger and guidance record are not required. The schema describes
format; it does not verify claims or select a next step. Report missing inputs
and stop the dependent work. Respect a round limit only when supplied for the
assignment.

## Independent operations

Planning, implementation, review, verification and Git operations remain available
when selected. A selected compound skill includes its stated internal steps and
component operations. It does not invoke the next task stage automatically. A broader
assignment may explicitly include several stages. State actual checks and
outstanding work at the end; do not impose an automatic transition.
Telemetry registration and execution archives use the separate
[capture tools](../../tools/workflow/capture/README.md) when selected.
