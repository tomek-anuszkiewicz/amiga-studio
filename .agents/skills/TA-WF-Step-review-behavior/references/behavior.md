# Behavior and correctness

- Trace inputs through decisions, state changes, callers and observable output.
  Inspect changed ordering, timing, invalidation, ownership, boundaries and failure
  paths, including interactions beyond edited lines.
- For defects, assess the original reproduction and mechanism under
  [defect resolution](../../TA-WF-Step-implement-change/references/defect-resolution.md). For refactors,
  follow migrated consumers and remaining obsolete paths.
- Derive expectations from approved requirements and hardware/reference contracts,
  not implementation agreement or passing tests alone.
- Select affected specifications through the shared exact `tracked_paths`
  boundaries and explicit requirement changes. Include unchanged affected callers
  and tests. For selected whole-stage parity, inspect all assigned subsystems and
  interactions and retain the full workspace static comparison. A static pass
  cannot establish semantic agreement; report unmet requirements and conflicts.
- Inspect concurrency, process lifetime, publication and recovery where affected.
  Parent-command success alone does not prove surviving writers stopped. For
  multi-source integration, trace immutable sources, all reference locks, one
  target publication and cleanup retries after assembly-checkout removal.
- Identify missing behavioral assertions for the tests lane. Leave detailed
  coverage, architecture style and document assessment to their owners.
- For result handoffs, trace automatic interpretation, one current option set,
  explicit selection, applicability recheck and dispatch of exactly the selected
  role/skill/scope. Preserve source verdicts and finding IDs across stale inputs,
  missing evidence, failure, interruption and bounded continuation.
- For official recommendations, trace the actual source skill through the matrix
  cell and its evidence condition. Reject unconditional or stale transitions and
  invented readiness. Direct user-selected skills outside the matrix remain valid
  under ordinary scope/authorization; the matrix must not become a dispatch gate.

Read the applicable advance requirements routed by [AGENTS.md](../../../../AGENTS.md).
Report concrete specification conflicts; reviewer preference creates no requirement.
