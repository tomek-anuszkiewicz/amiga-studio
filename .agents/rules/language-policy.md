# Language policy

Answer in the user's language and persist repository artifacts in English.
`.agent/tasks/` is always excluded from language/vocabulary checks, even when named
explicitly. Translate user concepts into idiomatic English rather than copying
untranslated prompt wording into repository artifacts.

Batch preparation requires the full repository-owned text scan through the canonical
commit profile, including unchanged and nonignored new files. Milestone preparation
retains that gate. Matching evidence may be reused; ordinary turns, generic validation
and document edits do not trigger scans or vocabulary work. An explicit scoped audit
selects its files without sweeping unrelated pending changes. Commands and dictionary
mechanics belong to [agent tooling](../references/agent-tooling.md#recording-and-validating-changes).

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
Unresolved findings/scan errors block readiness. Continue the same canonical batch
after fixes, retaining valid independent evidence; do not duplicate a full scan or
emulator tests for vocabulary-only corrections. An explicit standalone audit repeats
its scan after corrections. Summarize automatic fixes and additions with reasons.
