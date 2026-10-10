# Test Execution Reference

This reference contains test commands, verification routing and diagnostic details. For an introduction to what each kind of test establishes, read the [testing overview](../docs/testing.md). Run commands from the repository root. Current options are available with `python tests/verify.py --help`; `--plan` previews selected commands.

## Completion verification

Verification orchestration and coverage result processing live in `tests/`;
worktree utilities live in `tools/harness/`. The diary appender belongs to `.agents/skills/compact-diary/scripts/`. Concrete project audits and checks live in `tests/checks/`. Run these scripts from the repository root. Their
command-line options and verification scope are unchanged by this layout.

The `test_runner` suites retain emulator execution checks and reference-result invariants. Standalone checks of its analyzers, statistics, PRNG, persistence, host setup, synthetic bus, program injection, report schemas, parsers and frame matcher are excluded; mixed suites retain their real execution cases. Benchmark CSV and empirical anomaly checks use stored results and do not establish fresh performance measurements.

Commit verification starts with ordinary workspace crate tests under LLVM coverage instrumentation, excluding `test_runner`. This is automatic, with no coverage switch. Only this execution contributes to the coverage report; the remaining quality gates and harness suites use normal Cargo artifacts. `cargo-llvm-cov` uses its separate `target/llvm-cov-target` cache, while normal Cargo commands retain their usual target directory. Raw measurement counters and the previous summary are cleared before each measurement, without deleting the build caches. Yellow `[INFO]` messages identify the instrumented and uninstrumented phases and summarize line, function and region coverage. Full summary data is saved to `.agent/tmp/coverage/summary.json`; there is no percentage threshold. Coverage does not measure performance or prove assertion quality. Stable coverage does not include doctest execution; branch coverage is not enabled.

Install the required tooling once:

```powershell
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

Run `python tests/verify.py` without arguments (or with `--help`) to display the verification sets, automatic CPU-suite selection rules, plan behavior and command examples. This prints instructions and exits successfully without running verification.

Verification runs on the current working tree and does not require staging or pending changes. For milestone closure run both separate sets, once each:

```powershell
python tests/verify.py --commit
python tests/verify.py --milestone
```

Routine commits use only the first command. The profiles are invoked separately. Commit first runs ordinary workspace crate tests with coverage, reports the measured coverage, then runs quality gates, standard `test_runner` suites and Python tool contracts without coverage instrumentation. Only Cartesian DMA and full SingleStepTests are conditional on staged, unstaged and nonignored new paths. Every commit scans all repository-owned text files against the English dictionary and reviewed project vocabulary, regardless of staging or changes, using `tests/checks/check_english.json`. Milestone runs the same full language audit. vAmigaTS Cargo suites and chipset workload tests have been removed; capture runs and chipset throughput comparisons are excluded from automatic verification. It does not run commit checks or automatically select DMA/SingleStep.

Python tool contracts live beside their owning tools. Commit verification runs
`unittest discover` separately for each directory below; the tools do not run
these tests during normal operation. Run a focused suite from the repository root
with `python -m unittest discover -s <directory> -v`, optionally adding
`-p <test_file.py>` to select one file.

| Directory | Coverage |
| --- | --- |
| [`tools/bootstrap/tests`](../tools/bootstrap/tests/) | Bootstrap and Qdrant provisioning command contracts |
| [`tools/bootstrap/conversion/tests`](../tools/bootstrap/conversion/tests/) | Shared client, transport, cache and PDF/HTML publication contracts |
| [`tools/bootstrap/pdf-to-markdown/tests`](../tools/bootstrap/pdf-to-markdown/tests/) | PDF stage validation, artifacts, restart and resume contracts |
| [`tools/amiga-rag-mcp-server/tests`](../tools/amiga-rag-mcp-server/tests/) | RAG CLI adapter contracts and real Codex MCP discovery |

The code-quality auditor accepts no arguments and always checks dead code,
visibility and accessor conventions across the workspace. Commit verification
runs it once; milestone verification does not repeat it. Every finding returns
exit code 1. Run `python tests/checks/audit_code_quality.py` for the same full audit
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
[sync-design-docs](../.agents/skills/sync-design-docs/SKILL.md),
[obsidian-vault-linking](../.agents/skills/obsidian-vault-linking/SKILL.md),
[audit-semantic-parity](../.agents/skills/audit-semantic-parity/SKILL.md),
[compact-diary](../.agents/skills/compact-diary/SKILL.md) and
[roadmap-maintenance](../.agents/skills/roadmap-maintenance/SKILL.md) at their defined
triggers. Semantic milestone review is required by its skill. A successful
`verify.py --milestone` alone does not certify these separate skill checks.

DMA and SingleStep are independent test sets. They can run alone, together, or alongside a completion profile:

```powershell
python tests/verify.py --verify-dma
python tests/verify.py --verify-singlestep
python tests/verify.py --verify-dma --verify-singlestep
python tests/verify.py --commit --verify-dma --verify-singlestep --plan
```

With no completion profile, `--verify-dma` runs only Cartesian DMA and `--verify-singlestep` runs only full SingleStepTests. Standalone sets use the current working tree, require no staging, and run no quality gates, ordinary crate tests, or Python contracts. With `--commit` or `--milestone`, the switches add their sets to that profile. Each selected suite runs once, even when changes also select it. Standalone runs do not replace completion verification. Add `--plan` to inspect commands without executing checks. There are no hooks. Unknown arguments are rejected.

The verifier includes formatting, Clippy, architecture rules, and per-commit hardware/code audits. Ordinary-crate coverage measures executed code; there is no public-function name-matching gate. All profiles accept clean trees, unstaged changes and new files. Change selection includes both endpoints of renames. Completion evidence must match the final committed source; rerun verification if that source changes. All profiles reject results if the source or index changes during execution. Ignored diagnostics belong in `.agent/tmp/`. The verifier does not stage or commit files. It only selects external commands, launches them and aggregates exit codes; source/index stability is an orchestration guard. Quality commands are declared in `tests/verify.py`. During execution, `[RUN]` identifies the active check and `[CMD]` prints its exact command immediately before launching it; explicit environment overrides appear as `[ENV]`. Full child output is saved in a unique log under `.agent/tmp/verify/`. `--log-level full|warn|error` controls child output: `full` streams everything, `warn` (default) shows warnings and errors, and `error` shows only errors. Filtered modes include up to 12 indented diagnostic context lines, limited to 300 characters per line. Commands and final results remain visible in every mode. If a failed command emits no recognized error marker, its last 12 log lines are shown as failure context; failed commands also show the full log path before the final FAIL result. There is no initial test-command preview. `--plan` starts with an explicit preview-only message and does not launch commands.

Focused development diagnostics also use `verify.py`:

```powershell
python tests/verify.py --unit
python tests/verify.py --integration
python tests/verify.py --harness
python tests/verify.py --all --plan
```

`--unit` runs the former isolated-crate tier and golden-row fixtures;
`--integration` runs memory_bus, machine_loop, debugger and GUI tests;
`--harness` runs architecture, benchmark smoke and Cartesian DMA. `--all` retains the former tier runner's meaning:
unit + integration, without harness or full SingleStepTests. Diagnostic selectors
may be combined with each other and explicit CPU selectors; each CPU suite runs
once. They run without coverage or completion gates and must be invoked separately
from `--commit` and `--milestone`. `--plan` and `--log-level` apply to diagnostics
as well. With no arguments the entry point still displays help.

The two expensive environments use CPU + synthetic TestMemoryBus, not production MemoryBus or PhysicalMemory. Their trigger paths are owned by [verify.py](verify.py):

| Trigger | Cartesian DMA | Full SingleStepTests |
| --- | --- | --- |
| CPU/config crate, shared AddressBus/BusResult contract, common synthetic harness, vectors, workspace manifest/lock/config | Yes | Yes |
| DMA harness or its test target | Yes | No |
| SingleStep runner/reporter or its test target | No | Yes |
| Production memory implementation, MemoryBus, machine loop, chips, GUI, tools, docs | No | No |
| Explicit force switch | `--verify-dma` | `--verify-singlestep` |
| Milestone | Only with explicit selector | Only with explicit selector |

Commit architecture tests run once as a fixed quality gate. Other standard commit tests run independently of changed paths, including ordinary CPU tests using real PhysicalMemory. The separate milestone set runs additional audits only; vAmigaTS Cargo suites and automatic chipset/capture runs have been removed.

Manual commands in the following sections and diagnostic selectors in `verify.py` remain development/diagnostic tools; they do not replace completion verification. Wall-clock microbenchmarks and external sampling profiles remain manual measurement tools: the verifier retains CPU benchmark regression fixtures. The chipset throughput comparison is available only as an explicit manual command. Before each authorized commit, `sync-design-docs` reviews only the intended changed paths and specifications selected through `tracked_paths`; unchanged documents and their dates stay untouched. Semantic milestone review, diary compaction, and roadmap pruning remain separate agent procedures.

---

## 1. M68000 SingleStepTests (Physical Hardware Silicon Verification)

The CPU core is validated against **Tom Harte's `SingleStepTests-680x0`** suite ([`ref_src/SingleStepTests-680x0/68000/v1/`](../ref_src/SingleStepTests-680x0/68000/v1)), consisting of 124 per-instruction test files and ~1,000,000 randomized test vectors captured directly from physical 68000 silicon pins.

### Execution Modes

#### A. Fast Smoke Test (Sampled)
By default, each instruction suite evaluates a sampled subset of 50 test cases (~5 seconds total):
```powershell
# Run sampled SingleStepTests across all implemented opcodes
cargo test -p test_runner --test test_singlestep

# Run tests for a specific instruction or family
cargo test -p test_runner --test test_singlestep test_nop
cargo test -p test_runner --test test_singlestep test_add_b
cargo test -p test_runner --test test_singlestep test_move_w
```

#### B. Full Exhaustive Verification (`SINGLESTEP_FULL`)
Setting `SINGLESTEP_FULL=1` disables sampling limits and executes **100% of all ~1,000,000 test vectors** across all suites in parallel (typically completes in ~5–6 seconds):

- **PowerShell (Windows):**
  ```powershell
  $env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep
  ```
- **Bash / Linux / macOS:**
  ```bash
  SINGLESTEP_FULL=1 cargo test -p test_runner --test test_singlestep
  ```

#### C. Custom Sample Limit (`SINGLESTEP_LIMIT`)
To evaluate an arbitrary sample size (e.g. 200 or 500 test cases per opcode):
```powershell
$env:SINGLESTEP_LIMIT = "500"; cargo test -p test_runner --test test_singlestep
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
cargo test -p test_runner --test test_dma_cartesian

# Run specific sub-suite
cargo test -p test_runner --test test_dma_cartesian test_dma_cartesian_system_and_traps
```

---

## 3. Automated Architecture Rules Compliance

Enforces architectural rules defined in [`AGENTS.md`](../AGENTS.md) (code formatting, file size limits $\le 800$ lines, zero runtime panics, zero custom macros, path privacy, and inlining rules):
```powershell
cargo test -p test_runner --test test_architecture_rules
```

---

## 4. CLI Diagnostic Tools & Regression Tracking

The `test_runner` crate provides a standalone CLI tool for inspecting coverage matrices, live failure diagnostics, and detecting regressions:

```powershell
# Display global pass/fail matrix and coverage summary across all opcodes
cargo run -p test_runner -- --summary

# Detect regressions and fixed tests compared to previous run
cargo run -p test_runner -- --diff

# Execute a single opcode suite directly with live diagnostic failure output
cargo run -p test_runner -- --suite ADD.b
```

- 🔴 **Regressions:** Tests that previously passed but now fail are highlighted with `⚠️ [REGRESSION DETECTED]`.
- 🟢 **Improvements:** Tests that previously failed but now pass are highlighted with `🎉 [PROGRESS / FIX]`.

---

## 5. Autonomous Testing Workflows & Slash Commands

For interactive pair-programming sessions with the AI Agent in the IDE chat, three repository skills drive the testing lifecycle:

| Skill | Role in Verification Lifecycle | When to Trigger |
| :--- | :--- | :--- |
| [`$test-runner`](../.agents/skills/test-runner/SKILL.md) | **Execution & Telemetry Engine** | Run tests across tiers, update snapshots in `.agent/tmp/singlestep/`, and report red/green regression diffs. |
| [`$integration-test-sprint`](../.agents/skills/integration-test-sprint/SKILL.md) | **Cascading Verification Protocol** | Drive multi-chip test sweeps (integration and hardware-capture scenarios), filter non-runnable tests, and cluster failures with a 2–3 attempt cap. |
| [`$synthesize-test-fixes`](../.agents/skills/synthesize-test-fixes/SKILL.md) | **Root-Cause Consolidation** | Audit recent commits or git diff, construct the 3-column diagnostic matrix (`[Symptom] \| [Location] \| [Mechanism]`), find the shared hardware law (common denominator), and strip local ad-hoc patches. |


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
cargo run -p test_runner -- vamiga --test coptim1

# Execute a specific test with verbose pixel difference coordinates:
cargo run -p test_runner -- vamiga --test coptim1 -v

# Run all runnable tests in a specific custom chip category:
cargo run -p test_runner -- vamiga --category Copper
cargo run -p test_runner -- vamiga --category Blitter
cargo run -p test_runner -- vamiga --category Denise
cargo run -p test_runner -- vamiga --category Paula

# Override the number of frames simulated (e.g. 16 frames):
cargo run -p test_runner -- vamiga --test coptim1 --frames 16

# Limit batch execution count during active development:
cargo run -p test_runner -- vamiga --category Copper --max-tests 5

# Inspect runnable vs deferred test suite breakdown across roadmap phases:
cargo run -p test_runner -- vamiga --list-deferred
```

### 5. Diagnostic Diff Reporting & Root-Cause Guidelines
When a test fails, the runner outputs a structured mismatch summary:
```text
❌ FAIL: 5500/204060 mismatched pixels (2.69%)
    Mismatch at (128, 42): actual=(0, 0, 0), expected=(255, 128, 0)
    Mismatch at (129, 42): actual=(0, 0, 0), expected=(255, 128, 0)
```

> [!IMPORTANT]
> **Zero Coordinate Nudging:** Per [`.agents/rules/structural-root-cause.md`](../.agents/rules/structural-root-cause.md), never fix pixel mismatches by nudging display coordinates ($\pm 1$ offset hacks). Investigate the underlying silicon timing: Color Clock phase alignment (`CCK1` vs `CCK2`), Copper instruction strobe delay, DMA slot contention, or display window (`DIWSTRT`/`DIWSTOP`) clipping.
