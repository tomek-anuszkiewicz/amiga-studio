# Performance and Readability

Readability principles apply to all project code. Hot-path constraints apply to
emulation execution, memory access, and interrupt paths.

## Readability and Clean Code

- Apply clean-code principles when writing and reviewing code: use names that
  express domain intent, keep functions focused on one coherent responsibility,
  and make control flow easy to follow. Avoid speculative abstractions and
  fragmentation that makes the physical model harder to trace.
- Evaluate condition readability by meaning and context, not operator counts.
  Name a predicate or extract a domain helper when it explains a decision;
  keep a direct condition when it is already clear. Reduce unnecessary nesting
  where doing so preserves the execution model.
- Preserve short-circuit evaluation, evaluation order, side effects, and hardware
  timing. Do not eagerly compute a named boolean when the original branch would
  skip a call or memory access. Preserve these semantics when extracting helpers.
- During implementation and review, identify confusing conditions and explain
  concrete improvements using this rule. Readability is assessed by the agent;
  no operator-count scanner or automatic threshold enforces it.

## Performance and Hardware Traceability

- Allocate no heap memory in hot loops. Use fixed-capacity or in-place state and contiguous arrays instead of pointer-chased dynamic structures.
- Avoid cascaded runtime opcode/size/addressing dispatch; use flat dispatch or concrete specialized handlers. Language-level prohibitions are owned by [rust-best-practices.md](rust-best-practices.md).
- Keep code traceable to registers, signals, and clock phases. Explain non-obvious silicon states with concise hardware rationale, and preserve lazy short-circuit evaluation when naming predicates.
- Choose inlining for actual call paths. Use `inline(always)` for ultra-hot ALU/CCR helpers and CPU register accessors; use `inline` for small public accessors and cross-crate forwarding helpers. Use `inline(never)` for cold exception/trap handlers.
- Do not force inline large control-flow functions (over roughly 15-20 lines) or indirect dispatch-table targets. Attribute choices must preserve the project's architecture checks for ALU, CCR, and cold exception handlers; performance conclusions require measurement.
- Avoid clever optimizations without evidence. Use [profile-external](../skills/profile-external/SKILL.md) for profiling and baseline comparisons, and [audit-code-quality](../skills/audit-code-quality/SKILL.md) for review.

CPU-only micro-step fusion, dual staging, and idle naming belong to [add-m68k-instruction](../skills/add-m68k-instruction/SKILL.md) and the CPU specifications.
