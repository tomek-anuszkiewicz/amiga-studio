# Testing Policy

- Rust tests live in `crates/<crate>/tests/`, never inline in `src/`. Test filenames start with `test_`; shared helpers live in `tests/common/mod.rs`. Each crate retains an active external test suite.
- Cover domain behavior and failure modes. Thin declarations and forwarders may rely on parent integration coverage; there is no minimum assertion count or arbitrary test-function quota.
- Add or update tests for behavior changes or coverage gaps. For mechanical moves, renames, formatting, or comments, run relevant existing tests instead of editing tests solely for file coupling.
- Maintain integration coverage when chips gate execution, transfer Chip RAM, signal peers, or compete for bus slots. Extend existing tests when they already exercise that coordination.
- For defects, follow [repro-first.md](repro-first.md). Remove test-only zombie runtime methods unless they intentionally expose host I/O or debugger controls.
- Verification has four tiers: isolated crate behavior, machine/debugger/GUI integration, CPU and bus silicon verification, and whole-machine hardware captures. Architecture checks are a separate gate.
- Use [test-runner](../skills/test-runner/SKILL.md) for commands and snapshot handling. Report actual checks and coverage gaps; changed test files alone prove no coverage.

The broader model is in [Testing Strategy and Quality Assurance](../../Obsidian/Amiga/Design/Testing%20Strategy%20and%20Quality%20Assurance.md).
