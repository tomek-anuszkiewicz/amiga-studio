# Independent assessment

Use the assigned review skill and its criteria to inspect the selected files,
changes and affected interactions against approved requirements. The user
chooses the scope and which reviews to run. There is no mandatory set of review
areas or rounds.

For a change review, assess the delivered changes identified by the supplied
commit range, diff or file list, together with their affected interactions.
Within each file, follow the changes and their consequences. Read unchanged
callers, dependencies, tests and specifications as needed to assess those
consequences.

For each finding, identify the delivered change that introduced, made reachable
or worsened the issue, or the approved requirement assigned to that change that
remains unmet. Keep unrelated pre-existing defects in separate optional
observations; they do not expand the assigned review or repair scope. A broader
stage or repository assessment requires an explicit assignment.

Run required checks and audits at their prescribed scope and report their actual
failures even when they concern unchanged files. The finding-scope rule does not
reduce those checks or turn a failed required check into a pass.

Use fresh context for an independent review. Report inherited author history or
missing inputs that limit independence. Read relevant source and requirements
before accepting an author's explanation. Reviewers report findings without
repairing code, tests or documentation or launching another reviewer.

Search the [review decisions register](review-decisions.md) by subject and owning
requirement. Inspect the current candidate and requirement, compare the trigger,
evidence and consequence with any relevant decision, and explain material differences.
Finding IDs identify findings within their original review. Assess each new claim
on its evidence. Reviewers and challengers may propose an entry or amendment in
their results. The main session records a decision only within authorized write scope.

## Which findings to report

For a behavior defect, trace a supported action in the current code from its
caller or workflow step to the incorrect result. State what should happen,
what happens instead and who or what sees the difference. Cite the relevant
source and distinguish a result established from code from one observed in a
run. A rare failure qualifies when the current path can reach it. If an
explicit requirement covers the case, cite it and show where the change falls
short, even if no current caller exercises it.

Give particular attention to silent errors. For a test or check that can pass
with the wrong result, name the behavior it claims to verify, the incorrect
result it would accept and the input or assertion that allows it. For an
incomplete report, identify what it omits and how the omission can escape
notice. Check whether another required check already detects the same error.

A probe establishes what happens under the conditions it supplies. Omit a
claim that depends solely on a future code change, an unsupported call or a
departure from the normal procedure. This includes a user manually changing
files or directories outside the application during an active operation,
unless handling that change is required. When a future edit or skipped maintenance step
would already cause the required checks to fail visibly, that failure is the
expected protection. Report a present gap if those checks could accept a
wrong result for supported use.

## Review after correction

[Vocabulary-only maintenance](workflow-contract.md#vocabulary-only-maintenance)
does not reopen a review or consume a corrective pass. Continue the existing
assessment of other changes; no new review, challenge or check is needed solely
because a justified dictionary entry was added.

Recheck selected findings against the actual correction and its affected callers.
Use applicable existing test results and say what they cover and where their
evidence stops. A source edit or report label alone cannot establish success.
Identify parts of the scope you did not inspect and any remaining uncertainty. No report hashes, reservations, candidate seals, round
manifests or human-checkpoint tokens are required.

Return the scoped verdict, findings and actual checks in text and JSON under the
[result contract](workflow-contract.md). A completed review may request changes;
missing required observations cannot establish approval. Suggestions return to
the user and do not authorize another operation.
