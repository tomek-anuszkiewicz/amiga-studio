# Specification compliance guidance

The shared [specification authority](../../AGENTS.md#specification-authority)
defines which requirements take precedence and how to resolve conflicts. Apply
this guidance to selected code changes, documentation updates and reviews. If
the assignment calls only for an assessment, report conflicts and leave the
requirements, code and reference data as they are.

## Unspecified Technical Details

Choose unspecified technical details within existing requirements and scope.
A missing description alone is not a specification conflict.

## Specification Changes Before Implementation

Record the approved change in the specification before implementing it. The
approval applies even if the current code differs from the approved requirement.
Resolve authorized differences together. If reconciling the differences requires
work outside the authorized scope, report that work as outstanding and do not
declare the reconciliation complete. Never weaken an approved requirement merely
to match existing code.

For provisional `delay_cck` and `MutationMode` choices, use the
[register mutation modeling guidance](register-mutation-modeling.md#provisional-register-mutation-parameters).
Reference traces, baselines, vectors and comparison masks follow
[reference artifact integrity](protected-verification.md#reference-artifact-integrity).
