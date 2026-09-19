---
trigger: always_on
description: High-performance systems programming, host hardware efficiency, zero runtime heap allocations, and zero user-defined macros/const-generics.
---

# High Performance & Host Hardware Efficiency (with Zero Readability Compromise)

## 1. The Host Hardware Reality (Hardware-Aligned Execution)
Modern host CPUs (x86_64, aarch64) are deeply pipelined superscalar architectures:
- **Branch Predictability:** Avoid unpredictable runtime branches in hot paths. Cascaded dynamic conditionals flush execution pipelines.
- **Contiguous Memory & Locality:** Sequential, flat array structures and compact data layouts outperform pointer chasing and scattered dynamic allocations.
- **Compact Hot Path:** Keep the primary instruction execution loop clean and linear, moving cold error paths out-of-line.

---

## 2. Core Performance Principles

### A. Flatten Execution & Eliminate Cascaded Runtime Branches
- Avoid nested dynamic conditionals in hot paths (e.g. `match opcode { ... match size { ... match ea_mode { ... } } }`).
- **Code may be expansive ("rozległy"):** Favor specialized code generation or dedicated direct handlers (e.g., the 65,536-entry static dispatch table) where addressing mode, register, and operation size are baked in at compile time, eliminating runtime branch evaluation.

### B. Intelligent Inlining & Lean Hot Paths
- **`#[inline(always)]`**: Reserved for ultra-hot arithmetic/logic and CCR flag calculations ($X, N, Z, V, C$) executed on every single clock cycle.
- **`#[inline]`**: For lightweight public getters, forwarding wrappers, and cross-crate helpers so LLVM can optimize across crate boundaries.
- **`#[inline(never)]`**: Mandatory on cold exception paths (Address Error vector 3, Illegal instruction traps, bus fault dumps). Keeping complex recovery logic out-of-line keeps the hot dispatch path linear and prevents code bloat.
- **No Inlining on Large Handlers**: Functions with > 15–20 lines of control flow or targets of indirect function pointers must not be forced inline.


### C. Zero Allocation in Emulation Loop
- Strictly zero dynamic heap allocations (`Vec::new`, `Box::new`, `format!`, `String`) inside `step()`, `step_cck()`, memory access, or interrupt polling.

### D. Fused CCK ALU Micro-Operations & Dual Staging (`addr1`, `addr2`)
- **Fuse ALU into 2-Clock CCK Phases:** Fuse ALU calculations, CCR updates, and Effective Address arithmetic directly into `MicroStep.alu_fn` of natural 2-clock Color Clock phases (`BUS_READ_IDLE`, `BUS_WRITE_IDLE`, prefetch/extension steps) rather than introducing separate zero-clock micro-steps.
- **Dual Staging Architecture:** For dual-memory instructions (`CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`), use dedicated staging registers `state.micro.addr1` and `state.micro.addr2`.
- **Address Error Invariance:** Byte operations may calculate both addresses upfront. Word and long operations **must** defer destination address calculation to the CCK2 idle phase of the source read, ensuring unaligned source reads trigger Address Error with the destination register completely untouched.
- **Direct Staged Writes:** Target memory writes directly via `WRITE_ADDR2_BYTE`, `WRITE_ADDR2_WORD`, `WRITE_ADDR2_PD_LONG_LOW`, and `WRITE_ADDR2_PD_LONG_HIGH`. Never juggle temporary pointers in `scratch[0..2]`, shift `destination >>= 16`, or use pointer-swapping helpers (`set_write_hi`).

---

## 3. The Non-Negotiable Constraint: Readability Without Compromise
High performance must **NEVER** be an excuse for unreadable, cryptic, or spaghetti code:
1. **No Clever Obscurity:** Do not sacrifice clarity for micro-optimizations that LLVM already handles.
2. **Clean Rust Idioms:** Use descriptive types, strong typing, meaningful enum variants, and clear data flow.
3. **Strict Prohibition of User-Defined Macros (`macro_rules!` Forbidden):** Custom macros are strictly forbidden across the codebase. In the era of LLMs, code generation is cheap, eliminating the historical need for macro deduplication. Macros break IDE code navigation (Go to Definition, Find References, Call Hierarchy), obscure call sites, produce confusing compiler diagnostics, and add unnecessary mental complexity. All repetitive code, static dispatch tables, and handlers must be written as explicit, self-documenting Rust functions, direct calls, or standard `const fn` arrays.
4. **Prohibition of Const-Generic Functions with Constant Parameters:** Using generic functions where generic parameters are constants (e.g. `fn op_foo<const S: usize, const M: usize>(...)`) is forbidden for instruction handlers, decoding, and core execution paths. Const-generic combinatorics obscure concrete execution flow, complicate backtraces and interactive debugging, and introduce cognitive overhead. In the era of LLMs, code generation is cheap—write explicit, concrete, specialized functions or direct flattened control flows instead of abstract const-generic templates.
5. **Self-Documenting Code:** Write code that reads like hardware specifications. A developer reading the CPU or Blitter core should immediately understand the circuit intent.
6. **Self-Documenting Boolean Logic & Prohibition of Condition Soup:** Multi-clause compound boolean conditions (`if (a || b) && c && d`) force readers to mentally reverse-engineer circuit states and are strictly prohibited in control flow. Decompose compound conditionals into named local boolean bindings (`let step_has_work = ...; let step_completed = ...;`) or lightweight `#[inline(always)]` domain predicate methods (`step.has_work()`, `self.is_active()`). Control flow statements must read like declarative prose (`if step_has_work && step_completed && sequence_did_not_branch { ... }`). Accompany non-obvious silicon hardware states with concise 1-line comments explaining the *hardware rule*, not just restating the syntax. Clean boolean expressions, collapsible control structures, and non-eager fallback calls are mechanically guarded via `clippy::nonminimal_bool = "warn"`, `clippy::needless_bool = "warn"`, `clippy::collapsible_if = "warn"`, and `clippy::or_fun_call = "warn"`.
7. **Strict Prohibition of Eager Speculative Variable Calculation (Preserve Short-Circuiting):** Explaining variables must **NEVER** eagerly evaluate sub-expressions, method calls, or operations that would otherwise be avoided via boolean short-circuit evaluation (`&&`, `||`) or branched execution (`match`, `if/else`). For example, in `if (self.ipl > self.interrupt_mask() && self.ipl > 0) || self.ipl == 7`, pre-computing `let exceeds_mask = self.ipl > self.interrupt_mask()` unconditionally forces `self.interrupt_mask()` to execute even when `self.ipl == 7` (NMI) or `self.ipl == 0` (no interrupt), degrading hot-path throughput. Similarly, before a `match` dispatch, pre-computing variables unconditionally forces evaluations when the selected condition requires none or only a subset of them. Explaining variables are strictly intended for values that are **already computed**, **unconditionally required**, or extracted **within the local branch** where they are actually consumed.

---

## 4. Code Review Checklist (Performance & Readability)
During `/code-review`, verify:
- [ ] Are cascaded dynamic `match` / `if` checks in hot loops avoided in favor of direct dispatch or flattened handlers?
- [ ] Are cold exception paths (e.g. stack frame creation, traps) annotated with `#[inline(never)]`?
- [ ] Are hot flag/ALU calculations annotated with `#[inline(always)]`?
- [ ] Is the hot path 100% allocation-free?
- [ ] Is the code completely free of custom macros (`macro_rules!`)?
- [ ] Are opcode handlers and execution paths free of const-generic functions (`<const N: ...>`) in favor of concrete specialized functions?
- [ ] Are all bus and internal idle cycles explicitly named with IDLE (`common::BUS_READ_IDLE`, `common::BUS_WRITE_IDLE`, `common::ALU_IDLE*`), with zero anonymous idle structs or legacy finish aliases?
- [ ] Are dual-memory instructions (`CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`) using `addr1` and `addr2` with direct `WRITE_ADDR2_*` writes and zero scratch juggling / `destination >>= 16`?
- [ ] For word and long dual-memory operations, is destination address calculation deferred to CCK2 of the source read to guarantee hardware Address Error invariance?
- [ ] Are ALU and effective address calculations fused onto 2-clock CCK phases (`alu_fn`) rather than using zero-clock dispatch steps?
- [ ] Are compound boolean conditions decomposed into named explaining variables or domain predicate methods, with zero raw condition soup?
- [ ] Do explaining variables preserve boolean short-circuit evaluation with zero eager speculative computation of unused branch values?
- [ ] Is the code clear, well-structured, self-documenting, and free of cryptic tricks?

---

## 5. Authoritative Performance & Benchmarking Specifications & Delegation

When profiling host CPU performance, auditing instruction cycle counts, or evaluating benchmarks, agents must adhere to:
- [`CPU Instruction Benchmarking.md`](../../Obsidian/Amiga/Design/CPU%20Instruction%20Benchmarking.md): Cycle timing models, instruction throughput, and empirical benchmarks.
- [`CPU Instruction Benchmark Catalog.md`](../../Obsidian/Amiga/Design/CPU%20Instruction%20Benchmark%20Catalog.md): Golden instruction cycle counts catalog and validation suites.
- [`CPU Instruction Benchmark Strategies.md`](../../Obsidian/Amiga/Design/CPU%20Instruction%20Benchmark%20Strategies.md): Contention-free benchmark harness strategies and execution modes.
- [`CPU Benchmark Analysis Guide.md`](../../Obsidian/Amiga/Design/CPU%20Benchmark%20Analysis%20Guide.md): Cycle timing discrepancy analysis and triage playbook.
- [`Performance Profiling and Optimization Strategy.md`](../../Obsidian/Amiga/Design/Performance%20Profiling%20and%20Optimization%20Strategy.md): Host CPU execution profiling, cache locality, and zero-allocation runtime metrics.

