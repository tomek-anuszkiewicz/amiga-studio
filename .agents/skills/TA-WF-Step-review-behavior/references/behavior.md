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
  Parent-command success alone does not prove surviving writers stopped. For local
  integration, trace the selected branch into `master` under the
  [Git procedure](../../TA-WF-Main-close-task/references/local-integration.md#authorization-and-preparation).
  Check the merge choice against ancestry, any conflict resolutions against approved
  requirements, and preservation of unrelated files and index entries.
- Identify missing behavioral assertions for the tests lane. Leave detailed
  coverage, architecture style and document assessment to their owners.
- For result handoffs, trace the text and JSON from the assigned skill to the main
  session using the [result contract](../../../references/workflow-contract.md).
  Check that both forms describe the same outcome and that the main session retains
  source verdicts, finding IDs and actual evidence. Account for stale inputs,
  missing evidence, failure and interruption when assessing readiness.
- For continuation, check that each action is within the user's selected scope
  and meets the selected procedure's prerequisites and limits. A result or
  recommendation informs the main session. Further operations require user
  selection unless the selected sequence already includes them.

Read the applicable advance requirements routed by [AGENTS.md](../../../../AGENTS.md).
Report concrete specification conflicts; reviewer preference creates no requirement.
