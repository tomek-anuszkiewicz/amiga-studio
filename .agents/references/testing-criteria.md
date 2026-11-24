# Test design and evidence criteria

Use these shared standards for selected test design, implementation, audit or
tests/evidence review. Changes to protected checks or acceptance follow
[verification change authorization](protected-verification.md#protected-verification).
A report-only assessor never adds tests or starts missing measurements automatically.

## Test organization and isolation

Tests belong in the owning crate/tool's local `tests/`, never inline in production
`src/` or in a new global component-test directory. Cross-component tests belong
to the integration owner. Test names describe the scenario and expected result.

Design tests to run in parallel. Isolate writable files and state so execution
order and overlap cannot affect another case. Read-only inputs may be shared.
If full isolation is impossible or disproportionately complex or costly, mark
the case for serial execution and record why it needs exclusive execution.
The [runner contract](../../tools/verification/README.md#bounded-parallel-execution)
owns process isolation, bounded scheduling and exclusive serial execution.

## Scenario design

Before proposing tests, check which approved requirements and interactions
between chips or modules the existing evidence covers. Identify concrete gaps,
then choose a small set of meaningful scenarios. For each, pick a suitable
test level, an independent expected result and a way to observe it. Prefer to
reuse existing behavioral or integration fixtures when the alternative is a
new framework. Do not set quotas for assertions, tests per function or
exhaustive scenario catalogues. Integration tests of the parent component may
cover thin declarations and forwarders.

For each proposed test, explain which current use, observed regression or
approved requirement it protects and what would happen if the behavior failed.
Include that explanation for tests of verification tools. A fixture may force a
rare failure to make it repeatable. Identify the conditions the fixture supplies,
then identify which conditions can occur through supported use or are covered
by the approved requirement. Report the demonstrated impact at its actual
severity.
Add coverage for a future use when the change introducing it is in scope.

Expected results come from a contract, specification, external reference or explicit
invariant. An implementation-generated baseline can establish stability or a
relative property such as invariance under waiting; it cannot independently prove
absolute correctness. Preserve independent literal encodings when sharing a
production constant would hide an incorrect definition.

Use existing host APIs or snapshots for observations. A new runtime API solely for
a test needs a demonstrated observation gap and an intentional interface. Test-only
code may be an intentional Host I/O or debugger control; do not delete it merely
because production callers are absent. Shared setup belongs in local test helpers.

## Behavior and domain boundaries

Assert expected state, side effects, failures and unchanged state where relevant.
Merely executing a line or changing a test file is not behavioral protection.
Maintain production integration coverage when chips gate execution, transfer Chip
RAM, signal peers or compete for bus slots. A synthetic CPU bus does not establish
production bus or chipset scheduler correctness.

The four verification tiers cover isolated crate behavior, machine/debugger/GUI
integration, CPU reference vectors and whole-machine hardware captures. Capture
execution is currently unavailable. For each SingleStep vector it runs, the
harness compares final state, instruction clocks and timed bus transactions
with the JSON reference. Sampling changes only how many vectors the harness
runs. Report harness limits explicitly. All comparison modes must retain the
required checks, and any skipped case without verification cannot count as a
success. The
[CPU verification specification](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md)
owns vector fields, observation boundaries and documented harness limits.

For a CPU execution path absent from supplied vectors, design a handwritten
regression in `crates/cpu/tests/` with independent expectations. Check resulting
state and side effects, total CPU clocks and each observable bus access's CCK,
address, direction and value. Include wait-state retries when bus behavior changes.
Report missing reference-vector coverage; neighboring passes do not verify that path.

GUI tests distinguish behavior/integration assertions from visual confirmation.
Apply the [GUI criteria](gui-criteria.md) and reproduce the original interaction
before repair. Converter work instead follows its
[nested testing scope](../../tools/bootstrap/AGENTS.md#converter-testing).

## Coverage and adequacy assessment

Apply this section when the selected operation assesses coverage or test
adequacy. An instrumented Implement Task run records test outcomes and available
reports. Verify Master uses the measured evidence in its independent tests review.

For an assigned coverage assessment, use the canonical coverage results to
find functions and source spans that did not run in the measured final version.
Coverage shows which code executed. Read the test assertions to determine which
behavior each test protects.

Check measured suites, targets, conditional compilation, instrumentation and stale
object mappings before identifying a gap. Evidence from another target may explain
a missing span but must be supplied, not assumed. Full executable-code coverage is
the goal, not a global percentage gate. Uncovered reachable changed behavior or
affected interactions require meaningful protection before readiness. Report
pre-existing out-of-scope gaps separately; never hide them with exclusions,
allowlists, reduced measurement scope or meaningless assertions.

The Implementer may add meaningful cases, adjust fixtures and strengthen
assertions for approved behavior within the selected write scope, including gaps
identified by coverage or review. Each such case does not need separate approval.
New behavior, changed acceptance criteria, protected verification changes and
out-of-scope work still need their applicable decisions. Required unresolved gaps
remain visible. A separate `TA-code-reviewer` instance assesses tests and supplied
coverage; `TA-test-runner` performs a selected measurement without changing tests.

For each uncovered area, distinguish a real reachable behavior gap from stale or
incomplete instrumentation, protection supplied by another measured suite, and
pre-existing out-of-scope work. Explain the missing scenario and independent
expected observation before recommending a test. High execution coverage with
weak assertions can still leave a protection gap. Do not manufacture tests for
unreachable code or add assertions that merely mirror the implementation.

## Evidence review

Distinguish evidence from behavioral tests, static architecture checks,
performance measurements and diagnostic traces. Put source-inspection checks
in verification tooling and measurements in benchmark targets. Keep generated
diagnostics, including the per-attempt timing report, ignored by Git. Preserve
the report locator and digest so the archived report's origin can be traced.
The report stays outside reusable gate artifacts and supplies no pass evidence.

When a change adds tests or alters how they run, use the original attempt's
timing report to check the affected suite. If the runner provides elapsed times
for individual cases, identify the slowest changed cases. Otherwise, report the
available suite or command elapsed time and state that case timing is
unavailable. Compare suite elapsed time with a comparable earlier run when
available. Report material increases or unexpected serial execution, including
the affected scope, execution mode and observed cost.
For an expensive Git scenario, distinguish the cost of building its starting
repository from Git calls made during the scenario. Reuse a prepared fixture
when it supplies the required starting state.

Match actual commands, exit statuses, acceptance criteria and output artifacts to
the selected scope. Distinguish executed checks from inspected older reports.
Failed, missing, interrupted or unrun checks are not passes. Report what was
actually tested; do not infer coverage of other commits, hardware, platforms or
performance. Keep shared writable targets isolated during tests.

The verifier selects commands but does not freeze inputs, cache acceptance or
require lifecycle transitions. Preserve useful local outputs before worktree
removal through the [artifact tool](../../tools/artifacts/README.md).
