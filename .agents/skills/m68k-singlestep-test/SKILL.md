---
name: m68k-singlestep-test
description: Run and diagnose M68000 instructions against cycle-exact Tom Harte physical silicon SingleStepTests test vectors.
---

# M68000 SingleStepTests Verification & Debugging Runbook

This skill guides the validation and debugging of the M68000 CPU implementation against the physical silicon single-step test suite:
- **Tom Harte SingleStepTests-680x0:** `ref_src/SingleStepTests-680x0/68000/v1/*.json` (124 JSON files, ~1,000,000 tests captured from real 68000 silicon pins).

---

## 1. Test Suite Locations & Structure

- **Path:** `ref_src/SingleStepTests-680x0/68000/v1/<INSTRUCTION>.<size>.json`
- **Specification Document:** Complete architecture, comparative matrix, and Rust data structures are in [CPU SingleStepTests.md](../../../Obsidian/Amiga/Design/CPU%20SingleStepTests.md).

---

## 2. Test JSON Schema Summary

Each test case contains:
1. `name`: Descriptive test case title with opcode and addressing mode.
2. `initial`:
   - `d0`-`d7`, `a0`-`a6`, `usp`, `ssp`, `sr`, `pc`
   - `prefetch`: `[pf0, pf1]` (two 16-bit words already in the queue)
   - `ram`: Array of `[address, byte_value]` pairs
3. `final`:
   - Expected values for all registers, active stack pointer (`usp`/`ssp`), `sr`, `pc`, `prefetch`, and final `ram` entries.
4. `transactions`:
   - Array of bus cycle activities: `r` (read), `w` (write), `n` (idle), `t` (TAS), `re` (read address error), `we` (write address error).
5. `length`: Total execution clock cycles.

---

## 3. Running Test Suites

### Running SingleStepTests via Cargo Test Runner
```powershell
# Fast sample run (default: 50 cases per suite, ~5.8s):
cargo test -p test_runner --test test_singlestep

# Full exhaustive verification (~300,000 cases across all 77 suites, ~14s):
$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep

# Full exhaustive run for a single instruction:
$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep -- test_add_b

# Custom sample limit (e.g. 500 cases):
$env:SINGLESTEP_LIMIT = "500"; cargo test -p test_runner --test test_singlestep -- test_move_w
```

---

## 4. Debugging & Root-Cause Diagnosis

When a test fails, follow this systematic diagnostic checklist:

### A. Condition Code Register (CCR / SR) Mismatches
- Compare bits 0-4 of `SR`:
  - Bit 4: **X** (Extend)
  - Bit 3: **N** (Negative)
  - Bit 2: **Z** (Zero)
  - Bit 1: **V** (Overflow)
  - Bit 0: **C** (Carry)
- **Common Pitfalls:**
  - Instructions like `MOVE` update $N$ and $Z$, clear $V$ and $C$, but **leave $X$ untouched**.
  - Logical operations (`AND`, `OR`, `EOR`) always clear $V$ and $C$.
  - Bit operations (`BTST`, `BSET`, `BCLR`, `BCHG`) only affect $Z$; all other flags ($X, N, V, C$) remain unchanged.
  - Subtraction/Comparison borrow: check whether borrow flag $C$ logic adheres to M68000 semantics ($Borrow = (Source > Destination)$).

### B. Program Counter (PC) & Prefetch Queue Alignment
- Remember the M68000 two-word prefetch pipeline:
  - `IR` (Instruction Register): holds the opcode being executed.
  - `IRC` (Prefetch Queue): holds the next prefetched 16-bit word.
- At instruction completion, the test asserts that `pc` and `prefetch: [pf0, pf1]` match the expected post-instruction state.
- If `pc` is 2 or 4 bytes ahead or behind:
  - Verify that immediate operands or extension words (`.w` / `.l`) were fetched from the prefetch queue and refilled from memory.
  - Verify whether the final instruction prefetch step occurred before instruction retirement.

### C. Stack Pointer & Supervisor Mode Transitions
- Check the $S$ bit (bit 13) in $SR$:
  - When $S = 1$, active $A_7$ is $SSP$.
  - When $S = 0$, active $A_7$ is $USP$.
- If $USP$ or $SSP$ fails assertions, verify whether the active $A_7$ accessor properly redirects to $SSP$ or $USP$.

### D. Address Errors (Vector 3, `$00000C`)
- If a test tests unaligned word or long access:
  - Word (`.w`) and long (`.l`) reads/writes to odd addresses (address bit 0 = 1) must abort execution and initiate the Address Error exception sequence.
  - Stack frame pushed:
    1. Function Code / Access Type (16-bit)
    2. Access Address (32-bit)
    3. Instruction Register opcode (16-bit)
    4. Status Register (16-bit)
    5. Program Counter (32-bit)
  - Next $PC$ is read from Vector 3 at address `$00000C`.

---

## 5. Test Result Tracking & Regression Diagnosis

The test harness automatically tracks test outcomes, diffs, and regressions across executions:

### A. Results Directory Structure (`.test_results/`)
- `.test_results/latest/<suite_name>.json`: Detailed outcomes of the most recent test run.
- `.test_results/previous/<suite_name>.json`: Rotated snapshot of the previous run used for regression diffing.
- `.test_results/summary.json`: Global repository-wide pass/fail matrix, percentages, and active failure names.

### B. Automated Regression & Progress Alerts
When running tests via `cargo test -p test_runner`, the test runner automatically compares the current run against the previous run:
- 🔴 **Regressions**: Tests that previously passed but failed after your changes:
  ```text
  ⚠️  [REGRESSION DETECTED] Suite 'Real68k::ADD.b': 1 test(s) that previously PASSED now FAILED!
     🔴 Broken: "001 [ADD.b D1, (d8, A4, Xn)] d334"
  ```
- 🟢 **Improvements**: Tests that were broken and are now passing:
  ```text
  🎉 [PROGRESS / FIX] Suite 'Real68k::MOVEA.w': 1 test(s) that previously FAILED now PASSED!
     🟢 Fixed:  "3c7a [MOVEA.w (d16, PC), A6] 19"
  ```

### C. Agent Inspection CLI Commands
Agents can quickly inspect test coverage or check for regressions without searching log outputs:
```powershell
# Check whether your latest code edits caused any regressions
cargo run -p test_runner -- --diff

# Print a tabular summary of pass rates and active failure cases across all opcodes
cargo run -p test_runner -- --summary

# Run a specific opcode suite with live diagnostics
cargo run -p test_runner -- --suite ADD.b
```

---

## 6. Diagnostic Failure Reports & Cycle Logging

When a test case fails, the runner outputs a structured diagnostic block:
```text
================================================================================
❌ TEST FAILURE: "049 ADD.b 6, (A2) 5c12"
   Location: ref_src/SingleStepTests-680x0/68000/v1/ADD.b.json [Test #49]
   Cycle:    12 clock cycles (approx. 6 CCK cycles)
--------------------------------------------------------------------------------
Differences detected:
  • Status Register / CCR Mismatch:
      Expected: 0xA008 [T:1 S:1 I:0 X:0 N:1 Z:0 V:0 C:0]
      Actual:   0xA00C [T:1 S:1 I:0 X:0 N:1 Z:1 V:0 C:0]
      Diff:     Z flag: expected 0, got 1 (unexpectedly SET)
  • Register D0:
      Expected: 0x0000002A
      Actual:   0x00000000 (diff: -42)
================================================================================
```
- **Cycle / CCK:** Shows the expected execution duration in master clock cycles and Amiga CCK cycles ($1\ \text{CCK} = 2\ \text{clocks}$, $1\ \text{bus cycle} = 4\ \text{clocks} = 2\ \text{CCK}$).
- **Decomposed CCR flags:** Shows exactly which flag ($X, N, Z, V, C$) diverged and whether it was unexpectedly SET or CLEARED.

---

## 7. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Pro High` (Deep micro-architecture analysis & silicon vector trace diagnosis).
- **Context Savings:** Absorbs hundreds of megabytes of raw Tom Harte test vectors, JSON schemas, and cycle-by-cycle trace dumps without polluting the main conversation.
- **Subagent Task Template:**
  - `TaskName`: "Diagnosing Silicon Vector: <opcode_or_suite>"
  - `TaskSummary`: "Validates M68000 micro-steps against Tom Harte silicon test vectors and isolates cycle or CCR discrepancies."
  - `Prompt`:
    ```markdown
    Execute single-step validation for suite: <SUITE_NAME>.
    Follow .agents/skills/m68k-singlestep-test/SKILL.md:
    1. Run `cargo test -p test_runner --test test_singlestep -- <suite_filter>`.
    2. If failure occurs, inspect the JSON vector in `ref_src/SingleStepTests-680x0/68000/v1/`.
    3. Trace micro-step progression in `crates/m68000/src/instructions/`.
    4. Diagnose root cause (CCR formula, bus idle timing, or prefetch order).
    5. Return strictly the Silicon Discrepancy Vector report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### 🔬 M68k SingleStep Diagnostic Vector
  - **Suite Evaluated:** `<suite_name>`
  - **Outcome:** [ALL PASS | FAILURE ISOLATED]
  - **Failing Vector Number:** `#<index>` (e.g. `#49`)
  - **Opcode Hex & Disassembly:** `$<code>` (`<mnemonic>`)
  - **Cycle Mismatch:** Cycle `<cycle_num>` (Expected `<expected_bus_activity>`, Actual `<actual_bus_activity>`)
  - **State Discrepancy:**
    - **CCR Diff:** `<flag>`: expected `<val>`, got `<val>`
    - **Register Diff:** `<reg>`: expected `$HEX`, got `$HEX`
  - **Root Cause & Code Location:** [`<file>.rs:L<line>`](file:///d:/Programowanie/Amiga/crates/m68000/src/instructions/<file>.rs#L<line>) — `<concise_explanation>`
  ```
