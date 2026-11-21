# Verification change authorization

## Protected Verification

Architectural tests, checker implementations and policy data require explicit user
approval for every addition, modification or deletion, even stricter checks and
mechanical refactors. This includes dependency permissions, limits, exceptions and
enforcement scope wherever stored. A generic implementation/refactor/test-fix/commit
request does not grant it. Present the current constraint, concrete diff, rationale
and effect on accepted code, then obtain approval before applying it. Satisfy the
existing constraint or obtain a decision; manifests, documentation, passing checks
and another agent's approval do not substitute.

Weakening any other test or required gate also needs that approval: expectations,
tolerances, masks, exceptions, removed/skipped assertions/cases and reduced scope.
Present evidence and lost protection; suspected obsolescence is not permission.
Ordinary maintenance preserving acceptance/coverage remains allowed, including
imports, internal-refactor fixtures, meaningful new tests and stronger assertions.
A specific approved contract change can authorize its matching ordinary test update.

## Reference Artifact Integrity

Reference traces, CSV baselines, cycle totals and SingleStep vectors are protected.
Never change them to make a failure pass. A justified correction requires root-cause
evidence, a proposed change and explicit approval; hardware-derived references
additionally need precise hardware evidence. For relevant review consult
[platform invariants](../../Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md)
and validated test evidence.

Harness normalization/comparison masks need precise evidence and a condition limited
to the demonstrated case, such as the exact exception frame and fields. Do not
broaden exceptions to obtain a pass. Report existing masks as verification limits;
narrowing them still follows defect/reference-conflict procedures.
