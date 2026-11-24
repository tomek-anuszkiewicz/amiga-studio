# Verification change authorization

## Protected Verification

Get explicit user approval before adding, changing or deleting architectural
tests, checker implementations or policy data. This applies to stricter checks
and mechanical refactors as well as dependency permissions, limits, exceptions
and enforcement scope wherever they are stored. A general request to implement,
refactor, fix tests or commit gives no such approval. Show the user the current
constraint, the proposed diff, the reason for it and how it would affect which
code passes. Follow the existing constraint unless the user approves the change.
Manifests, documentation, passing checks and another agent's approval cannot
substitute for that decision.

Get the same explicit approval before weakening any other test or required gate.
This includes weakening expectations, tolerances, masks or exceptions, removing
or skipping assertions or cases, and reducing scope. Present the evidence and
explain which protection the change would remove. A check that appears obsolete
still requires approval to weaken. Ordinary maintenance that preserves
acceptance and coverage remains allowed, including import changes, fixture
updates for internal refactors, meaningful new tests and stronger assertions.
A specific approved contract change can authorize its matching ordinary test update.

Justified English word-list additions and exact-file literal allowances are ordinary
[vocabulary-only maintenance](workflow-contract.md#vocabulary-only-maintenance),
including additions made during ordinary work. They need no separate approval or
follow-up checks. This exception does not cover scanner logic, discovery,
exclusions or other verification policy changes.

A due corrective English scan, including standalone vocabulary cleanup,
authorizes exactly the removal batch accepted on fresh, complete repository
evidence under the [vocabulary removal procedure](workflow-contract.md#validated-vocabulary-removals).
Confirm the result with the full post-edit comparison. A report-only request
supplies findings. Other vocabulary removals require approval of their concrete
diff.

## Reference Artifact Integrity

Reference traces, CSV baselines, cycle totals and SingleStep vectors are protected.
Never change them to make a failure pass. To justify a correction, show evidence
of the root cause, propose the change and obtain explicit approval. For a
hardware-derived reference, also provide precise hardware evidence. For relevant review consult
[platform invariants](../../Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md)
and validated test evidence.

For normalization or comparison masks in the harness, provide precise evidence
and limit the condition to the demonstrated case, such as the exact exception
frame and fields. Do not broaden exceptions to obtain a pass. Report existing
masks as limits on what verification establishes. Follow the procedures for
defects and reference conflicts even when narrowing a mask.
