# Specification compliance guidance

The shared [specification authority](../../AGENTS.md#specification-authority)
owns requirement precedence and conflict decisions. Apply this guidance to selected
code changes, documentation updates and reviews. A report-only assessment reports
conflicts without editing requirements, code or reference data.

## Unspecified Technical Details

Choose unspecified technical details within existing requirements and scope.
A missing description alone is not a specification conflict.

## Specification Changes Before Implementation

Record approved requirement changes in specifications before implementing them;
reuse that approval even when current code differs.
Resolve authorized differences together; report out-of-scope work
without declaring completion. Never weaken an approved requirement merely to match
existing code.

For provisional `delay_cck` and `MutationMode` choices, use the
[register mutation modeling guidance](register-mutation-modeling.md#provisional-register-mutation-parameters).
Reference traces, baselines, vectors and comparison masks follow
[reference artifact integrity](protected-verification.md#reference-artifact-integrity).
