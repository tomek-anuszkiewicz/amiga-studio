---
name: code-review
description: Comprehensive architecture, rules compliance, and code quality audit for Amiga 500 emulator tasks and milestones
---

# Workflow: Code & Specification Compliance Review

Use this workflow to conduct an independent, rigorous audit of changes before completing a task or milestone.

---

## 1. Automated Architecture & Formatting Verification
Execute formatting check and the automated architectural test suite:
```powershell
cargo fmt --all -- --check
cargo test -p test_runner --test test_architecture_rules
```
Ensure all rules pass:
- Code is 100% formatted via standard `cargo fmt`.
- Rust source file size limit `<= 800` lines in `crates/*/src/` (excluding recognized static tables/exceptions). Documentation files have NO line limits.
- Strict flat instruction hierarchy: zero subdirectories in `crates/m68000/src/instructions/` (all instructions are single flat `<mnemonic>.rs` files), strict 1:1 mnemonic alignment, zero umbrella multi-instruction files (no `mul.rs`, `div.rs`, `bcd.rs`, `link_unlk.rs`, `privileged.rs`).
- Zero `.unwrap()` / `.expect()` calls in core emulation crates.
- Zero custom macros (`macro_rules!`) and zero const-generic handlers.
- Zero hardcoded external user/host paths.

---

## 2. Git Diff Inspection
Inspect the unstaged or branch diff:
```powershell
git diff
```
Audit the diff against the guidelines in `AGENTS.md`:

### A. Endianness & Systems Safety
- [ ] No host-endian pointer casting or `transmute` on guest memory.
- [ ] Explicit Big-Endian conversion (`from_be_bytes`, `to_be_bytes`).
- [ ] Explicit wrapping arithmetic (`wrapping_add`, `wrapping_sub`) on ALU and cycle operations.
- [ ] Emulated guest faults do not panic the host process.

### B. Host CPU Mechanical Sympathy & Readability
- [ ] Branch minimization: hot loops favor flattened, direct dispatch over deep nested `match`/`if` trees ("code may be expansive").
- [ ] Zero allocations: no `Vec`, `Box`, `String`, or `format!` in `step()`, `step_cck()`, or memory paths.
- [ ] Endianness Bypass: bitwise operations (`AND`, `OR`, `EOR`, `NOT`, `CLR`) avoid redundant byte swapping in hot loops.
- [ ] **Readability, Zero Macros & No Const-Generic Handlers**: code is clean, idiomatic Rust, self-documenting, completely free of custom macros (`macro_rules!`), and free of const-generic handler matrices (`<const N: ...>`) in favor of concrete specialized functions.

### C. Inlining Strategy
- [ ] `#[inline]` on public accessors, single-expression helpers, forwarding wrappers, and cross-crate conversions.
- [ ] `#[inline(always)]` strictly reserved for ultra-hot CCR condition code flags and inner arithmetic.
- [ ] No `#[inline]` on functions > 15–20 lines of complex control flow or dispatch function pointers.
- [ ] `#[inline(never)]` on cold exception/trap vectors and error dumps (keeping L1i cache dense).

### D. Workspace & Architecture
- [ ] Respects 3-tier re-export strategy (Tier 1: config, Tier 2: peers memory_bus/m68000, Tier 3: sub-components rtc/copper/blitter).
- [ ] Strict flat instruction hierarchy: zero subdirectories in `crates/m68000/src/instructions/` (all instructions are single `<mnemonic>.rs` files, e.g. `mulu.rs`/`muls.rs`, `divu.rs`/`divs.rs`, `link.rs`/`unlk.rs`, `abcd.rs`/`sbcd.rs`/`nbcd.rs`, `trapv.rs`/`rtr.rs`/`rte.rs`/`stop.rs`/`reset.rs`/`move_usp.rs`; zero umbrella files).
- [ ] **Idle Micro-Step Naming & Common Primitives**: all bus and internal idle phases explicitly feature IDLE (`common::BUS_READ_IDLE`, `common::BUS_WRITE_IDLE`, `common::ALU_IDLE*`); zero anonymous idle structs (`MicroStep { bus_fn: None, alu_fn: None, ... }`) or legacy aliases.
- [ ] Zero circular references between peer subsystems (`no Rc<RefCell>`).
- [ ] **WASM Portability**: zero OS calls (`std::time::Instant`, `std::thread`, `std::fs`) in core emulation crates.
- [ ] **Decoupled SaveState**: all subsystem state structs implement `serde::Serialize` and `Deserialize`.

### E. Specification Compliance & Anti-Hack Rule
- [ ] Zero silent deviations from hardware specifications.
- [ ] Zero ad-hoc test-specific hacks to pass synthetic vectors without user escalation.
- [ ] **Anti-Tamper & Golden Hash Invariance**: Zero blind updates to golden master hashes (`GOLDEN_*_HASH`), reference cycle counts, or test fixtures to silence failing tests.

### F. Language Policy & English Purity
- [ ] **Strict English Purity in Diff**: Verify that `git diff` introduces ZERO non-English words, identifiers, or prompt echoes in source code, docstrings, and inline comments (per `language-policy.md`). All terms from Polish user prompts must be fully translated into idiomatic English before coding. Quoting Polish prompt phrases in code comments (even in quotation marks) is strictly prohibited.

### G. Documentation, Diary & Roadmap
- [ ] Corresponding design doc under `Obsidian/Amiga/Design/` updated.
- [ ] Design document pruned of pre-implementation speculative code, draft sketches, and duplicate code snippets of already-written code (the codebase is the single source of truth; design docs must not duplicate implemented code).
- [ ] Completed roadmap steps pruned from `ROADMAP.md` and summarized in the baseline section.
- [ ] **Engineering Diary Updated (`DIARY.md`)**: Detailed entry added to `DIARY.md` (Section 10) recording what was actually changed, why, and architectural decisions (preserving granular history beyond squashed/merged git commits).
- [ ] Crate dependency Mermaid graph updated in `General Architecture.md` if `Cargo.toml` dependencies changed.

### H. Defect Retrospection & Institutional Prevention (If Bug Fix / Refactor)
- [ ] Root cause identified and documented ("Why did this happen?").
- [ ] Dedicated regression test(s) added covering the exact failure mode and adjacent edge cases.
- [ ] Institutional prevention evaluated: architectural rule, lint, design doc, or DoD checklist updated to ensure this class of defect never recurs.

### H. Comprehensive Unit Test Coverage
- [ ] **Every New / Modified File with Testable Logic Has Dedicated Unit Tests:** Verify that any file containing state machines, hardware models, math/ALU operations, algorithms, statistics, parsers, or program builders has dedicated unit tests (`tests/<module_name>.rs` or inline `#[cfg(test)] mod tests`).
- [ ] **Edge Cases & Boundary Coverage:** Tests verify happy paths, zero/empty states, boundary conditions, and invalid inputs.
- [ ] **No Flaky Tests:** Tests execute deterministically without sleeps, wall-clock timing races, or host CPU load dependencies.

---

## 3. Full Test Suite Verification
Run all workspace unit and integration tests:
```powershell
cargo test
```

### M68000 Full Exhaustive Verification
If any file in `crates/m68000` was added, modified, or refactored:
```powershell
$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep
```
Verify that all 77 suites run across all ~300,000 test cases with 100% green passes.

---

## 4. Audit Verdict
Deliver a structured audit report:
- **Verdict**: `APPROVED` or `CHANGES REQUESTED`
- **Checklist Summary**: Checked items from the Definition of Done.
  - [ ] **Code Formatting & Architecture Tests:** `cargo fmt` and `test_architecture_rules` 100% clean.
  - [ ] **Unit Test Coverage:** Every module with testable logic has dedicated unit tests (`tests/<module>.rs` or inline).
  - [ ] **Zero Panics & Endianness:** No `.unwrap()` in runtime, explicit Big-Endian conversion & wrapping math.
  - [ ] **Host CPU Mechanical Sympathy:** Flattened dispatch, zero allocations in hot paths, inlining compliance.
  - [ ] **Readability, No Macros & No Const Generics:** Explicit code, zero `macro_rules!`, zero const-generic handlers.
  - [ ] **Language Policy Purity:** Zero non-English words or prompt echoes in source code, docstrings, or comments.
  - [ ] **Living Docs, Diary & Roadmap:** Pruned obsolete code, removed implemented code snippets, updated `DIARY.md` changelog, updated roadmap.
- **Action Items**: Concrete file and line references if any rule is violated.

