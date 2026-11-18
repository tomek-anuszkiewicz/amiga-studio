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

Requested investigation uses [TA-profile-emulation](../skills/TA-profile-emulation/SKILL.md).
Selected implementation uses [TA-optimize-emulation](../skills/TA-optimize-emulation/SKILL.md).
Preserve an original baseline before mutation, comparable active/idle measurements,
correctness and a final comparison with that baseline. Distinguish host savings
from emulated clock counts; no speedup claim without measurement. Small recurring
savings are valid candidates even below a profiler display threshold or arbitrary
percentage, but the assignment does not authorize unrelated optimization.

CPU phase and micro-step fusion requirements remain in the
[CPU Micro-Step State Machine](../../Obsidian/Amiga/Design/CPU%20Micro-Step%20State%20Machine.md).
