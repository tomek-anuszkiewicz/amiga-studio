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
2. **Atomic hardware circuit state machines**: Sequential execution flows where splitting clock cycle phases across files obscures circuit timing.
3. **Exhaustive linear decoders / pattern matchers**: Addressing mode decoders or exception vectors intended to be audited linearly from top to bottom.

## 5. When to Split (Architectural Triggers)
Split regardless of line count when:
- The file has multiple reasons to change (e.g. bus arbitration vs address mapping vs configuration).
- Pure serializable state structs are mixed with heavy operational simulation logic.
- In-file unit tests grow beyond ~150–200 lines (move to `tests/*.rs`).
- Sub-features are completely independent (e.g. Paula's audio DACs vs floppy disk controller vs UART).
