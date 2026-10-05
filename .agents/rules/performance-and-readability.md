# Performance and Readability

Applies to emulation execution, memory access, and interrupt paths.

- Allocate no heap memory in hot loops. Use fixed-capacity or in-place state and contiguous arrays instead of pointer-chased dynamic structures.
- Avoid cascaded runtime opcode/size/addressing dispatch; use flat dispatch or concrete specialized handlers. Language-level prohibitions are owned by [rust-best-practices.md](rust-best-practices.md).
- Keep code traceable to registers, signals, and clock phases. Explain non-obvious silicon states with concise hardware rationale, and preserve lazy short-circuit evaluation when naming predicates.
- Use `inline(always)` for ultra-hot ALU/CCR helpers and CPU register accessors; use `inline` for small public accessors and cross-crate forwarding helpers. Use `inline(never)` for cold exception/trap handlers.
- Do not force inline large control-flow functions (over roughly 15-20 lines) or indirect dispatch-table targets. Attribute choices must preserve the project's architecture checks; performance conclusions require measurement.
- Avoid clever optimizations without evidence. Use [profile-external](../skills/profile-external/SKILL.md) for profiling and baseline comparisons, and [audit-code-quality](../skills/audit-code-quality/SKILL.md) for review.

CPU-only micro-step fusion, dual staging, and idle naming belong to [add-m68k-instruction](../skills/add-m68k-instruction/SKILL.md) and the CPU specifications.
