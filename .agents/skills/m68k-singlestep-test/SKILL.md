---
name: m68k-singlestep-test
description: >-
  Use this skill when running, validating, or debugging M68000 CPU instructions against the cycle-exact SingleStepTests test suite in ref_src/SingleStepTests-m68000/v1/. Covers running test cases, interpreting test JSON schemas, diagnosing register, CCR, and prefetch mismatches, and handling address errors.
---

# M68000 SingleStepTests Verification & Debugging Runbook

This skill guides the validation and debugging of the M68000 CPU implementation against the exhaustive 127-file test suite located in `ref_src/SingleStepTests-m68000/v1/`.

---

## 1. Test Suite Location & Structure

- **Path:** `ref_src/SingleStepTests-m68000/v1/*.json`
- **Naming Pattern:** `<INSTRUCTION>.<size>.json` (e.g., `ADD.b.json`, `MOVE.w.json`, `LSR.l.json`) or `<INSTRUCTION>.json` (e.g., `NOP.json`, `MULU.json`, `ILLEGAL_LINEA.json`).
- **Specification Document:** Detailed design and Rust structs are in [CPU SingleStepTests.md](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design/CPU%20SingleStepTests.md).

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

### Running Specific Instruction Tests via Cargo
```powershell
# Run only NOP tests
cargo test tests::cpu::test_nop

# Run a specific arithmetic or logic suite
cargo test tests::cpu::test_add_b
cargo test tests::cpu::test_move_w

# Run exception and illegal instruction tests
cargo test tests::cpu::test_illegal_linea
cargo test tests::cpu::test_trap

# Run all CPU single step tests
cargo test tests::cpu
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

## 5. Isolating a Single Failing Test Case

If a test file containing thousands of cases fails, print the test case name:
1. Check the assertion error: `"RAM mismatch at 0x... in test: <NAME>"` or `"SR mismatch in test: <NAME>"`.
2. Filter or run only that specific test case in the test harness by checking `test.name.contains(...)` for fast iteration.
