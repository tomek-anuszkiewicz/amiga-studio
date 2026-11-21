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

Map approved requirements and chip/module interactions to existing evidence before
proposing tests. Identify a small set of meaningful scenarios, suitable levels,
independent expected results, observation methods and concrete gaps. Prefer reused
behavioral/integration fixtures to a new framework. Do not impose assertion-count,
per-function or exhaustive catalogue quotas. Thin declarations and forwarders may
be protected through parent integration coverage.

For each proposed test, name the current project use, observed regression or
approved requirement it protects and the effect a failure would have. Apply
this to tests of verification tools as well. A fixture may force a rare failure
to make it repeatable. Identify which conditions the fixture supplies and which
are reachable through supported use or covered by the approved requirement.
Report the demonstrated consequence at its actual severity.
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

The four verification tiers are isolated crate behavior, machine/debugger/GUI
integration, CPU reference vectors, and whole-machine hardware captures. Capture
execution is currently unavailable. Every executed SingleStep vector compares
final state, instruction clocks and timed bus transactions with its JSON reference;
sampling only changes the number of inputs. Report harness limits explicitly.
No weakened comparison modes or unverified-success skips are permitted. The
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

For an assigned coverage assessment, inspect canonical coverage's unexecuted
functions and source spans with the final measured source. Coverage is execution
evidence, not correctness; assess the assertions protecting the observed behavior.

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

Separate behavioral tests, static architecture checks, performance measurements and
diagnostic traces. Source-inspection checks belong in verification tooling;
measurements in benchmark targets. Generated diagnostics, including the per-attempt
timing report, stay ignored. Keep the report locator and digest for archival
provenance. The report stays outside reusable gate artifacts and supplies no pass evidence.

When a change adds tests or alters how they run, use the original attempt's
timing report to check the affected suite. When the runner reports per-case
elapsed times, identify the slowest changed cases. Otherwise, report suite or
command elapsed time and state that case timing is unavailable. Compare suite
elapsed time with a comparable earlier run when available. Report material
increases or unexpected serial execution, including the affected scope,
execution mode and observed cost.
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
