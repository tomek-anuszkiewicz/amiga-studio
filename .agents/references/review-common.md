# Independent assessment

Use the assigned review skill and its criteria. Inspect the selected files, changes
and affected interactions against approved requirements. The user selects the
scope and review operations; no fixed set of lanes or review rounds is mandatory.

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

For each concrete issue, explain the trigger, expected and observed behavior,
consequence, relevant location and supporting criterion. Preserve supplied finding
IDs when reassessing them. A forced probe establishes only the conditions it
introduced; show a supported path to the reported problem. Distinguish an observed
failure from a hypothesis or missing evidence.

## Review after correction

Recheck selected findings against the actual correction and its affected callers.
Use applicable existing test results, stating their scope and limits. Do not infer
success from a source edit or a report label. Identify scope that was not inspected and any
remaining uncertainty. No report hashes, reservations, candidate seals, round
manifests or human-checkpoint tokens are required.

Return the scoped verdict, findings and actual checks in text and JSON under the
[result contract](workflow-contract.md). A completed review may request changes;
missing required observations cannot establish approval. Suggestions return to
the user and do not authorize another operation.
