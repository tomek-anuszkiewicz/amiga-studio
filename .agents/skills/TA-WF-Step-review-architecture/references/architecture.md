# Architecture and code quality

Apply [architecture criteria](../../../references/architecture-criteria.md) as the
authoritative detailed result standards. Read shared Rust/egui practices and workspace/domain
requirements for the changed scope, not unrelated lane checklists.

Inspect changed production numeric literals explicitly, including hardware
addresses, flags, masks, shifts, encodings, timing values, limits and table entries.
Verify owning definitions and exact REGISTER_, BIT_ and MASK_ meanings; preserve
ordinary-index, zero/increment and independent-test-encoding exceptions.

Assess boundaries, minimum visibility, re-exports, cohesion, duplicate mechanisms,
complete internal migrations, accessors, hot-path constraints and prohibited
constructs. Confirm dead-code candidates against actual identity, callers,
conditional compilation and Host I/O contracts. Report confirmed issues and
rejected scanner candidates; do not prune during review.

Use existing Clippy/architecture/code-quality results, without rerunning them.
Missing/stale results remain explicit limitations. Protected checks, dependency
permissions and enforcement changes require their concrete authorization;
a passing checker is not approval. Leave assertion adequacy and gate accounting
to their owning lane while reporting any demonstrated cross-lane issue.

For changed PowerShell tools, apply the relevant
[PowerShell result criteria](../../TA-write-powershell/references/acceptance.md).

For workflow changes, check that the main session follows
[AGENTS.md](../../../../AGENTS.md#user-selected-operations) when coordinating the
user-selected operation. The selected skill owns its procedure and stopping boundary.
Keep result formatting and data validation separate from authorization and execution.
Check affected consumers against the current
[result contract](../../../references/workflow-contract.md) after an internal migration.
