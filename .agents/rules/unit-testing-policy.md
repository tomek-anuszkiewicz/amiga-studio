---
trigger: model_decision
description: Mandatory comprehensive unit test coverage for functional and utility classes; headless integration tests for GUI.
---

# Comprehensive Unit Testing Policy (Functional & Utility Classes)

To guarantee emulator fidelity, zero regressions, and complete opcode support, this project enforces strict unit and integration testing mandates across all workspace crates.

---

## 1. Functional & Utility Classes: Mandatory Comprehensive Unit Tests

Every non-UI functional module, utility class, parser, decoder, evaluator, data structure, and hardware state machine **must have comprehensive unit test coverage**:

1. **Disassemblers & Parsers (`crates/disassembler/src/lib.rs`, `crates/debugger/src/assembler.rs`):**
   - **Exhaustive Opcode Coverage:** Must test every supported M68000 instruction mnemonic, addressing mode, and condition code variant.
   - **Zero Unintended Fallbacks:** Unit tests must assert that valid instructions never fall back to `DATA.W` or unformatted raw bytes.
   - **Roundtrip Validation:** Where applicable, assemble -> disassemble roundtrip tests must verify consistency between assembler and disassembler.
   - **Syntax & Error Handling:** Assemblers and parsers must test invalid syntax, range bounds, odd addresses, and error reporting.

2. **Core Subsystems & Utilities (`disassembler`, `debugger`, `rtc`, `memory_bus`, `config`):**
   - **Breakpoint & Watchpoint Evaluators:** Test address matching, register conditions (`==`, `!=`, `<`, `>`, `<=`, `>=`), and compound expressions.
   - **Trace Buffers & History:** Test circular buffer wrapping, cursor seeking, boundary clamping, and zero-allocation ring buffers.
   - **Binary Loaders:** Test RAM injection, overlay toggling, prefetch priming, and entry point setup.
   - **Hardware Timers & RTC:** Test BCD conversion, leap year calculation, clock division, and rollover.

---

## 2. GUI Components: Mandatory Headless Integration Tests

Unlike backend systems, GUI components under `crates/gui` must not rely on fragile, mocked unit tests of individual UI fragments. Instead, they must be validated through **end-to-end headless integration tests** (`crates/gui/tests/test_interactions.rs`):

1. **Simulated Context & Event Queues:**
   - Execute full-frame passes using headless `egui::Context::run(RawInput { ... }, |ctx| app.update_ui(ctx))`.
   - Simulate realistic user events: key presses (`F5`, `F8`, `F10`, `F11`, `F12`), text input, Enter/Escape commits, drag-and-drop.
2. **Layout & State Assertions:**
   - Verify view mode switching (`ViewMode::Developer` vs `ViewMode::ScreenOnly`).
   - Verify layout stability, dock widths, scroll regions, and zero-jitter editing.
   - Verify focus acquisition on active edit and clean dismissal on Escape or outside clicks.
3. **Zero Host Panics:**
   - Ensure consecutive frame renders produce valid shapes/textures without panicking under any state.

---

## 3. Test Placement Architecture: Dedicated `tests/` Directory Only (Zero Inline Tests in `src/`)

To guarantee clean separation between production logic and test harnesses, all tests across the workspace must reside strictly in external test suites:

1. **Dedicated `tests/` Directory Standard:**
   - For every workspace crate (e.g. `crates/audio/`, `crates/agnus/`, `crates/machine_loop/`), all unit tests, integration tests, and regressions **must be placed in `crates/<crate>/tests/`** (e.g. `crates/<crate>/tests/test_<crate>.rs` or `crates/<crate>/tests/*.rs`).
2. **Strict Prohibition of Inline Tests in `src/`:**
   - Embedding `#[cfg(test)] mod tests { ... }` or `#[test]` inside `crates/<crate>/src/lib.rs` (or any other `src/*.rs` file) is strictly forbidden across all workspace crates.
3. **Core Architectural Rationale:**
   - **Pure Production Code:** Production code in `src/` remains lean, uncluttered, and readable. Static analysis, dead-code detection, and file size limits ($\le 800$ lines) reflect genuine runtime code.
   - **Decoupled API Verification:** External test files compile as distinct crates, forcing tests to exercise modules strictly through public interfaces as downstream consumers (`machine_loop`, `debugger`, `gui`) do.
   - **Zero Host Panics Validation:** Keeps runtime panic checks in CI (`test_zero_runtime_panics_or_unwraps`) strictly focused on production code without needing test-specific exemptions.

---

## 4. Definition of Done Checklist for Testing

Before declaring any feature, bug fix, or opcode implementation complete:
- [ ] Are all new or modified functional methods backed by unit tests?
- [ ] Are all tests located strictly in `crates/<crate>/tests/`, with zero inline `#[cfg(test)]` in `crates/<crate>/src/`?
- [ ] Does the disassembler handle all expected opcode patterns without falling back to `DATA.W`?
- [ ] Do all unit tests run fast (< 1s total) and deterministic with zero race conditions?
- [ ] Are GUI changes verified with headless integration tests in `crates/gui/tests/test_interactions.rs`?
- [ ] Do all workspace test suites pass (`cargo test --workspace` / target-directed tests)?

---

## 5. Bug Fixing & Defect Resolution: Repro-First Mandate
Whenever resolving a bug, timing divergence, or instruction failure, follow the mandatory Red-Green-Refactor protocol in [`repro-first.md`](repro-first.md):
1. Write an isolated, failing reproduction test in `crates/<crate>/tests/`.
2. Confirm the failure on current unmodified code.
3. Implement the minimal fix in `crates/*/src/`.
4. Verify non-regression across adjacent suites and keep the test committed as a permanent regression sentinel.

---

## 6. Execution Skills for Testing
- **Crate Unit & Integration Test Scaffolding:** Follow [`scaffold-crate-tests`](../skills/scaffold-crate-tests/SKILL.md) to author comprehensive external test suites under `crates/<crate>/tests/test_<crate>.rs`.
- **CPU Silicon Cycle Verification:** Follow [`m68k-singlestep-test`](../skills/m68k-singlestep-test/SKILL.md) when validating instructions against Tom Harte physical silicon vectors (`SingleStepTests-680x0`).

