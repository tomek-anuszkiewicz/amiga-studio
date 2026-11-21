# Independent assessment

This reference defines review inputs and evidence. The caller selects a named
review skill; this reference does not dispatch a review set or select another lane.
Use the [invocation contract](workflow-contract.md) for identities, limits and dual
results. Review-owned criteria are binding result standards, alongside the advance
requirements that remain in rules and specifications.

## Assigned reviewer

Read only the assigned lane, relevant requirements and exact candidate. Start
without the author's conversation or reasoning. If history was inherited, report
the isolation failure; that instance cannot supply an independent batch approval.
Record the actual access controls. Review is read-only apart from result artifacts
and explicitly assigned diagnostics; do not edit source, tests or documents, stage,
commit, delegate or start competing shared-target checks.

Inspect the selected diff and affected callers before consulting author results.
The candidate includes selected new files and dirty content, not merely HEAD.
Review the entire assigned scope, including relevant unchanged consumers and
requirements. For each central guarantee, start with the supported operation or approved requirement it protects and, if the candidate shows a concrete path to a violation, trace or probe that path to establish its consequence. After finding a blocker, finish the
remaining accessible scope in the same pass. Return all supported material
findings together. Name the paths, interactions and criteria actually inspected.
If the limit or a missing input prevents completion, state the unchecked scope,
its consequence and the verdict the available evidence supports.

A source trace can establish this
before anyone encounters the failure. For a diagnostic that forces a platform,
state or timing, explain how the same conditions arise through a supported
entry point. A forced probe alone establishes only what happened in the probe.
Leave scenarios that require a future caller or behavior change to that change.

Report stable finding IDs, severity, confidence, location, trigger, expected and
observed behavior, consequence and supporting requirement/evidence. Separate
required corrections, optional improvements and unverified evidence. For each
required finding, name the binding requirement or gate separately from the
effect. For a gate failure, describe the actual effect on verification or
approval. Attribute a product impact only when supported. Omit stylistic
preferences about already-clear code or prose.

Record baseline, candidate, inspected scope and limitations. Give the result
its hashed review inputs and fresh context receipt. Bind evidence to the
reviewed candidate. JSON uses
`approved`, `changes_requested` or `not_assessed`; an unfinished or failed review
cannot approve. When the assigned scope is covered, required evidence is present
and no required correction is supported, return `approved` in that round.
An empty findings list is a complete review result.

### Workflow defects

For a required workflow repair, give the shortest supported sequence that
reaches the failure on the candidate. Start at a supported entry point or at
an intermediate state established by a recorded run or a trace through the
code and procedure. Name who performs each decisive action, what state it
changes and which later decision or visible outcome fails.

A focused probe may recreate the intermediate state. Identify the conditions it
supplied and show how the workflow can produce them together in that order.
If the setup supplies the only evidence for the decisive transition, identify
the missing step and leave the claim off the required repair list. Reconsider
it when new evidence establishes the step. An independently binding requirement
or failed mandatory check can still be reported on its own evidence.

## Four-lane batch evidence

Batch preparation requires four independent instances of `TA-code-reviewer` with
fresh context, separately assigned behavior, architecture, tests/evidence and
documentation skills. The caller schedules them within available slots, in waves
if necessary, and retains all four reports and identities. No lane launches another
lane. An author, Test Auditor or CPU diagnosis cannot replace a required reviewer.

The assignment includes exact baseline/candidate, scope and requirement locations,
plus proposed partition at batch preparation. The tests/evidence lane confirms
canonical evidence for the combined final candidate. Milestone gates apply only
at their actual trigger. Standalone review uses its selected focused evidence and
does not manufacture batch readiness. Missing or failed required lanes block
readiness. The receiver preserves each original verdict and applies the
[binary finding decisions](workflow-guidance.md#challenge-required-findings)
to assess whether a completed lane has any remaining required findings.

## Review after correction

The caller supplies the predecessor report, the exact predecessor and corrected
candidates, their actual delta and the current criteria and evidence. Compare
the original trigger on both candidates for each selected finding. Keep its
original task, work item, reporting lane and ID. The current owning reviewer
records independent evidence for a verified or refuted disposition. An
implementer repair claim leaves it pending. Reopen a settled ID only with new
evidence that changes the disposition. Link duplicates to the original issue.

For a newly reported issue, distinguish parent pass and current failure,
failure on both within earlier covered scope, failure in an earlier documented
gap and an unavailable parent observation. Record changed requirements, expanded
scope and newly available evidence separately. Keep the defect's `origin`
separate from what the comparison establishes about the review history. Evaluate
the current consequence and binding requirement on their own evidence.

Use the [review routing tool and table](../../tools/workflow/README.md#review-routing-and-human-checkpoint).
Focused reviews cover the owners of selected findings and the effects of the
actual correction. Once all focused reviewers verify their assigned repairs
and approve, obtain every remaining required approval on the exact frozen
candidate.

### Reuse scoped approvals

A previous approval remains attributed to its original reviewer
and candidate. Credit it on the current inputs only if it already covers this
candidate, baseline, scope, requirements, criteria, partition and evidence.
Retain matching prior evidence. Unknown impact calls for all applicable lanes.
A changed baseline requires first-review coverage and retains the round count.
Standalone compliance and parity keep their own triggers and scopes.

The verifier independently owns test-evidence validity and reuse. Review reports
link actual commands/artifacts and distinguish newly executed from retained
evidence; they neither replace a failed gate nor authorize repairs or publication.

The receiver applies [the human review checkpoint](workflow-guidance.md#human-review-checkpoint)
before offering repair, another lane or subsequent work after the third round.
Healthy work already running on the frozen candidate can return its evidence.
The receiver then applies [next-step guidance](workflow-guidance.md).
Reviewers provide their own verdict and recommendation, not a caller menu or
dispatch decision. Assess guidance against the actual evidence and authorization,
including the current option identity, exact executor/skill binding, selection
boundary and stale-candidate handling. A rendered instruction is not proof of its
execution, and a repair claim cannot replace renewed affected review.
