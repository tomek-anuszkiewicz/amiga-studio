---
trigger: model_decision
description: Rust source file size limits (<= 800 lines in crates/*/src/), cohesion rules, and flat 1:1 opcode hierarchy under crates/cpu/src/instructions/.
---

# Module Cohesion & Rust Source File Size Guidelines (`.rs` Files Only)

## 1. Scope: Rust Source Code Files (`.rs`) Only
This guideline applies **strictly to Rust source code files (`.rs`)** within workspace crates (`crates/*/src/`).

**Documentation, Design Specifications & References Have NO Line Limits:**
Technical design specifications, architectural blueprints, and hardware reference manuals under `Obsidian/Amiga/Design/` and `Obsidian/Amiga/Reference/` have **no line count limits**. They can and should be as long, comprehensive, and exhaustive as necessary to serve as complete, authoritative single-source-of-truth documents. Do not artificially split or truncate markdown documentation.

## 2. Core Principle: "Aspect-per-File", Not "Class/Struct-per-File"

A foundational architectural mandate across all workspace crates is organizing code by **behavioral aspect / capability**, strictly rejecting the OOP anti-pattern of "one class/struct per file":

1. **Strict Prohibition of Struct-per-File Fragmentation:**
   - Do **NOT** fragment closely related structs, enums, helper types, and algorithms into an explosion of tiny 20-line micro-files (`Breakpoint.rs`, `BreakpointCondition.rs`, `BreakpointHit.rs`, etc.).
   - Fragmenting tightly coupled models across files adds cognitive noise, breaks cross-item visibility, and forces artificial `pub(crate)` ceremony.

2. **Aspect-per-File Cohesion:**
   - A single file must represent a **coherent behavioral aspect or domain capability** of the subsystem, encapsulating all data structures, enums, evaluations, and state mutations required for that capability.
   - **Canonical Reference (`crates/debugger/src/`):**
     - `assembler.rs`: Text-to-binary 68000 instruction parsing, tokenization, and machine code generation.
     - `breakpoints.rs`: Breakpoints, watchpoints, register/memory condition evaluators, and hit-testing logic.
     - `loader.rs`: Binary executable injection into RAM and CPU entrypoint initialization.
     - `session.rs`: Debugger lifecycle, operational state tracking, and session coordination.
     - `stepping.rs`: Instruction flow navigation (step into, step over, step out, run-to-cursor, frame advance).
     - `temporal.rs`: Time-travel rewind ring buffers, state snapshots, and timeline cursor scrubbing.
     - `trace.rs`: Execution trace history ring buffer, disassembly capture, and bottom log display.
   - **Peer Example (`crates/memory_bus/src/`):**
     - `arbitration.rs`: DMA bus contention and priority arbitration.
     - `map.rs`: Address space decoding, memory banks, and open-bus fallback handlers.
     - `registers.rs`: Custom register shadow snapshotting and read/write routing.
     - `bootstrap.rs`: Gary boot overlay toggle mechanics.

## 3. Rust Source Size Thresholds
- **< 300 lines (Healthy)**: Target baseline for single-aspect modules, hardware registers, and state definitions.
- **300–600 lines (Sweet Spot)**: Ideal size for cohesive units combining types, enums, lookup functions, and operational logic.
- **600–800 lines (Review Trigger)**: Review for multiple responsibilities (Single Responsibility Principle violations), independent sub-domains, or test code that should be extracted into submodules.
- **> 800 lines (Split Mandate)**: Rust source files exceeding 800 lines must be split into a submodule directory (`foo/mod.rs`, `foo/handlers.rs`, etc.) unless qualifying as a Recognized Exception.

## 4. Recognized Exceptions (Permitted to Exceed 800 Lines)
Splitting these files hurts performance, breaks static table locality, and damages readability:
1. **Compile-time static dispatch and lookup tables**: e.g., `dispatch_table.rs` (65,536-entry opcode decoding logic, compile-time tables) and large mathematical LUTs.
2. **Exhaustive linear instruction decoders or atomic hardware circuit state machines**: Sequential execution flows where splitting clock cycle phases across files obscures circuit timing (e.g., `add.rs`, `sub.rs`, `and.rs`, `or.rs`, `cmpi.rs`, `move_b.rs`, `move_w.rs`, `move_l.rs`).

### Two-Way Exception Governance & Prohibition of Silent Mutations
Modifications to `LINE_COUNT_EXCEPTIONS` (in `crates/test_runner/tests/test_architecture_rules.rs` and `tools/harness/audit_code_quality.py`) are strictly governed:
1. **Zero Autonomous Additions:**
   - When a file exceeds 800 lines, the agent is **strictly prohibited from autonomously adding it to `LINE_COUNT_EXCEPTIONS`** to silence CI failures.
   - The agent must decompose the file per Section 5/Section 7, or present the issue to the user with exact metrics and await an explicit user command to grant an exception.
2. **Zero Autonomous Deletions:**
   - When a refactored file drops to $\le 800$ lines, the agent is **strictly prohibited from silently pruning it from `LINE_COUNT_EXCEPTIONS`**.
   - Automated architecture tests (`test_no_stale_line_count_exceptions`) and code quality audits (`[stale_line_count_exception]`) will explicitly fail or warn, alerting the user to decide and authorize the removal.
3. **Continuous Automated Bidirectional Verification:**
   - `test_file_size_limits`: Fails if any unexempted production file exceeds 800 lines.
   - `test_no_stale_line_count_exceptions`: Fails if any file registered in `LINE_COUNT_EXCEPTIONS` has $\le 800$ lines or does not exist on disk.

## 5. When to Split (Architectural Triggers)
Split regardless of line count when:
- The file has multiple reasons to change (e.g. bus arbitration vs address mapping vs configuration).
- Pure serializable state structs are mixed with heavy operational simulation logic.
- In-file unit tests grow beyond ~150–200 lines (move to `tests/*.rs`).
- Sub-features are completely independent (e.g. Paula's audio DACs vs floppy disk controller vs UART).

## 6. Strict Flat Instruction Hierarchy (`crates/cpu/src/instructions/`)
The instruction directory is governed by a **strict flat hierarchy rule**:
1. **Zero Subdirectories in `instructions/`**:
   - Creating subdirectories or multi-file submodules under `crates/cpu/src/instructions/` (such as `instructions/add/`, `instructions/cmpi/`, `instructions/mul/`) is **strictly forbidden**.
   - All instruction files must reside flat directly under `crates/cpu/src/instructions/<mnemonic>.rs`.
2. **Strict 1:1 Mnemonic Alignment**:
   - Each M68000 instruction mnemonic must have its own dedicated `.rs` file directly under `instructions/`.
   - Bundling multiple distinct mnemonics into legacy umbrella files (such as `mul.rs`, `div.rs`, `link_unlk.rs`, `bcd.rs`, `privileged.rs`) is strictly forbidden.
   - For example:
     - `mulu.rs` and `muls.rs` are separate files.
     - `divu.rs` and `divs.rs` are separate files.
     - `link.rs` and `unlk.rs` are separate files.
     - `abcd.rs`, `sbcd.rs`, and `nbcd.rs` are separate files.
     - `trapv.rs`, `rtr.rs`, `rte.rs`, `stop.rs`, `reset.rs`, and `move_usp.rs` are separate files.
3. **Addressing Mode Combinatorics (<800 lines exception)**:
   - When an instruction implementation exceeds 800 lines due to exhaustive addressing mode coverage (e.g., `add.rs`, `sub.rs`, `and.rs`, `or.rs`, `cmpi.rs`), it qualifies as an authorized **Recognized Exception** registered in `LINE_COUNT_EXCEPTIONS` in `test_architecture_rules.rs`. It must **NEVER** be split into a subdirectory.
4. **Justified Architectural Exceptions**:
   - `move_sr_ccr.rs` and `logic_sr_ccr.rs`: Grouping status/condition register operations isolates privilege checks, SR masking, and CCR extraction without causing `ori.rs`, `andi.rs`, `eori.rs` (~789 lines each) or `move_w.rs` (~1,800 lines) to violate line limits.
   - `move_b.rs`, `move_w.rs`, `move_l.rs`: Size-based slices for general `MOVE`.
   - `bcc.rs`, `dbcc.rs`, `scc.rs`: Condition code instruction families.

---

## 7. Execution Skill: `refactor-split-module`

When refactoring files approaching or exceeding 800 lines into submodules, follow the systematic recipe in [`refactor-split-module`](../skills/refactor-split-module/SKILL.md) to extract cohesive units, wire 3-tier re-exports, preserve zero allocation and safe borrowing invariants, and pass automated architecture limits.
