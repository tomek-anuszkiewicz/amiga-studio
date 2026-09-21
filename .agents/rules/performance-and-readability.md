---
trigger: always_on
description: >
  Zero-allocation hot paths, zero macro_rules!, zero const-generic handlers,
  no cascaded runtime branches, intelligent inlining, and self-documenting
  boolean logic. Code Review checklist and CPU micro-step CCK details live
  in the code-review skill and add-m68k-instruction skill respectively.
---

# High Performance & Readability — Core Invariants

## Hot Path Constraints (All Emulation Crates)
- **Zero heap allocation** in `step()`, `step_cck()`, memory access, interrupt polling:
  no `Vec::new`, `Box::new`, `format!`, `String` inside the execution loop.
- **No cascaded runtime branches** in hot paths — avoid `match opcode { match size { match ea_mode { } } }`.
  Favor flat dispatch or compile-time specialized handlers.
- **Contiguous memory layouts:** flat arrays over pointer-chased dynamic structures.

## Inlining Policy
- `#[inline(always)]` — ultra-hot ALU and CCR flag calculations only.
- `#[inline]` — lightweight public getters and cross-crate forwarding wrappers.
- `#[inline(never)]` — mandatory on cold exception/trap paths to keep the hot dispatch linear.
- No forced inlining on functions > 15–20 lines of control flow.

## Strict Code Prohibitions (Compiler-Enforced via Clippy)
- **Zero `macro_rules!`** across the entire workspace. Write explicit specialized functions.
- **Zero const-generic instruction handlers** (`fn op<const S: usize>()`). Write concrete functions.
- **No condition soup:** decompose multi-clause `&&`/`||` chains into named explaining variables
  or domain predicate methods (`self.is_active()`, `step.has_work()`).
- **No eager speculative variable computation:** explaining variables must not force evaluation
  of sub-expressions that boolean short-circuit (`&&`, `||`) would otherwise skip.

## Readability Non-Negotiable
- Code reads like a hardware specification — name the register, the clock phase, the signal.
- No clever micro-optimizations that LLVM already handles.
- Accompany non-obvious silicon states with a 1-line comment explaining the *hardware rule*.

## CPU Micro-Step Details (M68000 — Closed Gate)
CCK phase fusion, dual staging (`addr1`/`addr2`), `WRITE_ADDR2_*` write patterns, Address Error
invariance, and IDLE constant naming apply exclusively to `crates/cpu/`. Consult the
[`add-m68k-instruction`](../skills/add-m68k-instruction/SKILL.md) skill when working there.

## Code Review Checklist
Full performance & readability audit checklist → [`code-review`](../skills/code-review/SKILL.md) skill.
Benchmarking specs → `Obsidian/Amiga/Design/CPU Instruction Benchmarking.md`.
