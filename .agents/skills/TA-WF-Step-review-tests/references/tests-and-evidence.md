# Tests and observed verification

Apply the shared [testing criteria](../../../references/testing-criteria.md),
[verification change authorization](../../../references/protected-verification.md#protected-verification)
and [command reference](../../../../tools/verification/README.md).

- Map selected behavior and interactions to independently expected assertions.
  Executed lines and percentages alone do not establish meaningful protection.
  Distinguish fixtures, production integration and hardware observations.
- Inspect supplied coverage and behavioral gaps within the requested scope.
  Distinguish measured, unmeasured and pre-existing gaps. A missing measurement
  does not authorize executing it during a report-only review.
- Check actual commands, exit statuses, scope and available outputs. Record
  failed, missing, interrupted and unrun checks according to their actual
  status. Use timing reports as measurements and assess readiness from the
  applicable verification evidence. Do not require frozen-tree records, report
  hashes, cached groups or a fixed set of review lanes.
- Check approval for changes to architecture checks or policy, weakened
  assertions or gates, and changes to goldens and references. A specific
  approved contract change can authorize the ordinary test update that follows
  from it. A finding alone gives no such authorization.
- Do not claim intermediate commits, assembled behavior, hardware, platforms or
  performance were tested beyond the observations. For selected merge work,
  separately passing source branches do not prove the combined behavior.
- For worktree cleanup, check that the
  [artifact tool](../../../../tools/artifacts/README.md) preserves useful local
  outputs before the worktree is removed. Check that a failed copy leaves the
  source work intact.

For Graphify changes, assess whether the relevant corpora stay isolated,
whether ignore settings and semantics are preserved, and where query results
come from. Assess read-only behavior and actual graph outputs. Production of
reference graphs requires explicit selection. Selected task closure includes
refreshing the project graph after successful final master checks.
For PowerShell changes, use the focused
[result criteria](../../TA-write-powershell/references/acceptance.md).

Report concrete findings and evidence limits. Do not start other operations or
require retired coordinator/checkpoint records. Telemetry/archive assessments use
their selected registration boundaries and source integrity checks independently.
