---
name: TA-review-semantic-parity
description: Assess stage-wide subsystem and interaction parity with approved specifications, preserving full static semantic-check scope.
---

# Review the selected stage's semantic parity

Apply [the invocation contract](../../references/workflow-contract.md),
[independent review contract](../../references/review-common.md) and relevant
[documentation criteria](../../references/documentation-criteria.md).
Role: `TA-code-reviewer`; result kind: `review`; lane: `semantic_parity`.
Inputs: actual minor-roadmap/major-milestone stage or explicitly selected subsystem,
exact snapshot, requirements, approval evidence and verification.

1. Establish scope from the assigned stage and its interactions, including unchanged
   affected code. If no scope is identifiable, return `needs_input`; do not choose
   an arbitrary chip. Use Design `tracked_paths` and source definitions to navigate.
2. Run the existing workspace-wide static comparison without narrowing it:
   `python .agents/skills/TA-review-semantic-parity/scripts/audit_semantic_sync.py --semantic-sync`.
   Retain current matching evidence when valid; a script pass is not semantic proof.
3. Assess requirements against code/tests, source-confirmed hardware behavior against
   retained knowledge, and provenance/modeling/established rationale across the
   stage. Do not build an exhaustive function/field catalog.
4. Return inspected subsystems/interactions, static evidence, knowledge gaps,
   unsupported claims, unmet requirements and conflicts. Record scope overlap if
   this broader result also satisfies required batch compliance.

Stop with findings only. Do not enrich documents, repair code, run milestone
verification or declare closure while required gaps remain. Report selected repair
recommendations without dispatching them.

