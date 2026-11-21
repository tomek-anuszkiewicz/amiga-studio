# Repository Verification Reference

This reference contains test commands, verification routing and diagnostic details. For an introduction to what each kind of test establishes, read the [testing overview](../../docs/testing.md). Run commands from the repository root. Invoke `python tools/verification/verify.py` without arguments or with `--help` to see the full command tree; `--plan` previews selected commands.

## Command syntax

Run commands directly from the selected checkout. The
[case reporting instructions](#case-reports) describe optional registration binding
and standalone reports.

Use `python tools/verification/verify.py` followed by one sentence below. There
is no installed short `verify` alias. Values containing spaces must be quoted.
Selection keywords are unique; unknown or incomplete sentences fail before
execution. `case` is a substring filter, including Python qualified test names.
Use `--exact` with a focused Rust or SingleStep `case` to match the complete
test or group name. For ordinary Rust tests, pass the full libtest name when it
includes a module path.

| Sentence | Scope |
| --- | --- |
| `commit` | Selected check profile; excludes full SingleStep |
| `milestone [singlestep]` | Full SingleStep, with or without the optional selector |
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
| `check english [files PATH...]` | Full language audit or explicit files; task plans under `.agent/tasks/` are always excluded |
| `check code|hardware|formatting|clippy` | One quality gate |
| `check build [crate NAME]` | Cargo compilation check; workspace by default |
| `check web` | Build the browser application through Trunk and nightly; Cargo lint warnings fail |
| `runner build` | Compile release test and benchmark targets without running them |
| `runner bench [case ID\|mnemonic NAME\|opcode XXXX] [duration-ms N] [series N]` | CPU performance test measurement |
| `runner gui -- OPTIONS` | GUI inspector capture |
| `runner analyze RUN_DIRECTORY` | Regenerate a stored run analysis without measurement |

`--plan` previews the exact child commands and environment. `--log-level
full|warn|error` controls output; both options may appear among sentence selectors.
`--results-dir PATH` places verifier logs in `PATH/verify/` and, for SingleStep,
reports and history in `PATH/singlestep/`. Relative paths start at the repository
root. Use a new directory per experiment to keep its SingleStep history separate.
Other tools, including coverage and benchmarks, retain their own output paths.
`--exact` is accepted only with `test rust crate NAME suite NAME case NAME` or
`test runner singlestep case NAME`; it does not change the selected build profile
or vector comparisons. The focused Rust form fails when the named suite executes
zero or multiple tests. SingleStep `case` names one instruction group; `sample`
controls how many vectors from that group execute.
CPU benchmarks use the Cargo benchmark target described below.
The aggregate SingleStep test saves suite results and calls report aggregation
once after all selected groups finish, including failing executions.
Only `runner gui -- OPTIONS` forwards native switches to a separate program.
Counts must be positive; the parser rejects unknown or repeated selectors and
missing or invalid values before execution, with exit code 2.
The `test_runner` crate contains test and benchmark targets with shared helpers
in `tests/common/`; chipset baseline recording has no executable entry point.
Benchmark analysis defaults to full output; other commands default
to warnings and errors. Focused commands save full logs in `.agent/tmp/verify/`
by default. Profiles use the same logging path; `--results-dir` selects another directory.

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

For an assigned focused check, run the exact documented command directly; no
separate execution skill is needed. Confirm tools/fixtures, isolate concurrent
writable build/report outputs, await exit status and inspect actual reports.
Keep the selected candidate stable. Report failed, interrupted, missing or unrun
work separately; a check does not authorize repairs, broader tests or coverage.
Use the [CPU diagnostic guidance](../../.agents/references/cpu-diagnostics.md)
when investigating an isolated instruction mismatch. Full standalone SingleStep
requires its explicitly selected complete scope; do not expand a diagnostic
sample into the full suite.

## Browser build verification

Run `python tools/verification/verify.py check web` during changes to GUI code,
workspace manifests or the browser build pipeline. Every `commit` profile selects
the same `WebAssembly build` gate after preflight and before coverage. It invokes
`tools/web.ps1 build`, including Trunk asset processing and the production shared
memory flags. The launcher sets `CARGO_BUILD_WARNINGS=deny`, so Cargo lint warnings
such as unused workspace dependencies fail both ordinary web builds and the gate.
This does not change the centrally reviewed Clippy permissions. Non-lint Cargo
warnings are outside this Cargo setting's scope.

The build requires Trunk, PowerShell, and nightly Rust with `rust-src` and the
`wasm32-unknown-unknown` target, as described in the [launch instructions](../../README.md).
Missing tools or a failed build fail the selected verification. Every invocation
executes its selected build/check commands in the current environment.

## Case reports

Run tests and benchmarks directly from the intended checkout. Repository launchers
write standalone reports by default. If an execution registration was explicitly
selected, pass its `AMIGA_TEST_TASK_ID` and `AMIGA_TEST_EXECUTION_ID` environment
variables as described in the [capture guide](../workflow/capture/README.md#case-reports).
The retired selected-command launcher is not required.

Test and benchmark launchers print the per-invocation Markdown report at
startup. It links the native event journal, command output and child reports.
Each invocation retains its own case attempts and durations. Command wall times
and coverage retain their existing reports.

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

These direct commands select a timed Rust case and Python discovery by file:

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

SingleStep runs default to `.agent/tmp/singlestep/`. With `--results-dir PATH`,
the verifier uses `PATH/singlestep/`. Inspect the output from the actual run;
there is no saved milestone verdict that substitutes for execution.

The single `#[test]` entry point calls `common::singlestep_suite::run_suite()`.
It runs selected instruction groups on bounded worker threads, catches group
assertions so every group finishes, then calls `common::reporting::update_global_summary()`
once before the final assertion. This updates `summary.json`, `summary.md` and
`diff.md` in the selected results directory inside the same Rust process.
Each suite writer only persists its own result and rotates its history.
The summary contains totals,
a table for all saved suites, and all failing case names. The diff lists cases
that changed from passed to failed or failed to passed, and suites without a
previous result. Suites and cases are ordered for readable comparisons.

Open the generated Markdown files directly after a SingleStep run. Each suite
retains its own latest and previous result, so saved totals may combine different
executions. Current execution status and current report/vector counts are listed
separately. Missing Rust completion/report metadata, an unfinished selection or zero
matching groups is incomplete and returns failure. Test failure status is preserved even if
finalization also fails. A successful test process cannot hide reporting failure.

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
The pinned assembler is installed with `tools/bootstrap/bootstrap.ps1 -Assembler`.
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
all attempts, host-load observations, failures and completion status. The runner
executes complete guest programs and resets CPU and memory outside each timer.
The default is three series of at least 400 ms of timed execution per case.
The minimum series nanoseconds per target is the selected whole-workload cost;
all series remain in the case report. No separate guest inspection run is made.
Assembly sources and binaries stay under `.agent/tmp/benchmarks/cpu_performance_tests/`.

The external monitor samples the pinned host core during each measured series
(all logical cores when affinity is unavailable). It subtracts the benchmark
process's CPU time from busy time and treats more than 20% average external
busy time in any series, or unavailable observation, as a
load disturbance. One disturbed attempt triggers one full-case rerun with the
same settings. Both attempts remain in the manifest, and a second disturbance
marks the result. Execution failure is recorded as a failed case without retry.

Completed runs automatically write `aggregate.json`, `aggregate.csv`,
`anomalies.json`, `comparison.json`, `baseline_comparison.json`,
`cross_case_anomalies.json`, `a7_comparison.json` and a readable `report.md`. Explicit analysis
regenerates those reports from stored data without measuring again. An incomplete
run cannot be analyzed or selected as a comparison. The existing previous-run
comparison uses the latest earlier locally available, completed full run with
matching host and settings. Each case additionally requires matching source,
configuration, image, target count and CPU clocks.

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
The Markdown report lists timing deviations of at least 5% that also exceed
their measured series spread, baseline regressions and incompatible cases,
failed or load-disturbed measurements, and series spreads of at least 10%.
Series-spread rows are collapsed for quick review. These diagnostic limits are
not pass/fail gates or estimates of statistical confidence. Whole-program times
include helpers and do not establish isolated opcode latency or CPU correctness.

Only a successful, complete, default-parameter catalog run automatically replaces the
trackable `benchmarks/m68k_benchmark_current.json` and `.csv` files. It also
writes `m68k_benchmark_anomalies.json`, `m68k_benchmark_baseline_comparison.json`,
`m68k_benchmark_cross_case_anomalies.json`, `m68k_benchmark_a7_comparison.json`
and `m68k_benchmark_anomaly_report.md`
beside them in `benchmarks/`. These are the reviewable reports to commit with
the current snapshot. The run directory retains its own detailed copies.
Filtered runs and shortened development runs receive reports only in their run
directories. `runner refresh RUN_DIRECTORY` explicitly replaces matching cases in
the current snapshot from one completed, undisturbed filtered run made with the
current host, settings, workload fingerprint, target count and CPU clocks. The
current CSV and JSON remain paired by SHA-256; `case_sources` in the JSON records
each replaced case's run ID. The root anomaly reports are regenerated from the
combined current snapshot. A `cases FILE` selector reads one case ID per line,
so related instruction and addressing-mode workloads can be measured together
before refresh. Reanalyzing an older full run cannot replace newer root files.
The approved baseline remains tracked and is never updated by the runner.
After review and explicit approval under the reference-artifact procedure, the
complete current JSON/CSV pair may become the next baseline, including a
composite snapshot with documented per-case sources. Copy both files together,
regenerate the baseline comparison and root anomaly reports, then review the
resulting Git diff. The normal commit verification and commit policy still apply.

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

These are check bundles, not commit or closure operations. The commit profile
retains quality checks, the browser build, ordinary tests/coverage and selected
Python tool suites. The milestone profile retains full SingleStep verification.
Use `--plan` to inspect commands without running them and `--results-dir <path>`
for a selected log/SingleStep output location.

Every invocation executes its selected checks. There are no frozen trees, saved
verdicts, reused passing groups, entry-point batch locks, sealed commits or Git
publication commands. The caller chooses the checkout and keeps shared build and
coverage outputs isolated. Results report actual commands, failures and unrun work.
The tool does not infer review, commit, merge, language-audit or cleanup authority.

### Generated timing report

Each ordinary verifier invocation writes a separate `test-runtime-report.md`
under `.agent/tmp/verification/diagnostics/`. The console prints its path and
content digest for archive lookup. `--results-dir` controls command logs and
SingleStep outputs; it does not enable verdict reuse.

The report records actual command wall times, failures and unrun work. Cargo
commands include measured build and execution time together unless the native
runner supplies a narrower breakdown. Coverage preparation, export and summary
have separate rows. Timing data does not certify a passing gate. Failed report
writing is reported as an incomplete invocation.

Native case reports and logs normally already survive in the primary checkout.
Preserve local timing reports, coverage and other ignored outputs with the
[artifact tool](../artifacts/README.md) before removing a task worktree.

### Bounded parallel execution

The existing Python test runner executes eligible cases with bounded workers and
retains justified serial cases. Rust test runners retain their native scheduling.
The verifier runs its selected command groups in order. A failed preliminary/build
check defers subsequent expensive phases; a failed coverage step stops its bundle.
Independent coverage exports must not share mutable counters. No batch-lock or
saved-evidence protocol coordinates separate verifier invocations.

### Coverage-guided testing

For an identified diagnostic need or an explicit coverage request, measure without completion gates:

```powershell
python tools/verification/verify.py test coverage crate denise
python tools/verification/verify.py test coverage workspace
```

Both commands use the commit coverage pipeline and run all ordinary tests of the
selected packages. The package command measures only the named package, without
automatically selecting its consumers. Use the workspace command for an explicitly
requested broad audit. There is no suite or case filter that could make a partial
test run look like complete package coverage.

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

Read `gaps.md`, inspect the corresponding production behavior and existing
assertions, then add or extend tests for in-scope gaps under the
[coverage criteria](../../.agents/references/testing-criteria.md#coverage-and-adequacy-assessment).
Check whether an omitted suite already tests the behavior before declaring a
missing test. Rerun coverage after test or production changes; use evidence only
from a successful run whose source and index remained unchanged. Before an
authorized commit, review the final commit report with its assertions. Report
unrelated existing gaps separately; do not suppress them or create a permanent
coverage exception list. Execution coverage does not establish assertion quality,
complete hardware scenarios or performance.

### Selection from changed inputs

The verifier uses staged, unstaged and nonignored new paths for check selection.
This selects work; it does not create a snapshot or prove source stability.
Commit selects affected Rust packages and their transitive consumers using
`impact.py`; unknown inputs or build/verification changes retain the conservative
full scope. Quality gates keep their existing selection. Full SingleStep remains
outside the commit profile and inside milestone verification.

Python tool ownership lives in `commands.py`. Without directory selectors,
`test python` selects affected tool suites. An explicit directory runs that suite
even when Git has no relevant changes. Telemetry/archive contracts remain a
separate suite; the retired workflow coordinator has no test suite.

Scoped documentation review and metadata/link, semantic, language and diary work
are selected separately. A successful profile does not assert that those operations
ran. Use `check english` for a requested full language scan or `check english files
<paths>` for selected files. Neither creates a sealed publication receipt.

## 1. M68000 SingleStepTests (Generated Reference Verification)

The CPU core is compared against **Tom Harte's `SingleStepTests-680x0`** suite ([`ref_src/SingleStepTests-680x0/68000/v1/`](../../ref_src/SingleStepTests-680x0/68000/v1)), consisting of 124 per-instruction test files and ~1,000,000 randomized vectors generated using a software implementation. The [upstream methodology](https://github.com/SingleStepTests/680x0#methodology) describes the checks applied to that implementation; these vectors are not direct physical 68000 measurements.

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
Setting `SINGLESTEP_FULL=1` removes the ordinary per-suite sample limit. Every
executed vector always compares CPU state, instruction clocks and
transaction callback times, direction, address, size and data. Internal delays
remain part of the expected timeline. There is no weaker assertion mode. All
124 JSON files are selected by the named tests; filtered CMPM tests additionally
exercise subsets of the CMP files. Initial RAM initializes the isolated fixture;
writes are checked through committed transaction callbacks. The reference's
`final.ram` is not a separate CPU assertion. The callback interface's timing and
electrical signal limits remain documented in the
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
Running `.\tools\bootstrap\bootstrap.ps1 -Sources` automatically scans for compressed archives (`*.json.gz`, `*.gz`, or `.zip`) under `ref_src/SingleStepTests-680x0/` and decompresses them into native `.json` files in `68000/v1/` using native .NET decompression (idempotent, zero external dependencies).

---

## 2. Ordinary CPU Wait-State Verification

The existing CPU crate suites use PhysicalMemory and a small recording
AddressBus wrapper to deny specific transfers zero, once or repeatedly. They
check read CCK1/write CCK2 waiting, idle phases without accesses, unchanged
latches or memory during refusal, stable retry identity and one successful
transfer. Real instruction fixtures cover long halves, address updates, RMW,
prefetch, stack/exception transfers, MOVEM and MOVEP. Actual bus refusals are
counted independently of elapsed clocks. Later transfers can wait again after
an earlier successful access.

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
organization, instruction file budgets, design-document links and approved
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
are inactive. New permissions require a deliberate rule change; the checker never
generates permissions from the current manifests. Removing a dependency is allowed.
External libraries and indirect paths through re-exports are outside this check.

The opcode micro-step coverage test uses the compiled CPU descriptor table and
lives in [`crates/cpu/tests/test_visibility.rs`](../../crates/cpu/tests/test_visibility.rs).
It runs with ordinary CPU tests. Formatting remains a separate `check formatting` gate.

---

## 4. Automatic Reports & Regression Tracking

The SingleStep test helpers save `summary.md` and `diff.md` in the
[selected report directory](#saved-singlestep-reports) with suite results. Open these files for
saved suite totals and regression status. Filtered runs can leave older suite
results in these reports; compare scope and freshness before interpreting totals.

```powershell
# Run sampled SingleStep tests for one opcode with live failure diagnostics
python tools/verification/verify.py test runner singlestep case test_add_b sample 50
```

- 🔴 **Regressions:** Tests that previously passed but now fail are highlighted with `⚠️ [REGRESSION DETECTED]`.
- 🟢 **Improvements:** Tests that previously failed but now pass are highlighted with `🎉 [PROGRESS / FIX]`.

---

## Hardware Capture References and Availability

The vAmigaTS reference assets remain under `ref_src/vAmigaTS/`. Tests can include
an ADF image or binary payload, a `test.retrosh` script describing configuration
and frame count, and a hardware reference capture. The former comparator used
716 x 285 RGB24 viewport captures (612,180 bytes).

The execution runner, script handling, payload injection, viewport extraction,
comparison commands and dependent chipset throughput benchmark have been
removed. Neither commit nor milestone runs captures, and there is currently no
manual capture CLI. A future capture workflow needs an executable workload and
validated comparison path before the reference assets can establish current
whole-machine behavior. Retained assets and historical results are not fresh
execution evidence.

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
