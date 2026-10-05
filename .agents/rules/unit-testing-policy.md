# Unit Testing Policy

## Placement and Coverage

- Rust tests live in `crates/<crate>/tests/`, never inline in `src/`. Test filenames start with `test_`; shared helpers live in `tests/common/mod.rs`.
- Cover domain logic with tests named for the behavior or module. Thin declarations and forwarders may rely on parent integration coverage.
- Add or update tests when behavior changes or existing coverage does not demonstrate the requirement. For mechanical moves, renames, formatting, or comments, run the relevant existing tests; do not edit tests merely to satisfy file coupling.
- Evaluate assertions by the behavior and failure modes they verify. There is no minimum assertion count or arbitrary number of test functions per crate. Each crate retains an active external test suite.
- In the completion report, identify the relevant verification and any coverage gaps. A changed test file alone proves no coverage.

## Four Verification Tiers

1. **Unit:** Isolated crate behavior; `python tools/harness/run_tests.py --unit`.
2. **Integration:** Machine coordination, debugger, and headless GUI; `python tools/harness/run_tests.py --integration`.
3. **CPU and bus silicon verification:** Tom Harte SingleStepTests and Cartesian DMA contention in `test_runner`.
4. **Whole-machine verification:** vAmigaTS hardware captures through the vAmiga runner.

Architecture checks are a separate cross-cutting gate. Commands and snapshot handling are in [test-runner](../skills/test-runner/SKILL.md).

## Integration and Defect Resolution

- Maintain machine integration coverage when a changed chip gates execution, transfers Chip RAM autonomously, raises inter-chip signals, or competes for bus slots. Extend existing tests when they already cover the coordination path.
- For defects, follow [repro-first.md](repro-first.md): confirm a focused failing test, fix the cause, verify the test and domain regressions. An existing isolated failing test can serve as the reproduction.
- Remove test-only zombie runtime methods unless they intentionally expose host input, output, or debugger controls; see [audit-code-quality](../skills/audit-code-quality/SKILL.md).

See [Testing Strategy and Quality Assurance.md](../../Obsidian/Amiga/Design/Testing%20Strategy%20and%20Quality%20Assurance.md) for the broader verification model.
