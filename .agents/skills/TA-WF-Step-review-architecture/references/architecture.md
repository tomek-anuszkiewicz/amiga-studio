# Architecture and code quality

Use the [architecture criteria](../../../references/architecture-criteria.md) as
the authoritative standards for the assessment. Read the shared Rust and egui
practices and the workspace and domain requirements that apply to the change.
Limit checklist reading to this review lane.

Inspect changed production numeric literals explicitly, including hardware
addresses, flags, masks, shifts, encodings, timing values, limits and table entries.
Verify which definitions own the values and the exact meanings of REGISTER_,
BIT_ and MASK_. Preserve the exceptions for ordinary indices, zero and increments,
and independent test encodings.

Assess boundaries, minimum visibility, re-exports, cohesion, duplicate mechanisms,
complete internal migrations, accessors, hot-path constraints and prohibited
constructs. Confirm dead-code candidates against actual identity, callers,
conditional compilation and Host I/O contracts. Report confirmed issues and
rejected scanner candidates; do not prune during review.

Use the existing Clippy, architecture and code quality results without
rerunning those checks. If a result is missing or stale, explain what that
prevents you from assessing. Specific authorization still applies to protected
checks, dependency permissions and enforcement changes even when a checker passes.
The test review lane assesses whether assertions are sufficient. It also checks
which commands ran for each verification gate, what they covered and what
results they returned. Report any demonstrated issue that affects another lane.

For changed PowerShell tools, apply the relevant
[PowerShell result criteria](../../TA-write-powershell/references/acceptance.md).

For workflow changes, check that the main session follows
[AGENTS.md](../../../../AGENTS.md#user-selected-operations) when coordinating the
user-selected operation. The selected skill owns its procedure and stopping boundary.
Keep result formatting and data validation separate from authorization and execution.
Check affected consumers against the current
[result contract](../../../references/workflow-contract.md) after an internal migration.
