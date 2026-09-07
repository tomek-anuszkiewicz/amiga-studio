# High Performance & Host CPU Mechanical Sympathy (with Zero Readability Compromise)

## 1. The Host Hardware Reality (Mechanical Sympathy)
Modern host CPUs (x86_64, aarch64) are deeply pipelined (14–20+ execution stages) superscalar architectures:
- **Branch Misprediction Penalty:** A single mispredicted branch (`if/else`, dynamic `match`) flushes the pipeline and wastes 15–20 CPU cycles.
- **Instruction Cache (L1i):** Typically 32 KB to 64 KB. Keeping the hot instruction loop compact and cache-dense is essential for sustained 60 FPS / cycle-exact emulation.
- **Data Cache (L1d):** Sequential, contiguous memory and flat arrays outperform pointer-chasing and scattered dynamic allocations.

---

## 2. Core Performance Principles

### A. Flatten Execution & Eliminate Cascaded Runtime Branches
- Avoid nested dynamic conditionals in hot paths (e.g. `match opcode { ... match size { ... match ea_mode { ... } } }`).
- **Code may be expansive ("rozległy"):** Favor specialized code generation or dedicated direct handlers (e.g., the 65,536-entry static dispatch table) where addressing mode, register, and operation size are baked in at compile time, eliminating runtime branch evaluation.

### B. Intelligent Inlining & Cache-Dense Hot Paths
- **`#[inline(always)]`**: Reserved for ultra-hot arithmetic/logic and CCR flag calculations ($X, N, Z, V, C$) executed on every single clock cycle.
- **`#[inline]`**: For lightweight public getters, forwarding wrappers, and cross-crate helpers so LLVM can optimize across crate boundaries.
- **`#[inline(never)]`**: Mandatory on cold exception paths (Address Error vector 3, Illegal instruction traps, bus fault dumps). Keeping complex recovery logic out-of-line keeps the hot dispatch loop contiguous and resident in L1i cache.
- **No Inlining on Large Handlers**: Functions with > 15–20 lines of control flow or targets of indirect function pointers must not be forced inline.

### C. Zero Allocation in Emulation Loop
- Strictly zero dynamic heap allocations (`Vec::new`, `Box::new`, `format!`, `String`) inside `step()`, `step_cck()`, memory access, or interrupt polling.

---

## 3. The Non-Negotiable Constraint: Readability Without Compromise
High performance must **NEVER** be an excuse for unreadable, cryptic, or spaghetti code:
1. **No Clever Obscurity:** Do not sacrifice clarity for micro-optimizations that LLVM already handles.
2. **Clean Rust Idioms:** Use descriptive types, strong typing, meaningful enum variants, and clear data flow.
3. **No Convoluted Macros:** Macros are allowed only for repetitive boilerplate generation (e.g., static dispatch table wiring). Never hide core architectural logic inside unreadable macro mazes.
4. **Self-Documenting Code:** Write code that reads like hardware specifications. A developer reading the CPU or Blitter core should immediately understand the circuit intent.

---

## 4. Code Review Checklist (Performance & Readability)
During `/code-review`, verify:
- [ ] Are cascaded dynamic `match` / `if` checks in hot loops avoided in favor of direct dispatch or flattened handlers?
- [ ] Are cold exception paths (e.g. stack frame creation, traps) annotated with `#[inline(never)]`?
- [ ] Are hot flag/ALU calculations annotated with `#[inline(always)]`?
- [ ] Is the hot path 100% allocation-free?
- [ ] Is the code clear, well-structured, self-documenting, and free of cryptic tricks?
