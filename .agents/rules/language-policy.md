# Language policy

Answer in the user's language. All roles write repository artifacts in English,
including saved plans and both text and JSON agent result files. This requirement
applies regardless of the conversation language, including Polish.

The `.agent/tasks/` exclusion applies only to automated language and vocabulary
checks, even when paths are named explicitly. Translate user concepts into
idiomatic English when writing repository artifacts.

Run one passing full English scan of the exact sealed commit in the task or
assembly checkout before conditional publication to local `master`. The scan
includes unchanged authored files and new nonignored files. The commit and
milestone profiles keep their existing selection.
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
Ambiguous terms and scanner or read errors block publication. Retain the failed
attempt and retry the full scan after resolving a transient error. Commit any
source, vocabulary or exact-file literal correction in the task or assembly
checkout under the applicable verification and review rules. Seal the corrected
candidate and scan it again. The publisher checks the passing receipt before
updating local `master`, and closure uses that receipt for graph and file
preservation. See the [sealed candidate scan](../../tools/verification/README.md#scan-the-sealed-candidate).
A standalone language audit repeats its scan after corrections. Summarize reviewed
additions and their reasons.
