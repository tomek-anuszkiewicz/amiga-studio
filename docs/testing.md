# Amiga 500 Test Suite & Verification Architecture

This document describes verification against CPU vectors, integration scenarios, and reference captures. Passing a suite proves only its implemented assertions and executed cases.

## Completion verification

Stage the complete intended source snapshot, then run:

```powershell
python tools/harness/verify.py --commit
python tools/harness/verify.py --milestone
```

Use one mode: commit checks affected test environments; milestone checks all environments and adds documentation, hardware, code-quality, language, chipset throughput, and all runnable vAmigaTS capture checks. Add `--plan` to inspect the selection without executing it. There are no hooks. Unknown arguments are rejected.

The verifier includes formatting, Clippy, architecture rules, API-reference scanning, and per-commit hardware/code audits. API-reference scans are textual checks, not execution coverage. It rejects tracked unstaged changes and untracked files, uses both endpoints of renames, and rejects results if the index or source changes during execution. Ignored diagnostics belong in `.agent/tmp/`. The verifier does not stage or commit files.

Ordinary crate tests follow the transitive workspace dependency graph, including nested objects and test dependencies. For example, a Blitter change selects Blitter, Agnus, MemoryBus, machine, debugger, and GUI tests. PhysicalMemory changes select ordinary CPU tests, which instantiate real memory, but do not select synthetic silicon/DMA tests unless the shared AddressBus/BusResult contract changes.

Special `test_runner` suites are selected in [verify_selection.py](../tools/harness/verify_selection.py):

| Test environment | Change triggers |
| --- | --- |
| CPU + synthetic TestMemoryBus | CPU, shared bus contract, CPU harness/diagnostics, silicon vectors, or owning test files; SingleStep always uses full mode |
| CPU benchmark + real memory | CPU, PhysicalMemory, configuration/RTC, benchmark implementation, persisted benchmark artifacts, analysis tools, or owning test files |
| Full A500Machine / vAmiga harness | Any machine component, vAmiga harness/assets, or owning test files |
| Python tool contracts | Changes to the tested tool select its fixture group: harness/governance, conversion, bootstrap, MCP, or worktree; owning fixture changes select that fixture |

Workspace manifest/lock changes select all Rust and Python suites. Individual crate manifests select that crate and its reverse dependents. Changes to shared test helpers select their owning suites. Architecture tests run once as a fixed quality gate. Milestone additionally compares the existing chipset performance baseline and runs the complete runnable vAmiga capture catalog; deferred cases remain deferred. Capture summary failures are checked explicitly because the underlying CLI does not reliably set a failing exit code.

Manual commands in the following sections and `run_tests.py` remain development/diagnostic tools; they do not replace completion verification. Wall-clock microbenchmarks and external sampling profiles remain manual measurement tools: the verifier runs their regression fixtures, and milestone runs the chipset throughput comparison without recording or changing goldens. Semantic review, design synchronization, diary compaction, and roadmap pruning still require agent/developer work.

---

## 1. M68000 SingleStepTests (Physical Hardware Silicon Verification)

The CPU core is validated against **Tom Harte's `SingleStepTests-680x0`** suite ([`ref_src/SingleStepTests-680x0/68000/v1/`](../ref_src/SingleStepTests-680x0/68000/v1/)), consisting of 124 per-instruction test files and ~1,000,000 randomized test vectors captured directly from physical 68000 silicon pins.

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
| [`$test-runner`](../.agents/skills/test-runner/SKILL.md) | **Execution & Telemetry Engine** | Run tests across tiers, update snapshots in `.test_results/`, and report red/green regression diffs. |
| [`$integration-test-sprint`](../.agents/skills/integration-test-sprint/SKILL.md) | **Cascading Verification Protocol** | Drive multi-chip test sweeps (Tier 2 machine loop & Tier 4 vAmigaTS), filter non-runnable tests, and cluster failures with a 2–3 attempt cap. |
| [`$synthesize-test-fixes`](../.agents/skills/synthesize-test-fixes/SKILL.md) | **Root-Cause Consolidation** | Audit recent commits or git diff, construct the 3-column diagnostic matrix (`[Symptom] \| [Location] \| [Mechanism]`), find the shared hardware law (common denominator), and strip local ad-hoc patches. |
