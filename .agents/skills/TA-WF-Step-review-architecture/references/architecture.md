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

For workflow guidance, retain one receiving-session procedure and the existing
schema's skill/role mapping. Keep data validation/rendering separate from human
authorization and execution; reject competing menus, duplicate registries and
obsolete result-field consumers after an internal migration.

The skill recommendation matrix has one data owner and a derived readable view.
Reuse the existing skill/role mapping; row/cell validation is data checking, not
an authorization engine, scheduler or second role registry. Keep direct invocation
independent from official recommendation membership.
