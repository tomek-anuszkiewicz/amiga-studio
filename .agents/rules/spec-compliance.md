# Specification compliance and references

Approved Design specifications govern behavior, timing, memory semantics and
architecture. Read affected requirements before design/editing. Handle concrete
conflicts immediately; ordinary work does not start a broad parity audit.

When a requirement, expected result or reference model conflicts with a specification,
stop that conflicting change and present the rule, contrary evidence and concrete
options. Obtain a recorded user decision before changing/waiving a requirement or
adding behavior outside scope, except the provisional-model case below. A review
finding or code agreement is not such authorization.

## Provisional Register Mutation Parameters

Per-register `delay_cck` and `MutationMode` choices are provisional modeling decisions,
including values adopted from reference emulators. During authorized implementation
or debugging, test results, traces or timing analysis may justify correction without
separate requirement-change approval. Update affected code/tests/design together.
Existing implementation/tests/prose do not make those choices immutable. This
exception never extends to hardware requirements or protected references.

## Compatible Documentation Enrichment

Specifications need not narrate every implementation detail. Choose unspecified
technical details within existing requirements and scope. Apply
[Documentation Value](../references/documentation-criteria.md#documentation-value) to scoped enrichment,
including source-backed hardware knowledge, rather than transcribing code. A missing
description alone is not a conflict. Do not legitimize a contradiction or unauthorized
feature by documenting it. Report-only reviewers identify findings; selected writers
perform compatible enrichment and repairs.

## Specification Changes Before Implementation

Record approved requirement changes in specifications before implementing them;
reuse that approval even when current code differs. Check affected implementation
and tests, including unchanged files, before completion. Editorial changes need no
code change. Resolve authorized differences together; report out-of-scope work
without declaring completion. Never weaken an approved requirement merely to match
existing code.

The [scoped specification review](../skills/TA-review-spec-compliance/SKILL.md)
and [stage-wide semantic comparison](../skills/TA-review-semantic-parity/SKILL.md)
own the formal assessment and its readiness criteria.

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
