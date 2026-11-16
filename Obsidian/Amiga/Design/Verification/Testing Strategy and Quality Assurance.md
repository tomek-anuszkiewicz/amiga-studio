---
title: "Amiga 500 Testing Strategy and Quality Assurance"
aliases: ["Testing Strategy", "QA Strategy", "Four-Tier Verification Architecture"]
tags: ["amiga", "design", "testing", "architecture", "qa"]
category: "Design"
subsystem: "testing"
status: "active"
created: 2026-09-14
updated: 2026-11-16
related: ["[General Architecture.md](../General%20Architecture.md)", "[CPU SingleStepTests.md](CPU%20SingleStepTests.md)", "[CPU Instruction Benchmarking.md](../Performance/CPU%20Instruction%20Benchmarking.md)", "[Machine Loop.md](../Machine%20Loop.md)", "[Platform Quirks and Invariants Catalog.md](../Platform%20Quirks%20and%20Invariants%20Catalog.md)"]
tracked_paths:
  - "crates/test_runner/tests"
  - "tools/verification"
  - "tools/bootstrap/tests"
  - "tools/bootstrap/conversion/tests"
  - "tools/bootstrap/html-to-markdown/tests"
  - "tools/bootstrap/pdf-to-markdown/tests"
  - "tools/bootstrap/book-to-markdown/tests"
  - "tools/amiga-rag-mcp-server/tests"
  - "tools/artifacts/tests"
  - "tools/blep_generator/tests"
---

# Amiga 500 Testing Strategy and Quality Assurance

## Completion entry point

Completion uses [verify.py](../../../../tools/verification/verify.py) with
`commit` for affected ordinary crate tests with coverage, quality gates,
standard harness suites when affected, Python tool contracts and the full English audit.
Selection includes staged, unstaged and nonignored new paths against the current
`HEAD`, including both endpoints of renames. Repeating the command checks the
whole pending change, including earlier edits still awaiting commit. Matching
successful evidence is reused automatically; a changed `HEAD` starts a new batch.
Local integration explicitly selects the complete candidate against an immutable
primary-branch commit with `--integration-base`, including committed task work.
This comparison base remains separate from HEAD, source and index stability.
Divergent synchronization requires full ordinary-workspace coverage and standard
harness suites. The base and selected paths appear in plans and completion records.
An explicitly requested rebase also supplies the immutable original task tip with
`--integration-source`, preserving divergent classification after history rewriting.
This tip participates in batch identity and freeze-time selection revalidation.
After final approval, the [local integration protocol](../../../../tools/verification/README.md#local-integration)
freezes the candidate and prepared partition. It validates the exact resulting
commit sequence without repeating expensive checks solely for mechanical HEAD
changes. This narrow bridge permits no general changed-HEAD evidence reuse.
Conditional reference publication installs only the sealed tree; interruptions
require recovery before source cleanup. A new primary baseline requires a newly
prepared combined candidate and conservatively revalidated evidence.
The [integration rule](../../../../.agents/rules/git-merge-commits.md#authorization-and-preparation)
requires approval of a concrete plan before mutation and owns bounded exceptions:
pending task changes may receive an explicitly unverified local working commit
before synchronization, with verification deferred to the complete integrated
result; intermediate real merges may likewise await the batch's final combined
verification and closure record; a clean unchanged single-source fast-forward may use affirmative confirmation that
its exact tip was already tested. The latter uses `integrate accept-tested` and
records confirmation separately from canonical test evidence. It runs no new
verification or review and creates no commit. New content requires the canonical
path; unknown test status is never inferred from the mere existence of a commit.
Several selected sources use one approved immutable source set, one assembled
candidate and one canonical profile against `B`. Intermediate merge history is
included in the recorded HEAD before verification; the frozen partition covers
only remaining mechanical commits. Review binds the selected source-set digest,
and publication locks all selected refs for one conditional `B -> C` transition.
Cleanup checks each source independently against its expected tip and retains
per-source progress in the surviving target checkout's control record. Later work
blocks only the affected cleanup; retry requires no new tests or commits. The
[integration command reference](../../../../tools/verification/README.md#selected-source-set)
owns the record and CLI details.
Cargo metadata identifies owning workspace packages and
their transitive consumers through normal, development and build dependencies,
including optional and target-specific declarations. All ordinary tests of the
selected packages run once under coverage; dependencies still compile as needed.
Independent documentation and known Python tool changes skip Rust tests and
coverage. Language data in `tools/verification/checks/english_words/*.txt` and
`tools/verification/checks/check_english.json` also selects no Rust tests and does
not expand the scope selected by other changes. Known orchestration inputs also
select no Rust tests: `verify.py`, `execution.py`, `evidence.py`, `batch_lock.py`
and `tests/test_evidence.py` under `tools/verification/`. Their Python contracts
still run, together with every existing quality gate. These inputs control
execution, logging, process ownership and evidence reuse; they do not expand
the package scope selected by concurrent emulator changes.
Changes inside a package, including fixtures, select that package and
its consumers. Manifest, lockfile, toolchain, Cargo configuration, verification
selection, command construction, coverage processing, check implementation and
design specification changes require the full Rust scope. This includes the CLI
grammar and Git change discovery because they determine selectors and the change
boundary. Other verification code and tests remain conservative; a new module or
renamed destination does not inherit orchestration ownership from its directory.
Unknown input ownership, unavailable metadata and a clean tree also select the
full Rust scope. This conservative fallback covers inputs outside Cargo's graph.
The worktree script retains its full Rust fallback even though its behavioral
contracts belong to the verification Python suite.
The plan explains selection and omissions. Coverage reports identify the selected
test packages and are not presented as whole-workspace measurements when partial.
Report totals include only current files in those packages; retired or unselected
source files retained in cached LLVM objects are excluded without deleting build caches.
Skipped coverage is not current evidence. Repeating the command can reuse a
successful coverage bundle only with matching inputs and intact archived reports,
under the [continuation contract](../../../../tools/verification/README.md#completion-verification).

During an explicit coverage task or a concrete diagnostic, `test coverage crate NAME` measures all ordinary tests
of one package without completion gates; `test coverage workspace` measures all
ordinary workspace packages. The same coverage pipeline produces a summary,
detailed LLVM data and a source-location report of unexecuted code. Reports
identify their selected packages and cover only the current build target and
instrumented code. Separate harness suites, doctests and branch coverage are
not included. Follow the [coverage policy](../../../../.agents/rules/unit-testing-policy.md#coverage-during-development-and-review)
to resolve behavioral gaps in changed emulator code and review its assertions
during preparation. Ordinary turns use focused checks without mandatory coverage
measurement. Full executable-code coverage is the goal; there is no
percentage gate or coverage exception list.

Formatting, Clippy, architecture, hardware/code audits, frontmatter and English
retain their full scope. Code-quality reference discovery includes production and
test JavaScript callers, preserving the distinction between host browser entry
points and test-only APIs. Commit also builds the production WebAssembly application
through [tools/web.ps1](../../../../tools/web.ps1), using its Nightly shared-memory
configuration. Cargo lint warnings fail that build. Native and Nightly compiler,
Trunk and launcher versions participate in execution conditions.

Each actual commit-profile invocation checks official Stable release metadata,
including when test evidence is reused. This informational check has a five-second
network timeout, compares only the installed Stable channel, and separately shows
the active verification compiler. Network or metadata failure does not block tests.
The notice is logged without becoming a gate or an evidence input; actual compiler
changes still invalidate execution conditions. `--plan` performs no network query. Cheap blocking gates precede coverage and expensive suites;
a failure pauses that work. Clippy follows static gates before coverage so its
findings also prevent avoidable test execution. Repeating the canonical command
automatically combines new results with valid successful groups for the same `HEAD`. It reruns failed,
missing or invalidated groups and rejects edits during execution. Coverage remains
one execution/export/summary bundle; no partial-report merging is performed.
The final verdict covers the complete current candidate, while each group's
reuse identity covers its inputs. `DIARY.md` is an editorial input to the English
gate, not to the known build, test and static-check groups. A diary-only correction
therefore preserves their successful results. Other documents, specifications
and unknown groups remain conservative; a Markdown extension alone never proves
independence. Every executing `commit`, `milestone`, `test`, `check` and `runner`
invocation holds one nonblocking operating-system lock at
`<checkout>/.agent/tmp/verification.lock`, acquired after argument parsing and
before selection, source snapshots, evidence writes or checks. A competing
invocation immediately fails with `[FAIL] Verification is already running in this
repository working directory.` Different profiles, branches and results directories
within one checkout share this lock; separate checkouts have independent locks.
Help and `--plan` neither acquire the lock nor change its file. The persistent
file's contents and existence never establish ownership. Normal exits, failures
and forced process termination release the lock without deleting or replacing
the file. Child commands do not inherit its handle.
After interruption, repeat the same command and results directory to retain
completed matching groups and rerun unfinished work. The verifier does not track,
wait for or terminate surviving children: a command left alive after its verifier
exits can overlap a later invocation. No process registry, ownership recovery or
force-unlock mechanism is provided. Direct Cargo, unittest and standalone helper
commands are outside this contract. Administrative `integrate` actions retain
their separate transaction protocol. The
[continuation reference](../../../../tools/verification/README.md#completion-verification)
describes evidence reuse and execution limits.
Verification-tool contracts run only when their code, tests or shared tooling
inputs change. Vocabulary and generated reports do not select these contracts.
Other Python contract selection remains as described below.
