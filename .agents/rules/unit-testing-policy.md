---
trigger: always_on
description: Mandatory comprehensive unit test coverage for functional and utility classes; headless integration tests for GUI.
---

# Comprehensive Unit Testing Policy (Functional & Utility Classes)

To guarantee emulator fidelity, zero regressions, and complete opcode support, this project enforces strict unit and integration testing mandates across all workspace crates.

---

## 1. Functional & Utility Classes: Mandatory Comprehensive Unit Tests

Every non-UI functional module, utility class, parser, decoder, evaluator, data structure, and hardware state machine **must have comprehensive unit test coverage**:

1. **Disassemblers & Parsers (`crates/disassembler/src/disassembler.rs`, `crates/debugger/src/assembler.rs`):**
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

## 2. Whole-Machine Loop Integration Mandate (Tier 2 Custom Chip Verification)

Isolated unit tests verify individual logic gates, ALU operations, and formula evaluations. However, hardware silicon behavior emerges from the tight, synchronized coordination between custom chips, DMA channels, and CPU bus cycles.

### A. The 4-Question Integration Checklist (When a Tier 2 Test is Mandatory)
Whenever any custom chip, coprocessor, peripheral device, or interrupt line is added, modified, or extended, evaluate this objective 4-question checklist:
1. **Control & Strobe Mutation:** Does changing register bits (e.g. `DMACON`, `INTENA`, `COPCON`) gate or trigger execution?
2. **Autonomous Memory Transfer:** Does the subsystem read or write Chip RAM over multiple CCK cycles (Copper, Blitter, Denise, Audio, Floppy)?
3. **Cross-Chip Signal & Interrupt Routing:** Does the subsystem raise an interrupt line, latch status, or send electronic signals to other chips (Paula -> CPU, Agnus -> Denise, CIAs -> CPU)?
4. **Hardware Contention & Wait States:** Does the subsystem compete for Chip RAM slots or stall the CPU?

If the answer to **any** of the four questions is **YES**, a corresponding Tier 2 integration test in `crates/machine_loop/tests/test_<subsystem>_machine_integration.rs` is **mandatory**.

### B. The 4 Standardized Integration Test Archetypes (Recipes)
Every Tier 2 integration test must follow one of four standardized, reproducible archetypes using `MachineHarness`:
1. **Archetype A: Autonomous Execution & Memory Mutation:**
   - Setup memory/registers -> trigger strobe -> step $N$ CCKs -> assert mutated Chip RAM buffer or target register.
2. **Archetype B: Signal Escalation & CPU Autovector:**
   - Setup interrupt vector table -> trigger event -> step CCKs -> assert CPU IPL escalation and PC vector branch.
3. **Archetype C: DMA Master & Channel Gatekeeping:**
   - Step with channel DMA bit set (assert progress) -> step with channel bit or master `DMAEN` cleared (assert zero progress).
4. **Archetype D: Contention & Dual-Bus Concurrency:**
   - Run active Chip RAM transfer -> assert CPU wait states when targeting Chip RAM alongside zero wait states when executing in Fast RAM.

### C. Synthetic, Zero-Asset Architecture:
- Tier 2 tests must remain fast (< 5ms per test) and self-contained in Chip RAM using `MachineHarness`.
- Never rely on Kickstart ROMs or ADF floppy disks for Tier 2 verification.
- Pinpoint exact subsystem regressions before executing heavy Tier 3 test runners.

---


## 3. GUI Components: Mandatory Headless Integration Tests

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

## 4. Test Placement Architecture: Dedicated `tests/` Directory & 3-Tier Taxonomy

To guarantee clean separation between production logic and test harnesses, all tests across the workspace must reside strictly in external test suites:

1. **Dedicated `tests/` Directory Standard:**
   - For every workspace crate (e.g. `crates/audio/`, `crates/agnus/`, `crates/machine_loop/`), all unit tests, integration tests, and regressions **must be placed in `crates/<crate>/tests/`** (e.g. `crates/<crate>/tests/test_<crate>.rs` or `crates/<crate>/tests/*.rs`).
2. **Canonical Test Naming Convention (`test_<name>.rs`):**
   - In Cargo, every `.rs` file directly inside `crates/<crate>/tests/` compiles into an independent test binary.
   - **All test files must strictly start with the `test_` prefix** (e.g. `test_anomaly.rs`, `test_line.rs`). Bare filenames without `test_` are prohibited. Shared non-test helper modules must reside inside subdirectories (e.g. `tests/common/mod.rs`).
3. **1:1 Multi-Module Parity:**
   - Multi-module crates with distinct computational modules (e.g. `blitter`, `disassembler`, `floppy`, `config`, `physical_memory`) must maintain dedicated 1:1 unit test files mirroring each submodule (`line.rs` -> `test_line.rs`, `minterm.rs` -> `test_minterm.rs`, `mfm.rs` -> `test_mfm.rs`).
4. **Strict Prohibition of Inline Tests in `src/`:**
   - Embedding `#[cfg(test)] mod tests { ... }` or `#[test]` inside `crates/<crate>/src/<crate>.rs` (or any other `src/*.rs` file) is strictly forbidden across all workspace crates.
5. **The 3-Tier Testing Taxonomy:**
   - **Tier 1 (Isolated Unit Tests, L1):** Fast, isolated tests of single crates and algorithmic modules (< 2s). Run via `python tools/run_tests.py --unit`.
   - **Tier 2 (Headless Multi-Crate Integration Tests, L2):** Cross-subsystem orchestration across machine loop, debugger, GUI, and bus routing (`machine_loop`, `debugger`, `gui`, `memory_bus`). Run via `python tools/run_tests.py --integration`.
   - **Tier 3 (Silicon Verification & Verification Harness, L3):** Tom Harte physical silicon SingleStepTests, Cartesian DMA contention sweeps, vAmigaTS RGB24 viewport matchers, opcode benchmarks, and architecture rules. Run via `python tools/run_tests.py --harness`.
6. **Core Architectural Rationale:**
   - **Pure Production Code:** Production code in `src/` remains lean, uncluttered, and readable. Static analysis, dead-code detection, and file size limits ($\le 800$ lines) reflect genuine runtime code.
   - **Decoupled API Verification:** External test files compile as distinct crates, forcing tests to exercise modules strictly through public interfaces as downstream consumers (`machine_loop`, `debugger`, `gui`) do.
   - **Zero Host Panics Validation:** Keeps runtime panic checks in CI (`test_zero_runtime_panics_or_unwraps`) strictly focused on production code without needing test-specific exemptions.

---

## 5. Automated Verification Gates & Change-Coupling Enforcement

To prevent shallow scaffolding and untested code from entering the repository, testing is enforced through automated gates and Git hooks:

1. **Change-Coupling Gate (`tools/check_test_coupling.py`):**
   - Whenever a commit or working tree changeset modifies or adds production code under `crates/<crate>/src/`, it **must also modify or add test files under `crates/<crate>/tests/`**.
   - Committing changes to `src/` without accompanying test changes is strictly blocked by the Git pre-commit hook and `pre_flight.py`.

2. **Minimum Test & Assertion Density (`test_architecture_rules.rs`):**
   - Every workspace crate must define at least **2 active `#[test]` functions** and at least **10 assertions** (`assert!`, `assert_eq!`, `assert_ne!`).
   - Single-test placeholder scaffolding is strictly forbidden.

3. **Public API Coverage Scanner (`tools/audit_api_coverage.py`):**
   - Scans all public functions (`pub fn`) declared in `src/` and verifies that they are referenced and tested in unit/integration test suites.
   - Peripheral and utility crates (`joystick`, `mouse`, `keyboard`, `game_ports`, `rtc`, `parallel_port`, `serial_port`, `frame_builder`) must maintain 100% public API test coverage.

4. **Git Pre-Commit Hook (`.git/hooks/pre-commit`):**
   - Automatically executes `tools/check_polish.py --git`, `tools/check_test_coupling.py --staged`, and `tools/pre_flight.py --quick` on every `git commit`.

---

## 6. Definition of Done Checklist for Testing

Before declaring any feature, bug fix, or opcode implementation complete:
- [ ] Are all new or modified functional methods backed by unit tests?
- [ ] Are all tests located strictly in `crates/<crate>/tests/`, with zero inline `#[cfg(test)]` in `crates/<crate>/src/`?
- [ ] Does the disassembler handle all expected opcode patterns without falling back to `DATA.W`?
- [ ] Do all unit tests run fast (< 1s total) and deterministic with zero race conditions?
- [ ] Are GUI changes verified with headless integration tests in `crates/gui/tests/test_interactions.rs`?
- [ ] Do all workspace test suites pass (`cargo test --workspace` / target-directed tests)?

---

## 7. Bug Fixing & Defect Resolution: Repro-First Mandate
Whenever resolving a bug, timing divergence, or instruction failure, follow the mandatory Red-Green-Refactor protocol in [`repro-first.md`](repro-first.md):
1. Write an isolated, failing reproduction test in `crates/<crate>/tests/`.
2. Confirm the failure on current unmodified code.
3. Implement the minimal fix in `crates/*/src/`.
4. Verify non-regression across adjacent suites and keep the test committed as a permanent regression sentinel.

---

## 8. Execution Skills for Testing
- **CPU Silicon Cycle Verification:** Follow [`m68k-singlestep-test`](../skills/m68k-singlestep-test/SKILL.md) when validating instructions against Tom Harte physical silicon vectors (`SingleStepTests-680x0`).

