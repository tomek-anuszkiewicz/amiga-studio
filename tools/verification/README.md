# Repository Verification Reference

This reference lists the test commands and explains how the verifier selects
checks and reports their results. For an introduction to the evidence each kind
of test provides, read the [testing overview](../../docs/testing.md). Run commands
from the repository root. To list every command, invoke
`python tools/verification/verify.py` without arguments or with `--help`. To preview
the commands for a selection, use `--plan`.

## Command syntax

From the selected checkout, run `python tools/verification/verify.py` followed by
one of the command forms below. The installation provides no short `verify`
alias. Quote values containing spaces. Use each selection keyword at most once.
The verifier rejects unknown or incomplete command forms before running checks.

The `case` selector looks for a substring in the name, including in qualified
Python test names. To match the complete name for a focused Rust or SingleStep
`case`, use `--exact`. For an ordinary Rust test whose libtest name includes a
module path, supply that full name. See the [case reporting instructions](#case-reports)
for associating a report with a task execution.

| Sentence | Scope |
| --- | --- |
| `commit` | Selected check profile; excludes full SingleStep |
| `milestone [singlestep]` | Workspace public function scan, then full SingleStep |
| `test rust [unit|integration|all|workspace]` | Diagnostic tiers; default is unit plus integration; workspace excludes test_runner |
| `test rust crate NAME [suite NAME] [case FILTER]` | One Cargo package, optionally one integration target and filter |
| `test coverage crate NAME` | All ordinary tests of one package with detailed coverage; excludes test_runner |
| `test coverage workspace` | All ordinary workspace tests with detailed coverage; excludes test_runner |
| `test python` | Tool contracts affected by changed owning inputs and shared dependencies |
| `test python directory PATH [file PATTERN] [case FILTER]` | One Python test directory and optional discovery filters |
| `test runner [standard|silicon]` | Standard harness by default (transaction checker regressions), or full SingleStep |
| `test runner singlestep [case FILTER] [full|sample N]` | Full vectors by default; explicit positive sampling limit overrides inherited environment |
| `check all` | All commit quality gates without crate coverage or ordinary tests |
| `check architecture [case FILTER]` | Python repository architecture checks; no Rust compilation |
| `check english [files PATH...]` | Full or selected-file English scan with a vocabulary report |
| `check english removals PATH` | Read-only validation of the combined effect of the proposed removals |
| `check code|hardware|formatting|clippy` | One quality gate |
| `check build [crate NAME]` | Cargo compilation check; workspace by default |
| `check web` | Build the browser application through Trunk and nightly; Cargo lint warnings fail |
| `check unused dependencies` | Check dependency declarations in all workspace packages |
| `check unused dependencies crate NAME` | Check dependency declarations in one workspace package |
| `check unused public crate NAME` | Find candidate unused public functions in one workspace package |
| `runner build` | Compile release test and benchmark targets without running them |
| `runner bench [case ID\|mnemonic NAME\|opcode XXXX] [duration-ms N] [series N]` | CPU performance test measurement |
| `runner gui -- OPTIONS` | GUI inspector capture |
| `runner analyze RUN_DIRECTORY` | Regenerate a stored run analysis without measurement |

`--plan` previews the exact child commands and their environment. `--log-level
full|warn|error` controls output. Both options may appear among the command's
selectors. `--results-dir PATH` puts verifier logs in `PATH/verify/`, SingleStep
reports and history in `PATH/singlestep/`, and public probe reports and build
logs under `PATH`. The verifier resolves relative paths
from the repository root. Give each experiment a new directory to keep its
SingleStep history separate. Other tools, including coverage and benchmarks,
keep their own output paths.
`--exact` is accepted only with `test rust crate NAME suite NAME case NAME` or
`test runner singlestep case NAME`; it does not change the selected build profile
or vector comparisons. The focused Rust command fails if the named suite runs
zero or multiple tests. In SingleStep, `case` names one instruction group and
`sample` controls how many vectors from that group run.
CPU benchmarks use the Cargo benchmark target described below.
The aggregate SingleStep test saves suite results and combines the reports once
all selected groups finish. This reporting step runs even when a group fails.
Only `runner gui -- OPTIONS` forwards native switches to a separate program.
Counts must be positive. The parser exits with code 2 before execution if it sees
unknown or repeated selectors or missing or invalid values.
The `test_runner` crate contains test and benchmark targets with shared helpers
in `tests/common/`; chipset baseline recording has no executable entry point.
Benchmark analysis prints full output by default. Other commands print warnings
and errors by default. Focused commands save full logs in `.agent/tmp/verify/`
by default. Profiles use the same logging path. Use `--results-dir` to select
another directory.

Examples:

```powershell
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_sub
python tools/verification/verify.py test runner singlestep case test_move_w sample 500
python tools/verification/verify.py test runner singlestep case test_add_b --exact sample 5 --results-dir .agent/tmp/experiments/add
python tools/verification/verify.py test python directory tools/bootstrap/tests file test_bootstrap_contract.py case test_all
python tools/verification/verify.py check architecture case test_file_size_limits
python tools/verification/verify.py runner bench inventory --plan
```

The generic Rust path preserves the selected Cargo test's normal environment.
An explicit `test_singlestep` target uses the launcher with the unoptimized
`test` profile. Dedicated SingleStep commands and milestone execution use
`verification`: release optimization with debug symbols, debug assertions and
overflow checks enabled. The launcher checks the CPU library and selected Cargo artifact's
settings and rejects configuration overrides that disable these checks.
Ordinary crate tests and coverage retain their existing profiles.
The dedicated SingleStep sentence explicitly chooses full or sampled execution.
Skill-owned scripts remain owned by their skills. Bootstrap, conversion,
indexing, worktree management, diary appending, tool installation and baseline
recording remain workflow operations with their own entry points.

For an assigned focused check, confirm that the tools and fixtures are available,
then run the exact documented command directly. The command needs no separate
execution skill. Keep the selected candidate stable and give concurrent
invocations separate writable build and report locations. Wait for the exit
status and inspect the reports from that invocation. Identify any failed,
interrupted, missing or unrun work. Authorization covers the selected check.
Get separate authorization for repairs, broader tests or coverage.
Use the [CPU diagnostic guidance](../../.agents/references/cpu-diagnostics.md)
when investigating an isolated instruction mismatch. Full standalone SingleStep
requires its explicitly selected complete scope; do not expand a diagnostic
sample into the full suite.

## Unused Rust dependencies and public functions

```powershell
python tools/verification/verify.py check unused dependencies
python tools/verification/verify.py check unused dependencies crate cpu
python tools/verification/verify.py check unused public crate cpu
```

The dependency commands run Cargo's compilation check for the selected
packages. They execute zero test suites and return a failing result when
Cargo reports an unused dependency in a selected package. Each finding names
the dependency and gives its manifest path and line. Coverage reads Cargo's
build diagnostics from its existing nextest invocation, including on a warm
build. The test outcomes are recorded separately from the dependency verdict.
Cargo determines which dependency tables and targets this check covers. See
[Cargo's unused dependency lint](https://doc.rust-lang.org/stable/cargo/reference/lints.html#unused_dependencies).

Install the public probe's Python dependencies with
`python -m pip install -r tools/verification/requirements.txt`. The probe runs
Cargo with `--offline`. Before running the probe, populate the Cargo cache for
the checkout's lockfile, for example with `cargo fetch --locked` from the same
checkout. If a missing dependency blocks the pristine build, the report records
the failed baseline and the comparison is unavailable for that feature selection.

The probe copies the current working inputs to a disposable workspace. For each
feature selection, it runs a pristine Cargo check and then checks all eligible
functions with crate-only visibility in one batch. A simple public re-export in
the selected package keeps the function public. The builds make `dead_code` a
warning so Cargo can reach workspace consumers, including integration test targets.
Narrowing an associated function can make Rust resolve its call to another
function and report a type error. If the call points to a single narrowed
function, the probe can restore that function's visibility and rerun the
check with the other edits in place.

A new warning identifies a candidate in the edited source. A precise reference
reported in any build excludes that function from the candidate list. Silence
is inconclusive. The `unused-public-*/report.json` file under `.agent/tmp/verify/`
records the functions, compiler diagnostics, build logs and consumers Cargo
reached. A retry confirms use when the call error disappears and Cargo
produces compiler output for every affected target. The report keeps both
build logs and identifies any unresolved errors. It also records omitted
configurations and source discovery gaps.
Inspect callers, re-exports, conditional code and external API obligations before
proposing a removal. Several functions narrowed together may become unreachable
as a group.

## English vocabulary

`check english` scans eligible tracked and nonignored new files. `check english
files PATH...` uses the specified files for diagnostics. A workflow English
scan always uses the full eligible working tree. Both modes skip
`.agent/tasks/` and the two Report Word Usage word lists. Each completed scan prints an `[INFO] English scan
report` line naming its UTF-8 JSON report under `.agent/tmp/english/`. The path
identifies this invocation and also appears in its command log. Use that path
when reading results from concurrent runs.

| Standalone exit code | Meaning |
| --- | --- |
| 0 | The scan passed. The report may still contain removal candidates. |
| 1 | The scan found unknown words or rejected a proposed batch. |
| 2 | A scan or verification error prevented a reliable result. An absent or invalid report also yields this status. |

`check all` runs the full English scan and records its pass or failure.
A request to run `check all` includes the English operation in the main
session. The session uses that invocation's scan report even on exit zero.
[Workflow English checks](../../.agents/references/workflow-contract.md#workflow-english-checks)

The report gives the scan `scope`, `requested_files`, `target_count`, `scanned_count`,
`complete` and `exit_code`. Read all entries in `findings` together.

- `unknown_word` gives the source path, line, word and preview.
- `unused_entry_candidate` gives the declaration identity and the reason it might
  be removable. Only a complete repository scan establishes absence across the
  selected corpus. A file scan can assess allowances for the owners it read.
- `scan_error` identifies the failed phase, affected path when known and
  diagnostic. The report retains observations from inputs already processed.

`deferred_owners` records allowances whose owners could not supply sufficient
evidence for removal. `inputs` records the eligible paths and input byte digests
for the current scan, along with the Git commits that establish owner deletions.

`check english removals PATH` reads a JSON array of the exact `entry` objects
from current removal candidates. A `word_list` entry identifies its list path
and one-based line. A `file_word` entry identifies the configuration path, the
exact owner and the zero-based array position. Both include the stored spelling
and normalized term. The check simulates removing all proposed declarations
together, including changes to the scanned list text. It compares unknown words
at their source occurrences. The report includes the checked entries, the
differences, any deferred entries and `validation.accepted`. Acceptance requires
complete evidence.
An accepted batch can return 1 if it preserves unresolved unknowns.

The checker and validator leave source files unchanged. Reports capture one
invocation's inputs and remain disposable. The main session applies validated
removals within the corrective English operation.
[Correct English](../../.agents/skills/TA-WF-Step-correct-english/SKILL.md) ·
[Validate and apply removals](../../.agents/skills/TA-WF-Step-optimize-english-vocabulary/SKILL.md#validate-and-apply-removals)

## Browser build verification

For standalone implementation and master repairs that change GUI code,
workspace manifests or the browser build pipeline, include
`python tools/verification/verify.py check web`. During
[Implement Task](../../.agents/skills/TA-WF-Main-implement-task/SKILL.md),
the runner uses a distribution built on the current inputs for a selected
browser interaction test. If it needs to build that distribution, the runner
runs `./tools/web.ps1 build` as part of the test assignment. Implement Task
defers standalone gates to final verification when selected.

Every `commit` profile selects the same `WebAssembly build` gate after
preflight and before coverage. The gate invokes `tools/web.ps1 build`, which
processes assets through Trunk and uses the production shared memory flags.
The launcher sets `CARGO_BUILD_WARNINGS=deny`, so Cargo lint warnings such as
unused workspace dependencies fail both ordinary web builds and the gate.
This does not change the centrally reviewed Clippy permissions. Non-lint Cargo
warnings are outside this Cargo setting's scope.

The build requires Trunk, PowerShell, and nightly Rust with `rust-src` and the
`wasm32-unknown-unknown` target, as described in the [launch instructions](../../README.md).
Missing tools or a failed build fail the selected verification. Every invocation
executes its selected build/check commands in the current environment.

## Case reports

Run tests and benchmarks directly from the intended checkout. Set both
`AMIGA_TEST_TASK_ID` and `AMIGA_TEST_EXECUTION_ID` to save a case report for an
assigned task. Unassigned runs discard case reporting and return their usual
test or benchmark results. See the [capture guide](../workflow/capture/README.md#case-reports).

When case reporting is enabled, the launcher prints the path to its Markdown report.
Open it to find the case attempts and durations for that invocation, with links
to the native event journal, command output and child reports. The existing
reports still provide command wall times and coverage.

The primary checkout retains these files when a linked worktree is removed.
See the [archive layout](../workflow/capture/README.md#case-reports) for
invocation metadata and interrupted-run recovery.

| Entry point | Case evidence |
| --- | --- |
| `verify.py test rust`, `test runner standard` | Cargo test binary and case or doctest identity, outcome and native failure output. Stable libtest leaves per-case elapsed time unavailable. |
| `verify.py test coverage`, commit coverage and direct nextest | Native nextest starts, outcomes and each reported attempt's elapsed seconds. |
| `verify.py test python`, `run_unittest.py`, `run_tests.py` | Unittest cases, native outcomes and monotonic timing. The parallel runner also captures subtest intervals and worker output. Standard discovery retains suite fixtures. |
| Book Review `run_tests.py` | Dependency setup, native unittest results and the suite's shared browser lifecycle. |
| `verify.py check architecture` | Named cases in the protected checker's existing unittest suite. Build and static scans without named cases report command status. |
| SingleStep and milestone | Original vectors, host seconds, expected guest clocks and comparator failure diagnostics. |
| `verify.py runner bench`, CPU `measure` and `run_mixed_scene.py` | Original outer attempt seconds and native series. A CPU retry retains its reason and evidence. |

The first command selects a timed Rust case. The second discovers Python tests
by filename.

```powershell
python tools/verification/run_rust_tests.py -- cargo nextest run -p config --test test_config -E 'test(=test_default_config)' --no-tests fail
python tools/verification/run_unittest.py discover -s tools/artifacts/tests -p test_report.py -q
```

The Rust launcher also accepts `cargo test` with native binary, profile,
ignored-test and output options. Use that route for Cargo doctests and its
scheduling behavior. The launcher uses libtest's named `pretty` format on the
console and in the attached native log when no format is set or when the caller
requests `--format terse`, `--format=terse`, or a libtest quiet flag. The same
Cargo run supplies the case names and outcomes for the report. If the native
output lacks individual results, the report retains any available suite counts
and marks the case evidence incomplete. Explicit libtest JSON or JUnit output
passes through to the native log with incomplete case evidence. When Cargo
exits with a named case still running, the report records an aborted attempt
with unavailable case duration and keeps Cargo's exit status. Nextest
provides native elapsed time for the final attempt. Earlier retry timings use
the precision of nextest's status lines when available, and diagnostics remain in
the native output. The report marks missing retry timings as incomplete.
For explicit Python IDs, pass the usual `unittest` module, class or method name
to `run_unittest.py`. Plain external Cargo and Python commands bypass these
repository launchers.

## Saved SingleStep reports

SingleStep runs place reports in `.agent/tmp/singlestep/` by default. With
`--results-dir PATH`, the verifier uses `PATH/singlestep/`. Inspect the output of
the actual run to establish the current verdict. Milestone verification requires
a new execution.

The single `#[test]` entry point calls `common::singlestep_suite::run_suite()`
to run the selected instruction groups on a limited number of worker threads.
The suite catches group assertion failures so each selected group can finish.
Each suite writer saves its own result and rotates its own history. Before the
final assertion, the same Rust process calls `common::reporting::update_global_summary()`
once to update `summary.json`, `summary.md` and `diff.md` in the selected results
directory. The summary lists totals, a table of all saved suites and the names
of all failing cases. The diff lists cases that moved from pass to fail or from
fail to pass, along with suites that have no previous result. Both reports order
the suites and cases for comparison.

Open the generated Markdown files directly after a SingleStep run. The reports
show the current run's status, report count and vector count separately. Saved
totals can span multiple runs because each suite retains its latest and previous
result. The invocation fails as incomplete if Rust completion or report metadata
is missing, the selection did not finish or zero groups match. If finalization
also fails after a test failure, the invocation retains the test failure status.
If the test process succeeds but reporting fails, the invocation still fails.

Full CMP and filtered CMPM have distinct report identities, for example
`Real68k::CMP.b` and `Real68k::CMP.b::CMPM`. Diffs require matching selection
identities and sample limits. Reports missing required fields such as `selection`
fail to load; parsed reports with an explicit empty selection are excluded from
comparisons. New and removed case names remain excluded from status-change comparisons.

Direct Cargo execution of `test_singlestep` also finalizes reports automatically.
The standard libtest runner sees one aggregate test. The existing verify `case`
selector matches group names inside Rust through `SINGLESTEP_CASE`; sample limits
still control vector counts only. For direct Cargo execution, select a group with
the same environment variable. Set `SINGLESTEP_EXACT=1` when the group name must
match exactly:

```powershell
$env:SINGLESTEP_CASE = "test_add_b"
python tools/verification/run_singlestep.py --profile verification
Remove-Item Env:SINGLESTEP_CASE
```

Set `SINGLESTEP_FULL=1` for all vectors; direct Cargo otherwise retains the
per-group sample defaults. `RUST_TEST_THREADS` controls group worker count,
defaulting to available logical CPUs. `SINGLESTEP_RESULTS_DIR` selects an isolated
report/history directory. Launchers reject concurrent use of the same directory
through `run.lock`; direct Cargo runs must also be kept separate. A killed or
aborted process cannot finish Rust aggregation; its metadata is incomplete and
older summaries do not establish completion for that run.
After forcibly terminating a launcher, inspect its process and `run.json` before
removing an abandoned lock. Build failure does not aggregate stale reports.
Complete test diagnostics are retained in `execution.log`; `--log-level full`
prints them after execution. `timing.json` separates already-built execution and
finalization; the launcher prints build/reuse time separately.

## CPU benchmark entry point

The active CPU measurement catalog is [benchmarks/m68k](../../benchmarks/m68k/README.md).
The command `tools/bootstrap/bootstrap.ps1 -Assembler` installs the pinned assembler.
Install the monitor dependency with `python -m pip install -r tools/verification/requirements.txt`.
The decoder inventory remains available for the separate coverage audit.

```powershell
python tools/verification/verify.py runner bench
python tools/verification/verify.py runner bench case add/add_w_d1_d0 duration-ms 50 series 1
python tools/verification/verify.py runner bench cases .agent/tmp/benchmark_cases.txt
python tools/verification/verify.py runner bench mnemonic ADD
python tools/verification/verify.py runner bench opcode D041
python tools/verification/verify.py runner refresh benchmarks/results/<run-id>
python tools/verification/verify.py runner bench inventory
python tools/verification/verify.py runner analyze benchmarks/results/<run-id>
```

Each measurement invocation creates one `benchmarks/results/<run-id>/` directory.
The manifest records the selected cases, effective duration and series count,
all attempts, host-load observations, failures and completion status. The Rust
runner executes complete guest programs and resets CPU and memory outside each timer.
The default is three series of at least 400 ms of timed execution per case.
The runner selects the lowest nanoseconds per target among the series as the
whole-workload cost. For an assigned task, the case report retains all series.
The measurement runs
the guest only as part of the workload, without a separate inspection run.
Assembly sources and binaries stay under `.agent/tmp/benchmarks/cpu_performance_tests/`.

During each measured series, the external monitor samples the pinned host core.
It samples all logical cores when affinity is unavailable. The Python benchmark
launcher uses those samples to subtract the benchmark process's CPU time from
host busy time. It treats more than 20% average external busy time in any series
or an unavailable observation as a load disturbance. One disturbed attempt triggers
one full-case rerun with the same settings. The manifest keeps both attempts, and
a second disturbance marks the result. If execution fails, the launcher records
a failed case and skips the retry.

On completion, the launcher automatically writes `aggregate.json`, `aggregate.csv`,
`anomalies.json`, `comparison.json`, `baseline_comparison.json`,
`cross_case_anomalies.json`, `a7_comparison.json` and a readable `report.md`.
Explicit analysis reads stored data and regenerates those reports without a new
measurement. Analysis and comparison require a complete run. For the comparison
with a previous run, the tool selects the latest earlier completed full run
available locally with a matching host and settings. Each case must also match
the source, configuration, image, target count and CPU clocks.

The baseline comparison uses the approved
`benchmarks/m68k_benchmark_baseline.json` and `.csv` directly, so it also works
when older local run directories are unavailable. It reports new, missing,
disturbed and incompatible cases separately from comparable timing changes.
The cross-case report compares matched ADD/SUB, ADDI/SUBI, ADDQ/SUBQ,
ADDA/SUBA, AND/OR, ANDI/ORI/EORI and NEG/NOT workloads. Matches require the
same target count, CPU clocks and normalized program shape. For canonical
addressing forms, operand size and source/destination direction are separated;
the gap at each mode is compared with the median gap across matched modes.
Other matched variants receive direct comparisons. The A7 report compares the
same instruction on A7 with matching A0-A6 workloads after normalizing register
numbers in the program. Unmatched cases remain visible in machine-readable data.
The Markdown report lists timing deviations of at least 5% when they also
exceed their measured series spread. It also lists baseline regressions,
incompatible cases, failed or load-disturbed measurements and series spreads
of at least 10%. It collapses the series-spread rows for quick review. These
thresholds flag measurements for diagnosis. They supply no pass/fail verdict or
estimate of statistical confidence. Since the whole-program times include helpers,
they cannot establish isolated opcode latency or CPU correctness.

The launcher automatically replaces the trackable
`benchmarks/m68k_benchmark_current.json` and `.csv` files only after a successful,
complete catalog run with default parameters. It also
writes `m68k_benchmark_anomalies.json`, `m68k_benchmark_baseline_comparison.json`,
`m68k_benchmark_cross_case_anomalies.json`, `m68k_benchmark_a7_comparison.json`
and `m68k_benchmark_anomaly_report.md`
beside them in `benchmarks/`. When committing the current snapshot, include these
reports for review. The run directory retains its own detailed copies.
Filtered runs and shortened development runs receive reports only in their run
directories. `runner refresh RUN_DIRECTORY` explicitly replaces matching cases in
the current snapshot from one completed, undisturbed filtered run made with the
current host, settings, workload fingerprint, target count and CPU clocks. The
current CSV and JSON remain paired by SHA-256; `case_sources` in the JSON records
each replaced case's run ID. The root anomaly reports are regenerated from the
combined current snapshot. A `cases FILE` selector reads one case ID per line,
so related instruction and addressing-mode workloads can be measured together
before refresh. Reanalyzing an older full run cannot replace newer root files.
The approved baseline remains tracked in Git. The runner never updates it.
After review and explicit approval under the reference-artifact procedure, the
complete current JSON/CSV pair may become the next baseline. This includes a
composite snapshot with documented per-case sources. Copy both files together,
regenerate the baseline comparison and root anomaly reports, and review the
resulting Git diff. Follow the normal commit verification and commit policy.

```powershell
Copy-Item -LiteralPath benchmarks/m68k_benchmark_current.json -Destination benchmarks/m68k_benchmark_baseline.json
Copy-Item -LiteralPath benchmarks/m68k_benchmark_current.csv -Destination benchmarks/m68k_benchmark_baseline.csv
git diff -- benchmarks/m68k_benchmark_baseline.json benchmarks/m68k_benchmark_baseline.csv
```

## Completion verification

Run selected profiles directly from the intended checkout:

```text
python tools/verification/verify.py commit
python tools/verification/verify.py milestone
```

These commands run bundles of checks. They leave commits and task closure to
their respective operations. The commit profile includes quality checks, the
browser build, ordinary tests and coverage, and selected Python tool suites. The
milestone profile first scans for candidate unused public functions in every
Cargo workspace package. It starts full SingleStepTests only after every probe
succeeds and produces a fresh, valid report with an empty candidate list. The
scan uses all workspace members, even on a clean tree or when the changes affect
only a few packages. A package with no eligible functions can pass. The probe
still recognizes expected compiler errors from its temporary visibility edits.
The commit profile omits this scan.

Verify prints each candidate's package, function, source location and report.
Check callers and re-exports and remove confirmed unused code. For a failed or
incomplete probe, consult the reported reason and any available build log. Each
sweep keeps its package reports in a new `unused-public-workspace-*` directory
under `.agent/tmp/verify/` or the selected `--results-dir <path>`. Rerun the
milestone profile after addressing the findings. Use `--plan` to preview the two
checks without executing them.

The caller chooses the checkout and keeps shared build and coverage outputs
isolated. Each invocation executes its selected checks and reports the commands
it ran, failures and work it did not run. The profiles provide no frozen trees,
saved verdicts, reuse of passing groups, entry-point batch locks, sealed commits
or Git publication commands. The verifier leaves review, commit, merge,
language-audit and cleanup authority to their respective operations.

### Generated timing report

Each ordinary verifier invocation writes a separate `test-runtime-report.md`
under `.agent/tmp/verification/diagnostics/`. The console prints the report's path
and content digest for archive lookup. `--results-dir` controls where the
verifier writes command logs, public probe reports and SingleStep outputs.
Each invocation still needs its own verdict.

The report records actual command wall times, failures and unrun work. Cargo
commands include measured build and execution time together unless the native
runner supplies a narrower breakdown. Coverage preparation, export and summary
have separate rows. Timing data does not certify a passing gate. Failed report
writing is reported as an incomplete invocation.

Native case reports and logs normally already survive in the primary checkout.
Preserve local timing reports, coverage and other ignored outputs with the
[artifact tool](../artifacts/README.md) before removing a task worktree.

### Bounded parallel execution

The verifier runs Bootstrap, Graphify, Conversion, HTML, PDF, RAG, Artifact,
Verification CLI and Workflow capture tests through `run_tests.py`. Each unittest
method runs in its own process with a private working directory and temporary
directory. The runner limits concurrent workers within the group and honors
explicit serial markers. The verifier finishes one group before starting the
next. Directory, file and case filters apply before workers start.

Book Review uses its dedicated runner and shared browser lifecycle. Explicitly
selected directories outside the registered groups use standard unittest
discovery. Rust test runners keep their native scheduling.

A failed preliminary or build check defers subsequent expensive phases. A failed
coverage step stops its bundle. Give independent coverage exports separate
writable counters. Separate verifier invocations share no batch lock or
saved-evidence protocol.

### Coverage-guided testing

Use these commands for selected whole-package or workspace tests with coverage.
They also support a diagnostic measurement without completion gates.

```powershell
python tools/verification/verify.py test coverage crate denise
python tools/verification/verify.py test coverage workspace
```

Both commands use the commit coverage pipeline to run all ordinary tests of the
selected packages. Choose the package command to measure the named package only.
Packages that depend on it are outside that selection. Use the workspace command
for an explicitly requested broad audit. Neither command accepts a suite or case
filter that could present a partial run as complete package coverage.

The pipeline writes three files under `.agent/tmp/coverage/`:

- `summary.json`: per-file and total line, function and region counts, with
  `verification_scope` identifying the test packages.
- `coverage.json`: scoped LLVM data, including function counts and source segments.
- `gaps.md`: per-file missing counts, unexecuted function entry locations and
  zero-count source spans, including partial-line gaps. Coordinates use
  `line:column` with an exclusive end; function entries combine instantiations
  at the same source location. Zero-count regions in individual function
  instances are also listed: they may execute in another instance even when
  the combined source view has no gap. Inspect their scenarios before adding tests.

All three reports and raw counters are cleared before measurement; build caches
remain. A failed coverage phase stops before exporting subsequent reports; other
commit checks still run. Reports include only current files in the selected
ordinary packages. They describe the current build target and instrumented code,
not every platform or conditional build. Code absent from instrumentation is not
proven covered. The Markdown report identifies source locations; LLVM's summary
counts remain authoritative because source spans are not the same as LLVM regions.

During [Implement Task](../../.agents/skills/TA-WF-Main-implement-task/SKILL.md),
the instrumented command supplies test outcomes and any available reports. That
stage defers assessment of coverage and assertions. For a selected standalone
assessment or [Verify Master](../../.agents/skills/TA-WF-Main-verify-master/SKILL.md),
read `gaps.md` with the corresponding production behavior and assertions.
Check whether an omitted suite already tests the behavior before declaring a
test missing. Use the [coverage criteria](../../.agents/references/testing-criteria.md#coverage-and-adequacy-assessment)
to identify meaningful tests or stronger assertions within the approved scope.
Report pre-existing unrelated gaps separately.

Refresh the measurement after code or test changes when the selected
assessment requires current evidence. Attribute results to the successful run
and its source and index inputs. Execution coverage shows what code ran.
Assertions, complete hardware scenarios and performance need their own evidence.

### Selection from changed inputs

The verifier uses staged, unstaged and nonignored new paths to choose checks.
That choice saves no snapshot and gives no evidence that the source stayed
stable. The commit profile uses `impact.py` to identify affected Rust packages
and packages that depend on them directly or indirectly. Unknown inputs or
changes to build or verification files keep the profile at its full scope.
Quality gates follow their existing selection rules. Full SingleStep runs in
milestone verification, outside the commit profile.

Python tool ownership lives in `commands.py`. Without directory selectors,
`test python` selects affected tool suites. An explicit directory runs that suite
even when Git has no relevant changes. Telemetry/archive contracts remain a
separate suite; the retired workflow coordinator has no test suite.

Scoped documentation review and metadata/link, semantic, language and diary work
are selected separately. A successful profile does not assert that those operations
ran. A due English workflow uses `check english` for the full scan.
`check english files <paths>` remains available as a diagnostic.

## 1. M68000 SingleStepTests (Generated Reference Verification)

The tests compare the CPU core with **Tom Harte's `SingleStepTests-680x0`** suite
([`ref_src/SingleStepTests-680x0/68000/v1/`](../../ref_src/SingleStepTests-680x0/68000/v1)).
The suite contains 124 per-instruction test files and ~1,000,000 randomized vectors.
A software implementation generated the vectors. They provide no direct physical
68000 measurements. The [upstream methodology](https://github.com/SingleStepTests/680x0#methodology)
describes the checks applied to that software implementation.

### Execution Modes

#### A. Fast Smoke Test (Sampled)
For an explicit sample of up to 50 cases per ordinary instruction test:
```powershell
# Run sampled SingleStepTests across all implemented opcodes
python tools/verification/verify.py test runner singlestep sample 50

# Run tests for a specific instruction or family
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_nop
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_add_b
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_move_w
```

#### B. Full Exhaustive Verification (`SINGLESTEP_FULL`)
Setting `SINGLESTEP_FULL=1` removes the ordinary per-suite sample limit. For each
vector it executes, the test compares CPU state, instruction clocks and the time,
direction, address, size and data of each transaction callback. Internal delays
remain part of the expected timeline. Every mode uses the same assertions. The
named tests select all 124 JSON files. Filtered CMPM tests also exercise subsets
of the CMP files. The test loads initial RAM into the isolated fixture and checks
writes through committed transaction callbacks. The reference's `final.ram`
adds no separate CPU assertion. See the callback interface's timing and
electrical signal limits in the
[CPU verification specification](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md#12-input-scope-and-complete-verification).

- **PowerShell (Windows):**
  ```powershell
  python tools/verification/verify.py test runner singlestep full
  ```
- **Bash / Linux / macOS:**
  ```bash
  python tools/verification/verify.py test runner singlestep full
  ```

#### C. Custom Sample Limit (`SINGLESTEP_LIMIT`)
To evaluate an arbitrary sample size (e.g. 200 or 500 test cases per opcode):
```powershell
python tools/verification/verify.py test runner singlestep sample 500
```

#### D. Automated Archive Decompression (`tools/bootstrap/bootstrap.ps1 -Sources`)
Running `.\tools\bootstrap\bootstrap.ps1 -Sources` automatically finds compressed
archives (`*.json.gz`, `*.gz`, or `.zip`) under `ref_src/SingleStepTests-680x0/`.
It uses native .NET decompression to produce native `.json` files in `68000/v1/`.
Repeating the operation leaves the same result, and it has zero external dependencies.

---

## 2. Ordinary CPU Wait-State Verification

The CPU crate suites use PhysicalMemory and a small AddressBus wrapper. The
wrapper records transfers and can allow a specified transfer immediately, refuse
it once or refuse it repeatedly. The tests check that reads wait at CCK1, writes
wait at CCK2 and idle phases issue no accesses. During a refusal, latches or
memory stay unchanged. Retries must keep the same transfer identity, and the
transfer must succeed exactly once. Fixtures that execute real instructions cover
long halves, address updates, RMW, prefetch, stack and exception transfers, MOVEM
and MOVEP. The suites count refusals and elapsed clocks separately. An earlier
successful access can still be followed by a transfer that waits.

```powershell
python tools/verification/verify.py test rust crate cpu
```

These tests run in ordinary crate selection and commit coverage. They do not
load SingleStep vectors or enumerate memory placements/DMA schedules. Production
MemoryBus and Agnus arbitration retain separate integration tests. The former
Cartesian suite, exclusive helpers and DMA/harness selectors are removed.
See [CPU wait coverage](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md#8-ordinary-cpu-wait-state-tests)
for the observation boundary.

---

## 3. Automated Architecture Rules Compliance

[`checks/check_architecture.py`](checks/check_architecture.py) runs 18 source and
repository checks: file size limits $\le 800$ lines, forbidden unwrap/expect calls,
custom macros, path privacy, inlining attributes, instruction layout, test
organization, the 16 KiB root `AGENTS.md` limit, design-document links and approved
dependencies between workspace crates. It uses the Python standard library
and Cargo metadata, without compiling Rust. Commit and `check all` run
the complete set; `case` selects control names by literal substring:
```powershell
python tools/verification/verify.py check architecture
python tools/verification/verify.py check architecture case test_file_size_limits
python tools/verification/verify.py check architecture case test_workspace_crate_dependencies
```

`APPROVED_CRATE_DEPENDENCIES` in the architecture checker is the explicit table
of permitted direct workspace dependencies. Production (`normal`), test (`dev`)
and build dependencies are separate; an omitted kind allows none. Every workspace
crate needs an entry, including crates with no internal dependencies. The check
uses `cargo metadata --no-deps --offline --locked` to include inherited, renamed,
optional and target-specific declarations, even when their features or platforms
are inactive. Adding a permission requires a deliberate rule change. The checker
never derives permissions from the current manifests. Removing a dependency is
allowed.
External libraries and indirect paths through re-exports are outside this check.

The opcode micro-step coverage test uses the compiled CPU descriptor table and
lives in [`crates/cpu/tests/test_visibility.rs`](../../crates/cpu/tests/test_visibility.rs).
It runs with ordinary CPU tests. Formatting remains a separate `check formatting` gate.

---

## 4. Automatic Reports & Regression Tracking

The SingleStep test helpers save `summary.md`, `diff.md` and the suite results
in the [selected report directory](#saved-singlestep-reports). Open the Markdown
files to see saved suite totals and regression status. A filtered run can leave
older suite results in these reports. Check which suites were selected and when
their results were written before interpreting the totals.

```powershell
# Run sampled SingleStep tests for one opcode with live failure diagnostics
python tools/verification/verify.py test runner singlestep case test_add_b sample 50
```

- 🔴 **Regressions:** Tests that previously passed but now fail are highlighted with `⚠️ [REGRESSION DETECTED]`.
- 🟢 **Improvements:** Tests that previously failed but now pass are highlighted with `🎉 [PROGRESS / FIX]`.

---

## Native Disk Boot Checks

The two checks in
[`test_native_disk_boot`](../../crates/machine_loop/tests/test_native_disk_boot.rs)
require private inputs and are ignored by ordinary test runs. Run them explicitly
with `--ignored`; missing inputs fail the selected check. Supply an unmodified
256 KiB Kickstart 1.3 rev 34.5 ROM through `AMIGA_KICKSTART_ROM`. No ROM is included.

```powershell
$env:AMIGA_KICKSTART_ROM = '<private-rom-path>'
python tools/verification/run_rust_tests.py -- cargo test -p machine_loop --release --test test_native_disk_boot cross1_boots -- --ignored --nocapture

$env:AMIGA_WORKBENCH_ADF = '<private-workbench-adf-path>'
python tools/verification/run_rust_tests.py -- cargo test -p machine_loop --release --test test_native_disk_boot workbench13 -- --ignored --nocapture
```

The cross1 check reads
`ref_src/vAmigaTS/Agnus/Copper/cross/cross1/cross1.adf` automatically. The Workbench
check reads the ADF named by `AMIGA_WORKBENCH_ADF`. Both insert their image into DF0
with write protection and run unthrottled on a PAL A500 with 1 MB of RAM.

The cross1 check verifies that the original program runs and produces Copper
color changes. It checks repeatability and includes a control run without a
disk. The Workbench check observes startup stages over 60 emulated seconds and
saves a frame at `.agent/tmp/native-disk/workbench.ppm`. Assess the desktop image
by opening that frame. The Workbench verdict covers the startup check alone.
Neither check compares an image with a hardware reference.

## Hardware Capture References and Availability

The vAmigaTS reference assets remain under `ref_src/vAmigaTS/`. Tests can include
an ADF image or binary payload, a `test.retrosh` script describing configuration
and frame count, and a hardware reference capture. The former comparator used
716 x 285 RGB24 viewport captures (612,180 bytes).

The repository removed the execution runner, script handling, payload injection,
viewport extraction, comparison commands and the chipset throughput benchmark
that depended on them. Neither the commit nor milestone profile runs captures.
There is currently no manual capture CLI. To use the reference assets as evidence
of current whole-machine behavior, a future workflow needs an executable workload
and a validated way to compare its output. The retained assets and historical
results do not establish a fresh execution.

External source provisioning remains available:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Sources
```

Provisioning does not restore execution or certify a capture comparison.

## Local integration

Use ordinary Git commands for the explicitly selected commit and merge. The old
`integrate` command, delivery coordinator and freeze/seal/publication records have
been removed. The [Git procedure](../../.agents/skills/TA-WF-Main-close-task/references/local-integration.md)
contains shared scope constraints. Before removing a worktree, preserve local
outputs with the [artifact tool](../artifacts/README.md), then use the unchanged
`tools/worktree.ps1`. Native case reports already stored in the primary checkout
survive there.
