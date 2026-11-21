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
- Check actual commands, exit statuses, scope and available outputs. Failed,
  missing, interrupted or unrun checks are not passes. Timing reports supply
  measurements, not a substitute verdict. Do not require frozen-tree records,
  report hashes, cached groups or a fixed set of review lanes.
- Check approval for architectural checks/policy, weakened assertions/gates,
  goldens and references. A specific approved contract change can authorize its
  ordinary matching test update; a finding alone cannot.
- Do not claim intermediate commits, assembled behavior, hardware, platforms or
  performance were tested beyond the observations. For selected merge work,
  separately passing source branches do not prove the combined behavior.
- For worktree cleanup, check that useful local outputs are preserved by the
  [artifact tool](../../../../tools/artifacts/README.md) before removal and that
  a failed copy leaves source work intact.

For Graphify changes, assess relevant corpus isolation, ignore-setting and semantic
preservation, query provenance, read-only behavior and actual graph outputs.
Reference production remains an explicitly selected operation; selected task
closure includes the project refresh after successful final master checks.
For PowerShell changes, use the focused
[result criteria](../../TA-write-powershell/references/acceptance.md).

Report concrete findings and evidence limits. Do not start other operations or
require retired coordinator/checkpoint records. Telemetry/archive assessments use
their selected registration boundaries and source integrity checks independently.
