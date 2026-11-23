# Skill result contract

The user chooses the outcome, the operations and any sequence of operations
through ordinary language or by naming skills. The main session uses the
routing and subagent settings in [AGENTS.md](../../AGENTS.md#user-selected-operations).
Work within the assigned scope and existing authorization. A result alone
triggers no other skill or lifecycle stage and grants no permission to commit,
merge, push or delete.

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

Use empty arrays where appropriate. Completing an assessment means the assessor
finished the assignment. Include any problems the assessor found. Report a check
as passing only when it ran and passed. Give the relevant
file, behavior and consequence for each finding. Include useful output paths in
check summaries without inventing file hashes, candidate seals or execution
identities.

Return JSON directly in the conversation unless the user or selected tool needs a
saved result. A separate matching Markdown file, automatic renderer, validation
command, round ledger and guidance record are unnecessary for this contract.
The schema defines the format and leaves verification of claims and selection
of the next step to the authorized work. Report missing inputs and stop the
work that depends on them. Respect a round limit only when the assignment
supplies one.

## Independent operations

### Vocabulary-only maintenance

Contextually justified additions to the configured English word lists and
exact-file `file_words` allowances are ordinary maintenance within authorized work.
The current assignee may make them directly without a separate permission request
or a new assignment. This applies during ordinary work as well as a selected
language operation.

The change must consist only of adding valid words, required inflections or
necessary literal exceptions. Resolve their meaning in the existing context
before adding them; dictionary absence alone is not a language defect. Keep
ordinary prose errors out of the lists. Changes to scanner logic, file discovery,
exclusions, patterns, gates, code or authored text are outside this exception.

After adding the entries, continue from the current selected stage. Do not start
or repeat a language scan, tests, builds, coverage, quality or documentation
checks, reviews, analysis or evidence validation because of the addition. A
request to add vocabulary needs no preliminary scan either. Do not count this
maintenance as a corrective pass or trigger targeted/final review rounds.

Accepting a word in context can resolve an unknown-word finding from a completed
scan. Keep the original scan outcome and report the added entry. State that the
scan did not run again. Existing checks of unchanged code or text still apply. When the assignment includes other changes, continue the work those
changes independently require. Run an extra scan only when the user
explicitly requests one or a separate authored-text change requires it.

Planning, implementation, review, verification and Git operations remain available
when selected. A selected compound skill includes its stated internal steps and
component operations. It does not invoke the next task stage automatically. A broader
assignment may explicitly include several stages. State actual checks and
outstanding work at the end; do not impose an automatic transition.
Telemetry registration and execution archives use the separate
[capture tools](../../tools/workflow/capture/README.md) when selected.

### Validated vocabulary removals

When the engineer explicitly selects [vocabulary optimization](../skills/TA-WF-Step-optimize-english-vocabulary/SKILL.md),
the assignee may remove the exact batch of declarations accepted by the read-only
validator on complete repository evidence. The validator must show that the
batch leaves the same unknown words at the same source occurrences. Recheck
discovery and input contents before editing. Repeat validation when either changes.
Run a full scan of the edited tree and compare the observed findings with the
validated outcome. This vocabulary maintenance requires no emulator tests.

Keep incidental removal candidates in the findings of the selected language
operation. The [protected verification rules](protected-verification.md#protected-verification)
govern all other removals and changes to the checker or its policy.
