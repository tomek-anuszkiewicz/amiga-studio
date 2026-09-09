# Module Cohesion & Rust Source File Size Guidelines (`.rs` Files Only)

## 1. Scope: Rust Source Code Files (`.rs`) Only
This guideline applies **strictly to Rust source code files (`.rs`)** within workspace crates (`crates/*/src/`).

**Documentation, Design Specifications & References Have NO Line Limits:**
Technical design specifications, architectural blueprints, and hardware reference manuals under `Obsidian/Amiga/Design/` and `Obsidian/Amiga/Reference/` have **no line count limits**. They can and should be as long, comprehensive, and exhaustive as necessary to serve as complete, authoritative single-source-of-truth documents. Do not artificially split or truncate markdown documentation.

## 2. Core Principle: Cohesion Over Fragmentation
Group closely related structs, enums, type definitions, and direct handlers in the same file when they cover the same architectural aspect (e.g. `MemoryBank`, `BankHandler`, and bank handler functions in `map.rs`). Do not fragment tightly coupled domain models across arbitrary micro-files.

## 3. Rust Source Size Thresholds
- **< 300 lines (Healthy)**: Target baseline for single-aspect modules, hardware registers, and state definitions.
- **300–600 lines (Sweet Spot)**: Ideal size for cohesive units combining types, enums, lookup functions, and operational logic.
- **600–800 lines (Review Trigger)**: Review for multiple responsibilities (Single Responsibility Principle violations), independent sub-domains, or test code that should be extracted into submodules.
- **> 800 lines (Split Mandate)**: Rust source files exceeding 800 lines must be split into a submodule directory (`foo/mod.rs`, `foo/handlers.rs`, etc.) unless qualifying as a Recognized Exception.

## 4. Recognized Exceptions (Permitted to Exceed 800 Lines)
Splitting these files hurts performance, breaks static table locality, and damages readability:
1. **Compile-time static dispatch and lookup tables**: e.g., `dispatch_table.rs` (65,536-entry opcode decoding logic, compile-time tables), precalculated BLEP windowed sinc tables, and large mathematical LUTs.
2. **Exhaustive linear instruction decoders or atomic hardware circuit state machines**: Sequential execution flows where splitting clock cycle phases across files obscures circuit timing (e.g., `add.rs`, `sub.rs`, `and.rs`, `or.rs`, `cmpi.rs`, `move_b.rs`, `move_w.rs`, `move_l.rs`).

## 5. When to Split (Architectural Triggers)
Split regardless of line count when:
- The file has multiple reasons to change (e.g. bus arbitration vs address mapping vs configuration).
- Pure serializable state structs are mixed with heavy operational simulation logic.
- In-file unit tests grow beyond ~150–200 lines (move to `tests/*.rs`).
- Sub-features are completely independent (e.g. Paula's audio DACs vs floppy disk controller vs UART).

## 6. Strict Flat Instruction Hierarchy (`crates/m68000/src/instructions/`)
The instruction directory is governed by a **strict flat hierarchy rule**:
1. **Zero Subdirectories in `instructions/`**:
   - Creating subdirectories or multi-file submodules under `crates/m68000/src/instructions/` (such as `instructions/add/`, `instructions/cmpi/`, `instructions/mul/`) is **strictly forbidden**.
   - All instruction files must reside flat directly under `crates/m68000/src/instructions/<mnemonic>.rs`.
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
