# Testing Policy

- Rust tests live in `crates/<crate>/tests/`, never inline in `src/`. Test filenames start with `test_`; shared helpers live in `tests/common/mod.rs`. Each crate retains an active external test suite.
- Cover domain behavior and failure modes. Thin declarations and forwarders may rely on parent integration coverage; there is no minimum assertion count or arbitrary test-function quota.
- Add or update tests for behavior changes or coverage gaps, subject to the bootstrap converter scope below. For mechanical moves, renames, formatting, or comments, run relevant existing tests instead of editing tests solely for file coupling.
- Maintain integration coverage when chips gate execution, transfer Chip RAM, signal peers, or compete for bus slots. Extend existing tests when they already exercise that coordination.
- For defects, follow [repro-first.md](repro-first.md). Remove test-only zombie runtime methods unless they intentionally expose host I/O or debugger controls.
- Verification has four tiers: isolated crate behavior, machine/debugger/GUI integration, CPU and bus silicon verification, and whole-machine hardware captures. Architecture checks are a separate gate.
- Use [test-runner](../skills/test-runner/SKILL.md) for commands and snapshot handling. Report actual checks and coverage gaps; changed test files alone prove no coverage.

## Bootstrap Converter Scope

Applies to `tools/bootstrap/conversion/`, `tools/bootstrap/pdf-to-markdown/` and `tools/bootstrap/html-to-markdown/`. Follow the [developer-led conversion workflow](../../tools/bootstrap/reference-conversion-contract.md#development-workflow); emulator hardware and CPU verification requirements are unaffected.

- Keep existing technical tests for configuration, Codex transport, response/cache validity and restart/artifact preservation. These tests do not certify conversion quality. Run relevant existing checks for a concrete change; do not turn every edit into a full converter test campaign.
- Do not add tests by default or create coverage matrices, exhaustive stage/branch suites or automated conversion-quality evaluations. A new test must address a concrete technical defect within the requested task, rather than hypothetical coverage or the current pipeline's internal layout.
- Do not restore the removed asset/crop tests or introduce pixel-perfect crop assertions unless the user explicitly requests that testing scope. When redesign supersedes a mechanism, remove or adapt its obsolete tests with it; do not preserve old behavior solely to satisfy old tests.
- Fragment conversion runs occur at the user's request. The user selects the source fragment and evaluates the generated content; report execution failures and output locations. Do not independently choose samples, expand to the full book/crawl, compare models or tune quality. Runtime input/output/schema validation remains part of converter execution.
- Required repository per-commit checks still apply. Report their actual results separately from fragment conversion and user quality assessment.

The broader model is in [Testing Strategy and Quality Assurance](../../Obsidian/Amiga/Design/Testing%20Strategy%20and%20Quality%20Assurance.md).
