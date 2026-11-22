# Behavior and correctness

- Trace inputs through decisions, state changes, callers and observable output.
  Inspect changed ordering, timing, invalidation, ownership, boundaries and failure
  paths, including interactions beyond edited lines.
- For defects, assess the original reproduction and mechanism under
  [defect resolution](../../TA-WF-Step-implement-change/references/defect-resolution.md). For refactors,
  follow migrated consumers and remaining obsolete paths.
- Derive expectations from approved requirements and hardware and reference
  contracts. Apply those expectations even when the implementation and its tests
  agree and the tests pass.
- Select affected specifications through the shared exact `tracked_paths`
  boundaries and explicit requirement changes. Include unchanged affected callers
  and tests. For selected whole-stage parity, inspect all assigned subsystems and
  interactions and retain the full workspace static comparison. Use the
  behavior assessment to establish whether the candidate meets the
  requirements, even if the static comparison passes. Report unmet
  requirements and conflicts.
- Inspect concurrency, process lifetime, publication and recovery where affected.
  Processes can survive a successful parent command and continue writing. For
  local integration, trace the selected branch
  into `master` under the
  [Git procedure](../../TA-WF-Main-close-task/references/local-integration.md#authorization-and-preparation).
  Check that the merge method fits the branch ancestry and that any conflict
  resolutions meet approved requirements. Check that the merge preserves
  unrelated files and index entries.
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

Read the requirements that [AGENTS.md](../../../../AGENTS.md) directs you to before
the assessment. Report concrete conflicts with existing specifications.
Reviewer preference creates no requirement.
