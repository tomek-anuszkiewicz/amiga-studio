# Performance and Readability

Readability principles apply to all project code. Hot-path constraints apply to
emulation execution, memory access, and interrupt paths.

## Refactoring Execution Logic

Preserve short-circuit evaluation, evaluation order, side effects, and hardware
timing. Do not eagerly compute a named boolean when the original branch would
skip a call or memory access. Preserve these semantics when extracting helpers.

## Performance and Hardware Traceability

- Allocate no heap memory in hot loops. Use fixed-capacity or in-place state and contiguous arrays instead of pointer-chased dynamic structures.
- Avoid cascaded runtime opcode/size/addressing dispatch; use flat dispatch or concrete specialized handlers. Language-level prohibitions are owned by [rust-best-practices.md](rust-best-practices.md).
- Keep code traceable to registers, signals, and clock phases. Explain non-obvious silicon states with concise hardware rationale, and preserve lazy short-circuit evaluation when naming predicates.
- Choose inlining for actual call paths. Use `inline(always)` for ultra-hot ALU/CCR helpers and CPU register accessors; use `inline` for small public accessors and cross-crate forwarding helpers. Use `inline(never)` for cold exception/trap handlers.
- Do not force inline large control-flow functions (over roughly 15-20 lines) or indirect dispatch-table targets. Attribute choices must preserve the project's architecture checks for ALU, CCR, and cold exception handlers; performance conclusions require measurement.
- Avoid clever optimizations without evidence. Use comparable benchmark measurements for performance claims and [audit-code-quality](../skills/audit-code-quality/SKILL.md) for review.

CPU execution phases and micro-step fusion follow the [CPU Micro-Step State Machine](../../Obsidian/Amiga/Design/CPU%20Micro-Step%20State%20Machine.md) specification.
