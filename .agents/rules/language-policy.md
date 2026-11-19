# Language policy

Answer in the user's language. All roles write repository artifacts in English,
including saved plans and both text and JSON agent result files. This requirement
applies regardless of the conversation language, including Polish.

The `.agent/tasks/` exclusion applies only to automated language and vocabulary
checks, even when paths are named explicitly. Translate user concepts into
idiomatic English when writing repository artifacts.

Ordinary task closure runs one full English scan in the primary checkout on local
`master` after publication. The scan includes unchanged authored files and new
nonignored files. The commit and milestone profiles do not select the scan.
An explicit scoped audit selects its files without sweeping unrelated work.
Commands and dictionary mechanics belong to [agent tooling](../references/agent-tooling.md#recording-and-validating-changes).

The authoritative scope is `tools/verification/checks/check_english.json`: generated
outputs, caches, vendor/imported collections and `tools/bootstrap/books/` remain
excluded at their directory boundaries, while authored code/tests/prompts/docs stay
covered. Add newly authored text formats where appropriate; do not exclude authored
content to hide findings. Lockfiles/binary formats are not scanned. Explicit paths
bypass ordinary exclusions but never always-excluded directories. The scanner's
configured word splitting, dictionary/vocabulary and worker limits remain mechanical
requirements; a scope/logic change needs protected-verification authorization.

## Resolving Findings

The user has standing authorization for clear contextual corrections and justified
vocabulary entries. Read each finding in context: dictionary absence, uppercase or
source occurrence alone does not establish invalidity or validity. Verify unfamiliar
technical terms against local definitions/API usage or authoritative sources. Fix
clear spelling/non-English/awkward prose without unrelated rewrites or meaning changes.
Keep appropriate valid wording and add reviewed terms rather than distort it for a gate.

Preserve API names, register/bit terms, quotations, test literals and persisted keys
whose spelling matters. Valid identifiers need not be renamed for a dictionary;
behavior-preserving local word-boundary/spelling corrections may update callers.
Maintain one explicit reviewed term per vocabulary line in the appropriate list;
inflections need their own entries. Never bulk-copy findings or whitelist misspellings.
Deliberate invalid/non-English examples use justified exact-file `file_words`, not
global vocabulary. This authority does not change behavior, contracts, goldens,
scanner logic, authored scope or other acceptance criteria.

Resolve clear findings first. Report only remaining ambiguity with file, line,
context and options; missing meaning or unverifiable terminology requires a decision.
During closure, ambiguous terms, source corrections and scanner or read errors
keep the published task open and block cleanup. Report the cause and retain the
worktree. An actual typo or required source edit goes through a separately selected
correction. With only justified vocabulary and exact-file literal additions, use
the [published closure procedure](../../tools/verification/README.md#post-publication-closure)
to create the one authorized vocabulary commit on the same `master`. Preserve
external changes if the recorded input moves or conflicts. After the commit,
proceed to the graph without another scan, verifier, tests, reviews or diary edit.
A standalone language audit repeats its scan after corrections. Summarize reviewed
additions and their reasons.
