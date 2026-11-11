# Specification Compliance and Reference Artifacts

Design specifications and [AGENTS.md](../../AGENTS.md) govern implementation. Never silently contradict or bypass documented behavior, timing, memory semantics, or architectural boundaries.

Read the relevant requirements before changing code and follow them throughout implementation. Handle concrete conflicts when encountered; do not defer them to a commit or milestone. This does not trigger a general code/design parity audit; scoped compliance review belongs to the authorized pre-commit workflow, and broader semantic parity belongs to milestone closure or an explicit request.

- When a requirement, test expectation, or reference emulator conflicts with the specification, stop the conflicting change and present the current rule, contrary evidence, and concrete options. Provisional register mutation parameters follow the [exception below](#provisional-register-mutation-parameters).
- Obtain an explicit recorded user decision before changing or waiving an existing requirement or introducing behavior outside the authorized scope. Otherwise implement the documented behavior. Compatible documentation enrichment follows the rule below and does not require separate approval.

## Provisional Register Mutation Parameters

Per-register `delay_cck` values and `MutationMode` choices are provisional modeling
decisions, including those adopted from reference emulators. They may be revised
during authorized implementation or debugging when test results, execution traces,
or timing analysis justify a correction. Existing code, documentation, and
implementation-level tests do not make these choices immutable; revising them does
not require separate requirement-change approval. Update affected code, tests,
and design descriptions together. Hardware requirements and protected reference
artifacts remain subject to the existing compliance rules.

## Compatible Documentation Enrichment

The specification governs implementation. It need not describe every implementation
detail before coding begins. Choose unspecified technical details autonomously
within the authorized scope and existing requirements.

- Add significant implementation details that explain behavior, timing, state
  transitions, subsystem interactions or constraints when they agree with the
  specification and the authorized change. Omission alone is not a conflict.
- Describe mechanisms needed to understand or verify the system; do not require
  documentation for every helper, field or local algorithm. Follow the scope and
  duplication rules in [docs-maintenance.md](docs-maintenance.md#living-design-specifications).
- Existing code is evidence of implementation, not authority to change a
  requirement. Do not legitimize a contradiction or an unauthorized feature by
  documenting it. Present such cases under the conflict procedure above.

## Specification Changes Before Implementation

Record user-approved requirement changes in the specification before implementing
the changed behavior. That approval authorizes the requirement change; do not ask
for the same approval again merely because existing code behaves differently.

- When editing requirements, check the affected implementation and tests before
  declaring the task complete. Editorial changes do not require code changes.
- Resolve known differences in code and relevant tests as part of the same
  change. If implementation work exceeds the authorized scope, report the
  difference and obtain direction before expanding it; leave completion blocked.
- Before committing, use [verify-spec-compliance](../skills/verify-spec-compliance/SKILL.md)
  to verify agreement in the intended snapshot, including changed requirements
  whose implementation files have no diff. Block the commit while differences
  remain or the affected implementation has not been verified. Do not revert or
  weaken an approved requirement merely to match existing code.

## Reference Artifact Integrity

- Reference traces, CSV baselines, cycle totals, and SingleStep vectors are protected references. Never change them merely to make a failing test pass.
- A justified reference correction requires root-cause investigation, presentation of the evidence and proposed change, and explicit user approval. Hardware-derived references also require precise hardware evidence.
- For hardware/regression review, consult the [Platform Quirks and Invariants Catalog](../../Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md) and relevant validated test evidence.
- Harness normalization or masked differences require precise evidence and a condition limited to the demonstrated case, such as the specific exception frame and fields. Do not broaden comparison exceptions to obtain a pass. Report existing masks as verification limits; narrowing them must follow the defect and reference-conflict procedures rather than silently changing expectations.
