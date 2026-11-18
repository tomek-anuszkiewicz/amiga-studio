# Testing policy

Tests belong in the owning crate/tool's local `tests/`, never inline in production
`src/` or in a new global component-test directory. Cross-component tests belong
to the integration owner. Reuse meaningful coverage and independently expected
results; detailed design/assertion/evidence standards are in
[testing criteria](../references/testing-criteria.md).

Software tasks design agreed scenarios before implementation. Implementers may
adapt fixtures and strengthen assertions within them. A newly discovered scenario
outside that plan requires a concrete caller decision; report-only auditors do not
add tests. Required in-scope gaps still block readiness. Assigned repairs load
[original reproduction](../references/defect-resolution.md) before mutation.

Maintain chip integration protection for gating, Chip RAM transfers, peer signaling
and bus contention. Synthetic CPU buses do not establish production integration.
Every executed SingleStep vector compares state, instruction clocks and timed bus
transactions to its reference; sampling only reduces input count. No weakened modes
or unverified-success skips. CPU paths absent from vectors require handwritten
independently expected cycle-exact regressions in `crates/cpu/tests/`, including
observable access timing/data and wait-state retries when relevant. Document harness
limits in the [CPU specification](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md).
Whole-machine hardware capture execution remains unavailable.

## Required Verification

Ordinary development runs relevant focused checks, repeated only for changed
inputs, failures or unresolved concerns. Python contracts follow affected owning
inputs/shared dependencies through `verify.py test python`; explicit directory
selection is diagnostic. Batch preparation requires `verify.py commit` for the
complete candidate. Actual minor-roadmap/major-milestone closure also requires
`verify.py milestone`, including full SingleStep and full language audit. Full
standalone SingleStep otherwise needs explicit diagnosis or request.

The [testing strategy](../../Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md#completion-entry-point)
and [verifier](../../tools/verification/README.md) own package/consumer selection,
shared/unknown-input fallbacks, quality gates, ordinary tests/coverage, standard
harness targets and Python scope. Do not replace their verdict with an agent claim.
For GUI, workspace-manifest or browser-pipeline changes, development also runs
`python tools/verification/verify.py check web`. Every commit profile requires the
actual browser build through `tools/web.ps1`; Cargo lint warnings fail that gate.
Missing/failed evidence blocks readiness. Run independent checks concurrently only
with isolated mutable inputs/targets; never edit the snapshot during checks.

## Coverage During Development and Review

Affected emulator production code requires canonical ordinary-package coverage
measurement and review of unexecuted functions/source spans during preparation.
Earlier measurement needs a diagnostic reason or explicit request. Assess final
assertions and measurement scope under [coverage criteria](../references/testing-criteria.md#coverage-and-adequacy-assessment);
percentages and passing tests alone do not establish protection.

Resolve uncovered reachable changed behavior and affected interactions before
freeze. Existing agreed scenarios permit routine test improvements; newly identified
scenarios beyond the approved plan are selected separately. Missing decisions leave
readiness incomplete, not silently waived. Report pre-existing out-of-scope gaps
separately; no automatic whole-emulator audit, coverage exclusions or meaningless
assertions to hide them. Bootstrap converter exceptions remain below.

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

Routine contextual language fixes, reviewed vocabulary and justified exact-file
literals have standing authorization under [language policy](language-policy.md#resolving-findings),
not permission to alter scanner logic/scope. Protected references and comparisons
remain under [reference integrity](spec-compliance.md#reference-artifact-integrity).

## Bootstrap Converter Scope

For `tools/bootstrap/conversion/`, `pdf-to-markdown/`, `html-to-markdown/` and
`book-to-markdown/`, use [converter testing](../../tools/bootstrap/AGENTS.md#converter-testing).
Its technical-test/editorial exceptions are authoritative for those tools, without
importing emulator CPU/hardware requirements or waiving repository per-commit gates.
