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
Inspect only interactions needed to assess the assigned risk. Follow a cross-lane
finding far enough to explain it, then identify its owner. A demonstrated issue
must not disappear because another reviewer might find it.

Before requesting a correction, identify the affected project operation or
approved requirement, the conditions that can arise in the candidate as used
or specified, and the concrete consequence. A source trace can establish this
before anyone encounters the failure. For a diagnostic that forces a platform,
state or timing, explain how the same conditions arise through a supported
entry point. A forced probe alone establishes only what happened in the probe.
Leave scenarios that require a future caller or behavior change to that change.

Report stable finding IDs, severity, confidence, location, trigger, expected and
observed behavior, consequence and supporting requirement/evidence. Separate
required corrections from suggestions. Record baseline, candidate, inspected
scope, relevant requirements/criteria, evidence and limitations. JSON uses
`approved`, `changes_requested` or `not_assessed`; an unfinished or failed review
cannot approve. When the assigned scope is covered, required evidence is present
and no required correction is supported, return `approved` in that round.
An empty findings list is a complete review result.

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
readiness. Consolidation preserves independent verdicts; it is not a fifth review.

## Reuse scoped approvals

After corrections, compare the exact delta with each original review's content,
requirements, criteria, partition and evidence using native Git and recorded
identities. Preserve the original reviewer, verdict and identity for carried
approval, plus the comparison supporting its applicability to the final candidate.
Do not describe retained approval as a new review.

Descriptive corrections for unchanged behavior return to the documentation lane;
that reviewer identifies any changed requirements or criteria affecting other
lanes. Behavior, architecture, requirements, partition or evidence changes return
to the lanes they affect. Unknown impact requires conservative reassessment.
A different baseline cannot inherit approval. A changed global fingerprint alone
does not reopen unchanged lanes. Never carry a missing or failed verdict.

The verifier independently owns test-evidence validity and reuse. Review reports
link actual commands/artifacts and distinguish newly executed from retained
evidence; they neither replace a failed gate nor authorize repairs or publication.

The receiver applies [next-step guidance](workflow-guidance.md) after assessment.
Reviewers provide their own verdict and recommendation, not a caller menu or
dispatch decision. Assess guidance against the actual evidence and authorization,
including the current option identity, exact executor/skill binding, selection
boundary and stale-candidate handling. A rendered instruction is not proof of its
execution, and a repair claim cannot replace renewed affected review.
