# Repository Verification Reference

This reference contains test commands, verification routing and diagnostic details. For an introduction to what each kind of test establishes, read the [testing overview](../../docs/testing.md). Run commands from the repository root. Current options are available with `python tools/verification/verify.py --help`; `--plan` previews selected commands.

## Command syntax

Use `python tools/verification/verify.py` followed by one sentence below. There
is no installed short `verify` alias. Values containing spaces must be quoted.
Selection keywords are unique; unknown or incomplete sentences fail before
execution. `case` is a substring filter, including Python qualified test names.

| Sentence | Scope |
| --- | --- |
| `commit [singlestep] [dma]` | Required commit profile with optional forced full CPU sets |
| `milestone [singlestep] [dma]` | Separate language profile with optional full CPU sets |
| `test rust [unit|integration|all|workspace]` | Diagnostic tiers; default is unit plus integration; workspace excludes test_runner |
| `test rust crate NAME [suite NAME] [case FILTER]` | One Cargo package, optionally one integration target and filter |
| `test python` | All five tool contract directories |
| `test python directory PATH [file PATTERN] [case FILTER]` | One Python test directory and optional discovery filters |
| `test runner [standard|harness|silicon]` | Standard harness by default, diagnostic harness or both full CPU sets |
| `test runner singlestep [case FILTER] [full|sample N]` | Full vectors by default; explicit positive sampling limit overrides inherited environment |
| `test runner dma [case FILTER]` | Cartesian DMA, optionally filtered |
| `check all` | All commit quality gates without crate coverage or ordinary tests |
| `check architecture [case FILTER]` | Python repository architecture checks; no Rust compilation |
| `check english [files PATH...]` | Full language audit or explicit files |
| `check code|hardware|formatting|clippy` | One quality gate |
| `check build [crate NAME]` | Cargo compilation check; workspace by default |
| `runner build` | Build the release test_runner binary |
| `runner summary|diff` | Read persisted result reports |
| `runner suite OPCODE` | Existing runner opcode diagnostics |
| `runner bench [quick\|standard\|thorough] [filter TAG] [unroll N] [passes N] [output PATH] [pin on\|off]` | CPU benchmark |
| `runner chipset [compare] [frames N]` | Chipset baseline comparison; no reference recording |
| `runner vamiga [category NAME] [test NAME] [frames N] [limit N] [verbose] [summary]` | Explicit whole-machine capture run |
| `runner vamiga list deferred` | Report deferred capture suites |
| `runner gui -- OPTIONS` | GUI inspector capture |
| `runner profile vamiga\|chipset\|bench SELECTORS` | Release build followed by external samply recording |
| `runner analyze CSV [report MARKDOWN]` | Analyze an existing benchmark CSV |

`--plan` previews the exact child commands and environment. `--log-level
full|warn|error` controls output; both options may appear among sentence selectors.
The `test_runner` binary accepts the same command words after Cargo's `--`:
`cargo run -p test_runner -- vamiga category Copper frames 16`.
Only `runner gui -- OPTIONS` forwards native switches to a separate program.
Counts must be positive; the binary rejects unknown or repeated selectors and
missing or invalid values before execution, with exit code 2. Benchmark profiles
set their defaults before explicit overrides, regardless of selector order.
Use `cargo run -p test_runner -- help` for binary instructions. The former
command switches and `benchmark-chipset` name have been replaced; standalone
baseline recording uses `chipset record frames N` and remains blocked in
verification. Reports and profiling default to full output; other commands default
to warnings and errors. Full logs are always stored in `.agent/tmp/verify/`.

Examples:

```powershell
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_sub
python tools/verification/verify.py test runner singlestep case test_move_w sample 500
python tools/verification/verify.py test python directory tools/bootstrap/tests file test_bootstrap_contract.py case test_all
python tools/verification/verify.py check architecture case test_file_size_limits
python tools/verification/verify.py runner vamiga category all --plan
python tools/verification/verify.py runner summary
```

The generic Rust path preserves the selected Cargo test's normal environment.
The dedicated SingleStep sentence explicitly chooses full or sampled execution.
Skill-owned scripts remain owned by their skills. Bootstrap, conversion,
indexing, worktree management, diary appending, tool installation and baseline
recording remain workflow operations with their own entry points.

## Completion verification

Verification orchestration and coverage result processing live in `tools/verification/`;
worktree management lives in `tools/worktree.ps1`. The shared diary appender lives in `tools/log_diary.py`. Concrete project audits and checks live in `tools/verification/checks/`. Run these scripts from the repository root. Their
Completion scopes remain unchanged. Task selection now uses subcommands; the former selection switches are rejected.

The `test_runner` suites retain emulator execution checks and reference-result invariants. Standalone checks of its analyzers, statistics, PRNG, persistence, host setup, synthetic bus, program injection, report schemas, parsers and frame matcher are excluded; mixed suites retain their real execution cases. Benchmark CSV and empirical anomaly checks use stored results and do not establish fresh performance measurements.

Commit verification starts with ordinary workspace crate tests under LLVM coverage instrumentation, excluding `test_runner`. This is automatic, with no coverage switch. Only this execution contributes to the coverage report; the remaining quality gates and harness suites use normal Cargo artifacts. `cargo-llvm-cov` uses its separate `target/llvm-cov-target` cache, while normal Cargo commands retain their usual target directory. Raw measurement counters and the previous summary are cleared before each measurement, without deleting the build caches. Yellow `[INFO]` messages identify the instrumented and uninstrumented phases and summarize line, function and region coverage. Full summary data is saved to `.agent/tmp/coverage/summary.json`; there is no percentage threshold. Coverage does not measure performance or prove assertion quality. Stable coverage does not include doctest execution; branch coverage is not enabled.

Install the required tooling once:

```powershell
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

Run `python tools/verification/verify.py` without arguments (or with `--help`) to display the verification sets, automatic CPU-suite selection rules, plan behavior and command examples. This prints instructions and exits successfully without running verification.

Verification runs on the current working tree and does not require staging or pending changes. For milestone closure run both separate sets, once each:

```powershell
python tools/verification/verify.py commit
python tools/verification/verify.py milestone
```

Routine commits use only the first command. The profiles are invoked separately. Commit first runs ordinary workspace crate tests with coverage, reports the measured coverage, then runs quality gates, standard `test_runner` suites and Python tool contracts without coverage instrumentation. Only Cartesian DMA and full SingleStepTests are conditional on staged, unstaged and nonignored new paths. Every commit scans all repository-owned text files against the English dictionary and reviewed project vocabulary, regardless of staging or changes, using `tools/verification/checks/check_english.json`. Milestone runs the same full language audit. vAmigaTS Cargo suites and chipset workload tests have been removed; capture runs and chipset throughput comparisons are excluded from automatic verification. It does not run commit checks or automatically select DMA/SingleStep.

Python tool contracts live beside their owning tools. Commit verification runs
`unittest discover` separately for each directory below; the tools do not run
these tests during normal operation. Run a focused suite from the repository root
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
milestone verification and the hardware-review skill do not repeat it. It does
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

DMA and SingleStep are independent test sets. They can run alone, together, or alongside a completion profile:

```powershell
python tools/verification/verify.py test runner dma
python tools/verification/verify.py test runner singlestep
python tools/verification/verify.py test runner silicon
python tools/verification/verify.py commit dma singlestep --plan
```

With no completion profile, `test runner dma` runs only Cartesian DMA and `test runner singlestep` runs only full SingleStepTests. Standalone sets use the current working tree, require no staging, and run no quality gates, ordinary crate tests, or Python contracts. Append `dma` or `singlestep` to `commit` or `milestone` to add those sets to the profile. Each selected suite runs once, even when changes also select it. Standalone runs do not replace completion verification. Add `--plan` to inspect commands without executing checks. There are no hooks. Unknown arguments are rejected.

The verifier includes formatting, Clippy, architecture rules, and per-commit hardware/code audits. Ordinary-crate coverage measures executed code; there is no public-function name-matching gate. All profiles accept clean trees, unstaged changes and new files. Change selection includes both endpoints of renames. Completion evidence must match the final committed source; rerun verification if that source changes. All profiles reject results if the source or index changes during execution. Ignored diagnostics belong in `.agent/tmp/`. The verifier does not stage or commit files. It only selects external commands, launches them and aggregates exit codes; source/index stability is an orchestration guard. Command selection is declared in [commands.py](commands.py); [cli.py](cli.py) owns the grammar, [execution.py](execution.py) owns logging, and [repository.py](repository.py) owns source identity. During execution, `[RUN]` identifies the active check and `[CMD]` prints its exact command immediately before launching it; explicit environment overrides appear as `[ENV]`. Full child output is saved in a unique log under `.agent/tmp/verify/`. `--log-level full|warn|error` controls child output: `full` streams everything, `warn` (default) shows warnings and errors, and `error` shows only errors. Filtered modes include up to 12 indented diagnostic context lines, limited to 300 characters per line. Commands and final results remain visible in every mode. If a failed command emits no recognized error marker, its last 12 log lines are shown as failure context; failed commands also show the full log path before the final FAIL result. There is no initial test-command preview. `--plan` starts with an explicit preview-only message and does not launch commands.

Focused development diagnostics also use `verify.py`:

```powershell
python tools/verification/verify.py test rust unit
python tools/verification/verify.py test rust integration
python tools/verification/verify.py test runner harness
python tools/verification/verify.py test rust all --plan
```

`test rust unit` runs isolated crates and golden-row fixtures;
`test rust integration` runs memory_bus, machine_loop, debugger and GUI tests;
`test rust all` combines those two tiers. `test runner harness` runs
benchmark smoke and Cartesian DMA. `test runner standard` selects ordinary harness
suites; `test runner silicon` selects full SingleStepTests and Cartesian DMA.
Focused commands do not run coverage or completion gates. With no arguments,
the entry point displays help without executing commands.

The two expensive environments use CPU + synthetic TestMemoryBus, not production MemoryBus or PhysicalMemory. Their trigger paths are owned by [commands.py](commands.py):

| Trigger | Cartesian DMA | Full SingleStepTests |
| --- | --- | --- |
| CPU/config crate, shared AddressBus/BusResult contract, common synthetic harness, vectors, workspace manifest/lock/config | Yes | Yes |
| DMA harness or its test target | Yes | No |
| SingleStep runner/reporter or its test target | No | Yes |
| Production memory implementation, MemoryBus, machine loop, chips, GUI, tools, docs | No | No |
| Explicit suite selection | `test runner dma` | `test runner singlestep` |
| Milestone | Only with explicit selector | Only with explicit selector |

Commit architecture tests run once as a fixed quality gate. Other standard commit tests run independently of changed paths, including ordinary CPU tests using real PhysicalMemory. The separate milestone set runs the full language audit; vAmigaTS Cargo suites and automatic chipset/capture runs have been removed.

Manual commands in the following sections and diagnostic selectors in `verify.py` remain development/diagnostic tools; they do not replace completion verification. Wall-clock microbenchmarks and external sampling profiles remain manual measurement tools: the verifier retains CPU benchmark regression fixtures. The chipset throughput comparison is available only as an explicit manual command. Before each authorized commit, `sync-design-docs` reviews only the intended changed paths and specifications selected through `tracked_paths`; unchanged documents and their dates stay untouched. Semantic milestone review, diary compaction, and roadmap pruning remain separate agent procedures.

---

## 1. M68000 SingleStepTests (Physical Hardware Silicon Verification)

The CPU core is validated against **Tom Harte's `SingleStepTests-680x0`** suite ([`ref_src/SingleStepTests-680x0/68000/v1/`](../../ref_src/SingleStepTests-680x0/68000/v1)), consisting of 124 per-instruction test files and ~1,000,000 randomized test vectors captured directly from physical 68000 silicon pins.

### Execution Modes

#### A. Fast Smoke Test (Sampled)
By default, each instruction suite evaluates a sampled subset of 50 test cases (~5 seconds total):
```powershell
# Run sampled SingleStepTests across all implemented opcodes
python tools/verification/verify.py test rust crate test_runner suite test_singlestep

# Run tests for a specific instruction or family
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_nop
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_add_b
python tools/verification/verify.py test rust crate test_runner suite test_singlestep case test_move_w
```

#### B. Full Exhaustive Verification (`SINGLESTEP_FULL`)
Setting `SINGLESTEP_FULL=1` disables sampling limits and executes **100% of all ~1,000,000 test vectors** across all suites in parallel (typically completes in ~5–6 seconds):

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

## 2. Cartesian DMA Contention Verification

Validates cycle-exact M68000 micro-stepping and wait-state handling under Agnus DMA bus contention across the full combinatorial Cartesian product:

- **Address Permutations ($2^k$):** Sweeps all role assignments of memory cells touched by the instruction (`ChipRam` vs `FastRam`).
- **DMA Schedule Permutations ($2^M$):** Sweeps every bit pattern of stalled vs free CCK slots across the execution window.
- **Asserted State Constraints:**
  1. *Cycle Invariance:* $C = C_0 + 2 \times \text{wait\_states}$
  2. *Fast RAM Immunity:* $C = C_0$ with zero wait states when memory addresses point to Fast RAM.
  3. *State Invariance:* Register values and RAM contents are 100% bit-identical to the uncontended golden run.

```powershell
# Run full Cartesian DMA contention test suite
python tools/verification/verify.py test rust crate test_runner suite test_dma_cartesian

# Run specific sub-suite
python tools/verification/verify.py test rust crate test_runner suite test_dma_cartesian case test_dma_cartesian_system_and_traps
```

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

## 4. CLI Diagnostic Tools & Regression Tracking

The `test_runner` crate provides a standalone CLI tool for inspecting coverage matrices, live failure diagnostics, and detecting regressions:

```powershell
# Display global pass/fail matrix and coverage summary across all opcodes
python tools/verification/verify.py runner summary

# Detect regressions and fixed tests compared to previous run
python tools/verification/verify.py runner diff

# Execute a single opcode suite directly with live diagnostic failure output
python tools/verification/verify.py runner suite ADD.b
```

- 🔴 **Regressions:** Tests that previously passed but now fail are highlighted with `⚠️ [REGRESSION DETECTED]`.
- 🟢 **Improvements:** Tests that previously failed but now pass are highlighted with `🎉 [PROGRESS / FIX]`.

---

## 5. Autonomous Testing Workflows & Slash Commands

For interactive pair-programming sessions with the AI Agent in the IDE chat, three repository skills drive the testing lifecycle:

| Skill | Role in Verification Lifecycle | When to Trigger |
| :--- | :--- | :--- |
| [`$test-runner`](../../.agents/skills/test-runner/SKILL.md) | **Execution & Telemetry Engine** | Run tests across tiers, update snapshots in `.agent/tmp/singlestep/`, and report red/green regression diffs. |
| [`$integration-test-sprint`](../../.agents/skills/integration-test-sprint/SKILL.md) | **Cascading Verification Protocol** | Drive multi-chip test sweeps (integration and hardware-capture scenarios), filter non-runnable tests, and cluster failures with a 2–3 attempt cap. |
| [`$synthesize-test-fixes`](../../.agents/skills/synthesize-test-fixes/SKILL.md) | **Root-Cause Consolidation** | Audit recent commits or git diff, construct the 3-column diagnostic matrix (`[Symptom] \| [Location] \| [Mechanism]`), find the shared hardware law (common denominator), and strip local ad-hoc patches. |


## Hardware Capture Diagnostics


The emulator features a dedicated verification harness and execution runner for the **vAmiga Test Suite (vAmigaTS)** (`ref_src/vAmigaTS/`, 2,077 test directories), providing cycle-exact chipset verification against golden 716×285 24-bit RGB `.raw` viewport captures recorded from real Amiga 500 hardware.

### 1. How vAmigaTS Verification Works
Each test under `ref_src/vAmigaTS/<Category>/<Subcategory>/<TestName>/` encapsulates a complete hardware test scenario:
- **Test Binary / ADF:** A bootable floppy disk image (`.adf`) or raw M68000 binary payload injected into Chip RAM at entry point `$1000`.
- **Test Script (`test.retrosh`):** Declarative shell script specifying memory configuration, initial register values, frames to simulate, and optional viewport cutouts.
- **Golden Reference Viewport (`screenshot.raw`):** Exact 24-bit RGB uncompressed pixel capture (716 × 285 pixels = 612,180 bytes) produced by clean silicon.
- **Verification Pipeline:**
  1. Sets up minimal OS stub interrupt and library vectors (`ExecBase` at `$0004`, `GfxBase`).
  2. Steps the machine loop for the scripted number of PAL frames (default: 8–16 frames).
  3. Extracts Denise's 24-bit RGB frame buffer and compares it pixel-by-pixel against `screenshot.raw`.
  4. Reports matching pixel count, mismatch coordinates, and actual vs expected RGB values.

### 2. Provisioning Prerequisite
Ensure reference assets are extracted prior to running vAmigaTS suites:
```powershell
.\tools\bootstrap\bootstrap.ps1 -Sources
```

### 3. Automatic Verification Scope

vAmigaTS Cargo test suites and chipset workload tests have been removed during test reorganization. Neither commit nor milestone verification runs vAmigaTS captures or chipset throughput comparisons. Reference assets and the manual CLI below remain available for explicitly requested investigations.

### 4. Interactive CLI Runner (`test_runner vamiga`)
Execute tests with live pixel diff reporting, category filtering, and frame control:
```powershell
# Execute a specific test by name with live pixel mismatch reporting:
cargo run -p test_runner -- vamiga test coptim1

# Execute a specific test with verbose pixel difference coordinates:
cargo run -p test_runner -- vamiga test coptim1 verbose

# Run all runnable tests in a specific custom chip category:
cargo run -p test_runner -- vamiga category Copper
cargo run -p test_runner -- vamiga category Blitter
cargo run -p test_runner -- vamiga category Denise
cargo run -p test_runner -- vamiga category Paula

# Override the number of frames simulated (e.g. 16 frames):
cargo run -p test_runner -- vamiga test coptim1 frames 16

# Limit batch execution count during active development:
cargo run -p test_runner -- vamiga category Copper limit 5

# Inspect runnable vs deferred test suite breakdown across roadmap phases:
cargo run -p test_runner -- vamiga list deferred
```

### 5. Diagnostic Diff Reporting & Root-Cause Guidelines
When a test fails, the runner outputs a structured mismatch summary:
```text
❌ FAIL: 5500/204060 mismatched pixels (2.69%)
    Mismatch at (128, 42): actual=(0, 0, 0), expected=(255, 128, 0)
    Mismatch at (129, 42): actual=(0, 0, 0), expected=(255, 128, 0)
```

> [!IMPORTANT]
> **Zero Coordinate Nudging:** Per [`.agents/rules/structural-root-cause.md`](../../.agents/rules/structural-root-cause.md), never fix pixel mismatches by nudging display coordinates ($\pm 1$ offset hacks). Investigate the underlying silicon timing: Color Clock phase alignment (`CCK1` vs `CCK2`), Copper instruction strobe delay, DMA slot contention, or display window (`DIWSTRT`/`DIWSTOP`) clipping.
