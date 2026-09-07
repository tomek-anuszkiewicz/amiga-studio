---
name: code-review
description: Comprehensive architecture, rules compliance, and code quality audit for Amiga 500 emulator tasks and milestones
---

# Workflow: Code & Specification Compliance Review

Use this workflow to conduct an independent, rigorous audit of changes before completing a task or milestone.

---

## 1. Automated Architecture Rule Verification
Execute the automated architectural test suite:
```powershell
cargo test -p test_runner --test test_architecture_rules
```
Ensure all 4 rules pass:
- File size limit `<= 800` lines (excluding `dispatch_table.rs`).
- Zero `.unwrap()` / `.expect()` calls in core emulation crates.
- Zero hardcoded external user/host paths.
- Zero stale `(Completed)` headings in `ROADMAP.md` active strategy.

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

### B. Inlining Strategy
- [ ] `#[inline]` on public accessors, single-expression helpers, forwarding wrappers, and cross-crate conversions.
- [ ] `#[inline(always)]` strictly reserved for ultra-hot CCR condition code flags and inner arithmetic.
- [ ] No `#[inline]` on functions > 15–20 lines of complex control flow or dispatch function pointers.
- [ ] `#[inline(never)]` on cold exception/trap vectors.

### C. Workspace & Architecture
- [ ] Respects 3-tier re-export strategy (Tier 1: config, Tier 2: peers memory_bus/m68000, Tier 3: sub-components rtc/copper/blitter).
- [ ] Zero circular references.

### D. Documentation & Roadmap
- [ ] Corresponding design doc under `Obsidian/Amiga/Design/` updated.
- [ ] Design document pruned of pre-implementation speculative code or draft sketches.
- [ ] Completed roadmap steps pruned from `ROADMAP.md` and summarized in the baseline section.
- [ ] Crate dependency Mermaid graph updated in `General Architecture.md` if `Cargo.toml` dependencies changed.

---

## 3. Full Test Suite Verification
Run all workspace unit and integration tests:
```powershell
cargo test
```

---

## 4. Audit Verdict
Deliver a structured audit report:
- **Verdict**: `APPROVED` or `CHANGES REQUESTED`
- **Checklist Summary**: Checked items from the Definition of Done.
- **Action Items**: Concrete file and line references if any rule is violated.
