---
name: code-review
description: >-
  Conduct an independent, uncompromising code, architecture, and specification compliance review for the Amiga 500 emulator. Audits git diffs, file sizes, inlining, zero-panics, endianness, design doc pruning, roadmap status, and runs automated architecture tests.
---

# Code & Specification Compliance Reviewer

This skill guides an agent (or subagent reviewer) through performing a strict post-implementation audit against [AGENTS.md](../../../AGENTS.md) and repository design guidelines.

---

## When to Activate

- Upon completing a roadmap milestone or substantial feature PR.
- When performing a double-check / review requested by the user.
- As a subagent reviewer tasked with auditing code diffs.

---

## Review Procedure

### Step 1: Automated Architecture Verification
Run the automated architectural test suite in `test_runner`:
```powershell
cargo test -p test_runner --test test_architecture_rules
```
If any test fails, review is immediately blocked until the violation is resolved.

### Step 2: Diff Inspection (`git diff`)
Analyze all modified and added files using `git diff`:
1. **Zero Panics**: Search for `.unwrap()` or `.expect(` in `crates/*/src/`. Emulated guest execution must never crash or panic the host.
2. **Endianness**: Verify Big-Endian multi-byte decoding (`u16::from_be_bytes`, `u32::from_be_bytes`).
3. **Wrapping Math**: Check ALU operations and cycle additions for wrapping arithmetic (`wrapping_add`, `wrapping_sub`).
4. **File Sizes**: Ensure no file exceeds 800 lines (excluding `dispatch_table.rs`).
5. **Host CPU Performance & Readability**:
   - Verify branch-minimization: hot loops favor flattened dispatch instead of nested `match`/`if` cascades ("code may be expansive").
   - Verify zero-allocations in hot execution paths (no `Vec`, `Box`, `format!`, dynamic boxed iterators).
   - Endianness Bypass: bitwise operations (`AND`, `OR`, `EOR`, `NOT`, `CLR`) and block copies avoid redundant byte swapping in hot loops.
   - **Readability, Macro & Const-Generic Prohibition**: Confirm code is clean, idiomatic Rust, self-documenting, strictly free of user-defined macros (`macro_rules!`), and free of const-generic handler functions (`<const N: ...>`). In the era of LLMs, code generation is cheap; macros and const-generic matrices break code navigation and add unnecessary cognitive complexity.
6. **Inlining Rules**:
   - Small accessors & cross-crate helpers: `#[inline]`.
   - CCR condition code flags & bit calculations: `#[inline(always)]`.
   - Cold exception handlers & diagnostic paths: `#[inline(never)]`.
   - Large functions (>15–20 lines) and dispatch targets: no inlining.
7. **WASM Portability & Host Isolation**:
   - Core emulation crates (`m68000`, `memory_bus`, `config`, `rtc`) must contain zero OS-specific calls (`no std::time::Instant`, `no std::thread`, `no std::fs`).
   - All I/O operates on external decoupled byte buffers (`&[u8]`).
8. **Decoupled State & Save States**:
   - State structs (`CpuState`, `MemoryBus`, chip states) derive `Serialize` & `Deserialize`.
   - Zero circular handles (`Rc<RefCell<...>>`) between peer subsystems.
9. **Specification Integrity (Anti-Hack Rule)**:
   - Zero silent deviations from hardware specs or ad-hoc special-casing just to force synthetic tests green.
10. **Path Privacy**: Ensure no host paths (`C:\Users\`, `/home/`, personal disk paths) are present.

### Step 3: Living Documentation Audit
1. **Design Documents**: Did the author update `Obsidian/Amiga/Design/`? Were speculative draft snippets or pre-implementation code sketches removed?
2. **Roadmap**: If a step in `ROADMAP.md` is 100% complete, was it removed from the active list and added to the baseline summary?
3. **Crate Graph**: Were crate dependencies in `General Architecture.md` updated if `Cargo.toml` was touched?

### Step 4: Issue Audit Report
Provide the audit report using the following standard template:

```markdown
### 🛡️ Code & Architecture Compliance Review:
- [ ] **Architecture Test Suite:** `cargo test -p test_runner --test test_architecture_rules` passed.
- [ ] **Zero Panics & Endianness:** No `.unwrap()` in runtime, explicit Big-Endian conversion & wrapping math.
- [ ] **Host CPU Performance & Sympathy:** Flattened dispatch (branch-minimization), zero heap allocations in hot path, endianness bypass on bitwise ops.
- [ ] **Readability, Zero Macros & No Const-Generic Handlers:** Clean idiomatic Rust, zero cryptic hacks, zero custom macros (`macro_rules!`), and concrete handlers without const generics.
- [ ] **WASM Core Purity:** Zero OS calls (`Instant`, `thread`, `fs`) in core emulation crates.
- [ ] **Decoupled SaveState:** Subsystem state derives `Serialize`/`Deserialize`, zero circular references (`Rc<RefCell>`).
- [ ] **Anti-Hack & Spec Integrity:** Zero ad-hoc test workarounds; hardware specifications followed strictly.
- [ ] **File Size & Cohesion:** All files <= 800 lines (or recognized exception).
- [ ] **Inlining Strategy:** Cross-crate `#[inline]`, CCR `#[inline(always)]`, cold paths `#[inline(never)]`.
- [ ] **Design Docs Pruning:** Living docs updated, speculative code pruned.
- [ ] **Roadmap Discipline:** Completed steps removed from active roadmap and summarized.
- [ ] **Path Privacy:** Zero external host paths.
- [ ] **Test Coverage:** All workspace tests pass 100% green (`cargo test`).

**Verdict:** [APPROVED | CHANGES REQUESTED]
**Observations / Required Actions:** (if any)
```
