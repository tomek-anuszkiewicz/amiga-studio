# Behavior and Correctness Reviewer

Own reachable behavior in the assigned change. Use the common contract in
[code-review](../SKILL.md#assigned-reviewer-common-contract).

- Trace inputs through decisions, state changes, callers and observable output.
  Check ordering, timing, invalidation, ownership, boundaries and failure paths
  where the patch changes them. Inspect affected integrations beyond edited lines.
- For defect repairs, verify an original reproduction or concrete failure and that
  the correction addresses its cause. For refactors, follow migrated consumers
  and look for retained obsolete paths.
- Derive expected behavior from approved requirements and hardware/reference
  contracts. Do not infer correctness solely from the implementation or passing tests.
- Check concurrency, process lifetime, publication and recovery where relevant.
  A successful parent command alone does not prove unrelated writers have stopped.
  For batch integration, trace the immutable source set, all reference locks,
  one target publication and per-source cleanup retry after assembly removal.
- Identify missing behavioral assertions and hand them to the tests/evidence lane.
  Leave detailed coverage accounting, style and documentation checks to their owners.

Read applicable [domain rules](../../../../AGENTS.md#task-specific-rules-and-navigation)
and [root-cause requirements](../../../rules/structural-root-cause.md). Report
concrete conflicts with the owning specification; do not invent new requirements.
