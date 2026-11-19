# Execution semantics and performance

Load for execution-path refactoring and requested performance work. Preserve
short-circuit evaluation, evaluation order, side effects and hardware timing when
extracting helpers or naming predicates. Do not eagerly compute an alternative
that the original branch would skip.

Execution, memory-access and interrupt hot paths allocate no heap memory. Use
fixed-capacity or in-place state and contiguous arrays. Avoid cascaded runtime
opcode/size/addressing dispatch; concrete specialized handlers and flat dispatch
must remain traceable to hardware phases. Language prohibitions belong to the
[Rust rule](rust-best-practices.md), and detailed readability/inlining acceptance
belongs to [architecture criteria](../references/architecture-criteria.md).

Use [TA-profile-emulation](../skills/TA-profile-emulation/SKILL.md) for an
investigation and [TA-optimize-emulation](../skills/TA-optimize-emulation/SKILL.md)
for selected optimization changes. The operations share the
[measurement contract](../skills/TA-profile-emulation/references/optimization-evidence.md).
Distinguish host cost from emulated clock counts. Measure before claiming a speedup.

CPU phase and micro-step fusion requirements remain in the
[CPU Micro-Step State Machine](../../Obsidian/Amiga/Design/CPU%20Micro-Step%20State%20Machine.md).
