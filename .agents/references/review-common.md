# Independent assessment

Use the assigned review skill and its criteria to inspect the selected files,
changes and affected interactions against approved requirements. The user
chooses the scope and which reviews to run. There is no mandatory set of review
areas or rounds.

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

For each concrete issue, explain what triggers it, what should happen, what you
observed and what consequence follows. Include the relevant location and
supporting criterion. Preserve supplied finding IDs when reassessing them. If
your probe forces a state, its result applies under the conditions it created.
Trace a supported path that leads to the reported problem.
Distinguish an observed failure from a hypothesis or missing evidence.

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
