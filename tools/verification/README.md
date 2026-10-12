# Repository Verification Reference

This reference contains test commands, verification routing and diagnostic details. For an introduction to what each kind of test establishes, read the [testing overview](../../docs/testing.md). Run commands from the repository root. Current options are available with `python tools/verification/verify.py --help`; `--plan` previews selected commands.

## Command syntax

Use `python tools/verification/verify.py` followed by one sentence below. There
is no installed short `verify` alias. Values containing spaces must be quoted.
Selection keywords are unique; unknown or incomplete sentences fail before
execution. `case` is a substring filter, including Python qualified test names.

| Sentence | Scope |
| --- | --- |
| `commit` | Required commit profile; excludes full SingleStep |
| `milestone [singlestep]` | Full language audit and full SingleStep; the selector does not duplicate execution |
| `test rust [unit|integration|all|workspace]` | Diagnostic tiers; default is unit plus integration; workspace excludes test_runner |
| `test rust crate NAME [suite NAME] [case FILTER]` | One Cargo package, optionally one integration target and filter |
| `test python` | Tool contracts affected by changed owning inputs and shared dependencies |
| `test python directory PATH [file PATTERN] [case FILTER]` | One Python test directory and optional discovery filters |
| `test runner [standard|silicon]` | Standard harness by default (transaction checker regressions), or full SingleStep |
| `test runner singlestep [case FILTER] [full|sample N]` | Full vectors by default; explicit positive sampling limit overrides inherited environment |
| `check all` | All commit quality gates without crate coverage or ordinary tests |
| `check architecture [case FILTER]` | Python repository architecture checks; no Rust compilation |
| `check english [files PATH...]` | Full language audit or explicit files |
| `check code|hardware|formatting|clippy` | One quality gate |
| `check build [crate NAME]` | Cargo compilation check; workspace by default |
| `runner build` | Compile release test and benchmark targets without running them |
| `runner bench [quick\|standard\|thorough]` | CPU benchmark |
| `runner gui -- OPTIONS` | GUI inspector capture |
| `runner profile bench [quick\|standard\|thorough]` | Build the CPU benchmark and run it under samply; no profile prints benchmark help |
| `runner analyze CSV [report MARKDOWN]` | Analyze an existing benchmark CSV |

`--plan` previews the exact child commands and environment. `--log-level
full|warn|error` controls output; both options may appear among sentence selectors.
CPU benchmarks use the Cargo benchmark target described below.
The aggregate SingleStep test saves suite results and calls report aggregation
once after all selected groups finish, including failing executions.
Only `runner gui -- OPTIONS` forwards native switches to a separate program.
Counts must be positive; the parser rejects unknown or repeated selectors and
missing or invalid values before execution, with exit code 2.
The `test_runner` crate contains test and benchmark targets with shared helpers
in `tests/common/`; chipset baseline recording has no executable entry point.
Benchmark analysis and profiling default to full output; other commands default
to warnings and errors. Full logs are always stored in `.agent/tmp/verify/`.

Examples:

```powershell
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_sub
python tools/verification/verify.py test runner singlestep case test_move_w sample 500
python tools/verification/verify.py test python directory tools/bootstrap/tests file test_bootstrap_contract.py case test_all
python tools/verification/verify.py check architecture case test_file_size_limits
python tools/verification/verify.py runner bench quick --plan
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

## Saved SingleStep reports

The single `#[test]` entry point calls `common::singlestep_suite::run_suite()`.
It runs selected instruction groups on bounded worker threads, catches group
assertions so every group finishes, then calls `common::reporting::update_global_summary()`
once before the final assertion. This updates `.agent/tmp/singlestep/summary.json`,
`summary.md` and `diff.md` inside the same Rust process.
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
identities and sample limits. Legacy unidentified reports, including old
colliding CMP results, are retained through normal history rotation but excluded
from comparisons until a compatible baseline exists. New and removed case names
remain excluded from status-change comparisons.

Direct Cargo execution of `test_singlestep` also finalizes reports automatically.
The standard libtest runner sees one aggregate test. The existing verify `case`
selector matches group names inside Rust through `SINGLESTEP_CASE`; sample limits
still control vector counts only. For direct Cargo execution, select a group with
the same environment variable:

```powershell
$env:SINGLESTEP_CASE = "test_add_b"
cargo test -p test_runner --profile verification --test test_singlestep
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

CPU measurements run through the `bench_instructions` Cargo target declared in
[`Cargo.toml`](../../crates/test_runner/Cargo.toml). SingleStep reports are
finalized automatically inside the aggregate SingleStep test.

```powershell
cargo bench -p test_runner --bench bench_instructions -- --help
cargo bench -p test_runner --bench bench_instructions -- --quick
```

Running without options prints instructions without measuring. Select one profile
with `--quick`, `--standard` or `--thorough`; `--help` and `-h` print instructions.
Unknown, repeated or combined options fail before measurement. The benchmark uses
700 instructions per block, profile-defined measurement counts, the full catalog,
and host core affinity when supported. Reports are saved under
`.agent/tmp/benchmarks/<profile>/`.

`verify.py runner bench` accepts only a profile and translates it to the native
option. After successful measurement, it runs
[analyze_benchmarks.py](analyze_benchmarks.py) on that profile's `m68k_benchmark.csv`
and saves `analysis_report.md` in the same directory. A failed benchmark skips
analysis; running without a profile prints help only. Direct `cargo bench` runs
measurement only. `runner analyze CSV [report MARKDOWN]` remains available for
existing datasets. `runner profile bench` accepts the same profile and
builds the same target without running it, reads its executable path from Cargo's
artifact messages, and passes that executable to `samply record`. A failed build
or missing artifact cannot launch a previous executable.

## Completion verification

Verification orchestration and coverage result processing live in `tools/verification/`;
worktree management lives in `tools/worktree.ps1`. The shared diary appender lives in `tools/log_diary.py`. Concrete project audits and checks live in `tools/verification/checks/`. Run these scripts from the repository root. Task selection uses subcommands; the former selection switches are rejected.

The `test_runner` suites retain emulator execution checks and reference-result invariants. Standalone checks of its analyzers, statistics, PRNG, persistence, host setup, synthetic bus, program injection, report schemas, parsers and frame matcher are excluded; mixed suites retain their real execution cases. Empirical anomaly checks use stored results and do not establish fresh performance measurements.

Commit verification starts with ordinary workspace crate tests under LLVM coverage instrumentation, excluding `test_runner`. This is automatic, with no coverage switch. Only this execution contributes to the coverage report; the remaining quality gates and harness suites use normal Cargo artifacts. `cargo-llvm-cov` uses its separate `target/llvm-cov-target` cache, while normal Cargo commands retain their usual target directory. Raw measurement counters and the previous summary are cleared before each measurement, without deleting the build caches. Yellow `[INFO]` messages identify the instrumented and uninstrumented phases and summarize line, function and region coverage. Full summary data is saved to `.agent/tmp/coverage/summary.json`; there is no percentage threshold. Coverage does not measure performance or prove assertion quality. Stable coverage does not include doctest execution; branch coverage is not enabled.

Install the required tooling once:

```powershell
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

Run `python tools/verification/verify.py` without arguments (or with `--help`) to display the verification sets, milestone CPU-suite scope, plan behavior and command examples. This prints instructions and exits successfully without running verification.

Verification runs on the current working tree and does not require staging or pending changes. For milestone closure run both separate sets, once each:

```powershell
python tools/verification/verify.py commit
python tools/verification/verify.py milestone
```

Routine commits use only the first command. The profiles are invoked separately. Commit first runs ordinary workspace crate tests with coverage, reports the measured coverage, then runs quality gates, standard `test_runner` suites and Python tool contracts without coverage instrumentation. It never selects full SingleStepTests, even for CPU changes. Every commit scans all repository-owned text files against the English dictionary and reviewed project vocabulary, regardless of staging or changes, using `tools/verification/checks/check_english.json`. Milestone runs that language audit plus full SingleStepTests, including on a clean tree so already committed milestone work is covered. It does not repeat the other commit checks. vAmigaTS Cargo suites and chipset workload tests have been removed; capture runs and chipset throughput comparisons are excluded from automatic verification.

Python tool contracts live beside their owning tools. Commit verification runs
`unittest discover` separately for each directory below; the tools do not run
these tests during normal operation. Without a commit, run only checks relevant
to a coherent change, not after every edit. Repeat checks only after affected
input changes, failures or unresolved concerns. `python tools/verification/verify.py test python` selects contracts
whose owning tool inputs or shared dependencies have staged, unstaged or nonignored
new changes; unrelated changes and a clean tree select no Python suites. Shared
conversion changes select both conversion and PDF contracts. The full contract
set remains mandatory before every authorized commit. Run a focused suite from the repository root
with `python tools/verification/verify.py test python directory <directory>`,
optionally adding `file <test_file.py>` and `case <filter>` to narrow discovery.

| Directory | Coverage |
| --- | --- |
| [`tools/bootstrap/tests`](../../tools/bootstrap/tests/) | Bootstrap and Qdrant provisioning command contracts |
| [`tools/bootstrap/conversion/tests`](../../tools/bootstrap/conversion/tests/) | Shared client, transport, cache and PDF/HTML publication contracts |
| [`tools/bootstrap/pdf-to-markdown/tests`](../../tools/bootstrap/pdf-to-markdown/tests/) | PDF stage validation, artifacts, restart and resume contracts |
| [`tools/amiga-rag-mcp-server/tests`](../../tools/amiga-rag-mcp-server/tests/) | RAG CLI adapter contracts and real Codex MCP discovery |
| [`tools/verification/tests`](tests/) | Verification grammar, selection, preview and failure propagation |

The code-quality auditor accepts no arguments and always checks dead code,
visibility and accessor conventions across the workspace. Commit verification
runs it once; milestone verification does not repeat it. Every finding returns
exit code 1. Run `python tools/verification/verify.py check code` for the same full audit
manually. Passing command-line arguments is rejected.

The hardware-quality auditor accepts no arguments and always checks hardware
bus topology and DMA mastership patterns. Commit verification runs it once;
milestone verification does not repeat it. It does
not check CCK timing, open-bus values or test-file presence.

There is no shared documentation-governance script in the verifier. Instruction
brevity, script ownership and procedure routing are reviewed manually.
Scoped design review, vault links and
frontmatter, static semantic comparisons, diary chronology and roadmap retention
are run by their owning skills. Follow
[sync-design-docs](../../.agents/skills/sync-design-docs/SKILL.md),
[obsidian-vault-linking](../../.agents/skills/obsidian-vault-linking/SKILL.md),
[audit-semantic-parity](../../.agents/skills/audit-semantic-parity/SKILL.md),
[compact-diary](../../.agents/skills/compact-diary/SKILL.md) and
[roadmap-maintenance](../../.agents/skills/roadmap-maintenance/SKILL.md) at their defined
triggers. Semantic milestone review is required by its skill. A successful
`verify.py milestone` alone does not certify these separate skill checks.

Full SingleStep runs automatically at milestones. Independent full runs remain available for explicit diagnosis or user requests:

```powershell
python tools/verification/verify.py test runner singlestep
python tools/verification/verify.py test runner silicon
python tools/verification/verify.py milestone --plan
```

With no completion profile, `test runner singlestep` runs only full SingleStepTests. Standalone sets use the current working tree, require no staging, and run no quality gates, ordinary crate tests, or Python contracts. Milestone always selects full SingleStep once; its optional selector does not duplicate execution. Commit rejects CPU suite selectors. Standalone runs do not replace completion verification. Add `--plan` to inspect commands without executing checks. There are no hooks. Unknown arguments are rejected.

The verifier includes formatting, Clippy, architecture rules, and per-commit hardware/code audits. Ordinary-crate coverage measures executed code; there is no public-function name-matching gate. All profiles accept clean trees, unstaged changes and new files. Change selection includes both endpoints of renames. Completion evidence must match the final committed source; rerun verification if that source changes. All profiles reject results if the source or index changes during execution. Ignored diagnostics belong in `.agent/tmp/`. The verifier does not stage or commit files. It only selects external commands, launches them and aggregates exit codes; source/index stability is an orchestration guard. Command selection is declared in [commands.py](commands.py); [cli.py](cli.py) owns the grammar, [execution.py](execution.py) owns logging, and [repository.py](repository.py) owns source identity. During execution, `[RUN]` identifies the active check and `[CMD]` prints its exact command immediately before launching it; explicit environment overrides appear as `[ENV]`. Full child output is saved in a unique log under `.agent/tmp/verify/`. `--log-level full|warn|error` controls child output: `full` streams everything, `warn` (default) shows warnings and errors, and `error` shows only errors. Filtered modes include up to 12 indented diagnostic context lines, limited to 300 characters per line. Commands and final results remain visible in every mode. If a failed command emits no recognized error marker, its last 12 log lines are shown as failure context; failed commands also show the full log path before the final FAIL result. There is no initial test-command preview. `--plan` starts with an explicit preview-only message and does not launch commands.

Focused development diagnostics also use `verify.py`:

```powershell
python tools/verification/verify.py test rust unit
python tools/verification/verify.py test rust integration
python tools/verification/verify.py test runner standard
python tools/verification/verify.py test rust all --plan
```

`test rust unit` runs the configured ordinary crate tests;
`test rust integration` runs memory_bus, machine_loop, debugger and GUI tests;
`test rust all` combines those two tiers. `test runner standard` selects ordinary harness suites, including
transaction checker regressions and SingleStep override contracts;
`test runner silicon` selects full SingleStepTests.
Focused commands do not run coverage or completion gates. With no arguments,
the entry point displays help without executing commands.

SingleStep uses CPU + synthetic TestMemoryBus, not production MemoryBus or
PhysicalMemory. Its selection is owned by [commands.py](commands.py):

| Trigger | Full SingleStepTests |
| --- | --- |
| Ordinary edits or task completion | No automatic full run |
| Commit, including CPU and harness changes | No |
| Explicit suite selection | `test runner singlestep` |
| Milestone, including a clean working tree | Full suite |

Commit architecture tests run once as a fixed quality gate. Other standard commit tests run independently of changed paths, including ordinary CPU tests using real PhysicalMemory. The separate milestone set runs the full language audit and full SingleStepTests; vAmigaTS Cargo suites and automatic chipset/capture runs have been removed.

Manual commands in the following sections and diagnostic selectors in `verify.py` validate ordinary tasks and support diagnosis; they do not replace pre-commit verification. Wall-clock microbenchmarks and external sampling profiles run explicitly. CPU benchmark regression test targets, the old chipset throughput workload and capture execution commands have been removed. Retained CPU baselines are measurement comparison inputs. Before each authorized commit, `sync-design-docs` reviews only the intended changed paths and specifications selected through `tracked_paths`; unchanged documents and their dates stay untouched. Semantic milestone review, diary compaction, and roadmap pruning remain separate agent procedures.

---

## 1. M68000 SingleStepTests (Physical Hardware Silicon Verification)

The CPU core is validated against **Tom Harte's `SingleStepTests-680x0`** suite ([`ref_src/SingleStepTests-680x0/68000/v1/`](../../ref_src/SingleStepTests-680x0/68000/v1)), consisting of 124 per-instruction test files and ~1,000,000 randomized test vectors captured directly from physical 68000 silicon pins.

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
executed vector always compares CPU state, listed RAM, instruction clocks and
transaction callback times, direction, address, size and data. Internal delays
remain part of the expected timeline. There is no weaker assertion mode. All
124 JSON files are selected by the named tests; filtered CMPM tests additionally
exercise subsets of the CMP files. Existing pipeline allowances, RAM masks and
unobserved electrical signals remain documented in the
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
load silicon vectors or enumerate memory placements/DMA schedules. Production
MemoryBus and Agnus arbitration retain separate integration tests. The former
Cartesian suite, exclusive helpers and DMA/harness selectors are removed.
See [CPU wait coverage](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md#8-ordinary-cpu-wait-state-tests)
for the observation boundary.

---

## 3. Automated Architecture Rules Compliance

[`checks/check_architecture.py`](checks/check_architecture.py) runs 17 source and
repository checks: file size limits $\le 800$ lines, forbidden unwrap/expect calls,
custom macros, path privacy, inlining attributes, instruction layout, test
organization, instruction file budgets and design-document links. It uses the
Python standard library and requires no Rust build. Commit and `check all` run
the complete set; `case` selects control names by literal substring:
```powershell
python tools/verification/verify.py check architecture
python tools/verification/verify.py check architecture case test_file_size_limits
```

The opcode micro-step coverage test uses the compiled CPU descriptor table and
lives in [`crates/cpu/tests/test_visibility.rs`](../../crates/cpu/tests/test_visibility.rs).
It runs with ordinary CPU tests. Formatting remains a separate `check formatting` gate.

---

## 4. Automatic Reports & Regression Tracking

The SingleStep test helpers save `summary.md` and `diff.md` in
`.agent/tmp/singlestep/` automatically with suite results. Open these files for
saved suite totals and regression status. Filtered runs can leave older suite
results in these reports; compare scope and freshness before interpreting totals.

```powershell
# Run sampled SingleStep tests for one opcode with live failure diagnostics
python tools/verification/verify.py test runner singlestep case test_add_b sample 50
```

- 🔴 **Regressions:** Tests that previously passed but now fail are highlighted with `⚠️ [REGRESSION DETECTED]`.
- 🟢 **Improvements:** Tests that previously failed but now pass are highlighted with `🎉 [PROGRESS / FIX]`.

---

## 5. Agent Test Execution

The [`$test-runner`](../../.agents/skills/test-runner/SKILL.md) skill runs tests
across tiers, updates snapshots in `.agent/tmp/singlestep/`, and reports
regression differences.


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
For future investigations, follow
[structural-root-cause.md](../../.agents/rules/structural-root-cause.md): reproduce
the observed failure, establish the relevant clock or state transition, and
repair its demonstrated mechanism. Coordinate or timing adjustments made solely
to fit an image do not establish hardware behavior.
