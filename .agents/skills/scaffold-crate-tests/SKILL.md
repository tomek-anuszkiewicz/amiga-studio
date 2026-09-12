---
name: scaffold-crate-tests
description: Scaffold external unit and integration test suites in dedicated crates/*/tests/ with zero inline tests in src/.
---

# Recipe: Crate Test Scaffolding & Coverage Expansion

This skill provides the operational procedure for authoring comprehensive unit and integration tests across workspace crates under `crates/<crate>/tests/test_<crate>.rs` per [`.agents/rules/unit-testing-policy.md`](../../rules/unit-testing-policy.md).

---

## 1. When to Use This Skill

Activate this skill whenever:
- Implementing a new workspace crate or adding non-trivial functional modules (parsers, decoders, state machines, timers).
- Expanding test coverage to cover boundary conditions, edge cases, or regression fixes.
- Scaffolding headless integration tests for GUI interactions.
- Verifying compliance with the dedicated `tests/` directory policy (`test_zero_inline_tests_in_crates_src`).

---

## 2. Core Testing Invariants

1. **Dedicated `tests/` Directory Only:** All tests must reside in `crates/<crate>/tests/` (e.g., `crates/<crate>/tests/test_<crate>.rs`).
2. **Zero Inline Tests in `src/`:** Never place `#[cfg(test)] mod tests { ... }` or `#[test]` inside `src/` files.
3. **Decoupled Consumer Verification:** External tests compile as separate test crates and interact with modules solely through public APIs (`use <crate_name>::*;`).
4. **Zero Host Panics:** Guest code and boundary conditions must never trigger host process panics.

---

## 3. Step-by-Step Execution Workflow

### Step 1: Create or Locate Test Suite File
Ensure the test file resides in the dedicated `tests/` directory:
```powershell
New-Item -ItemType Directory -Force "crates/<crate>/tests"
New-Item -ItemType File -Force "crates/<crate>/tests/test_<crate>.rs"
```

### Step 2: Implement Test Coverage by Subsystem Archetype

#### Archetype A: Hardware State Machines & Timers (CIA, RTC, Audio, Paula)
- **Clock Rollover & Prescaling:** Test timer underflow, one-shot vs continuous modes, and E-Clock dividing.
- **BCD & Time Arithmetic:** Test BCD encoding/decoding, 24-hour rollover, leap year logic, and latched registers.
- **Interrupt Signals:** Test interrupt flag assertion, interrupt mask gating, and interrupt clear protocols.

#### Archetype B: Disassemblers & Parsers (M68000, Assembler, Debugger)
- **Exhaustive Opcode Matrix:** Test all instruction mnemonics, size variants (`.b`, `.w`, `.l`), and addressing modes.
- **Zero Fallbacks:** Assert that valid instructions never fall back to raw data words (`DATA.W`).
- **Roundtrip Validation:** Where applicable, assert that assembling source code and disassembling the resulting machine code produces identical mnemonics.

#### Archetype C: Buffer Management & Ring Buffers (Trace, History, FIFO)
- **Wrap-Around Integrity:** Verify ring buffer pointers wrap cleanly without data corruption or memory leaks.
- **Boundary Clamping:** Verify seek cursors, zero-length reads, and out-of-range seeks clamp gracefully without panicking.

#### Archetype D: Headless GUI Integration Tests (`crates/gui/tests/`)
- Test headless frame runs via `egui::Context::run(RawInput { ... }, |ctx| app.update_ui(ctx))`.
- Simulate keyboard events (`F5`, `F12`, `Escape`, `Enter`) and verify view mode switches without host panics.

### Step 3: Run & Verify Test Suite
Execute the targeted test suite:
```powershell
cargo test -p <crate_name>
```

### Step 4: Run Architecture Compliance Gate
Verify that the new tests reside strictly in `tests/` and comply with all architectural rules:
```powershell
cargo test -p test_runner --test test_architecture_rules -- test_zero_inline_tests_in_crates_src
cargo fmt --all -- --check
```

---

## 5. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Medium`
- **Context Savings:** Absorbs dozens of test function drafts, compiler trait errors, mock setup boilerplate, and test failure dumps.
- **Subagent Task Template:**
  - `TaskName`: "Scaffolding Tests: <crate_name>"
  - `TaskSummary`: "Creates comprehensive unit and integration test suites under crates/<crate_name>/tests/."
  - `Prompt`:
    ```markdown
    Scaffold external test suite for crate `<CRATE_NAME>`.
    Follow .agents/skills/scaffold-crate-tests/SKILL.md:
    1. Inspect public APIs in `crates/<crate_name>/src/`.
    2. Create `crates/<crate_name>/tests/test_<crate_name>.rs` (never inline in `src/`).
    3. Cover state transitions, boundary limits, and error vectors.
    4. Verify tests pass: `cargo test -p <crate_name>`.
    5. Verify architecture gate: `cargo test -p test_runner --test test_architecture_rules -- test_zero_inline_tests_in_crates_src`.
    6. Return strictly the Test Coverage Matrix below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### 🧪 Crate Test Scaffolding Report
  - **Crate Tested:** `<crate_name>`
  - **Test File Created:** [`crates/<crate_name>/tests/test_<crate_name>.rs`](file:///d:/Programowanie/Amiga/crates/<crate_name>/tests/test_<crate_name>.rs)
  - **Test Count Added:** `<N>` unit/integration tests
  - **Test Coverage Matrix:**
    | Test Function | Target Feature | Simulated Edge Conditions |
    | :--- | :--- | :--- |
    | `test_state_reset` | Default State | Cold reset vectors, flag clears |
    | `test_boundary_wrap` | Counter / Buffer | 16-bit / 32-bit arithmetic overflow |
  - **Test Execution Result:** `cargo test -p <crate_name>`: `<N>` passed; 0 failed.
  - **Zero Inline Tests Gate:** `test_zero_inline_tests_in_crates_src` passed.
  ```
