---
name: code-review
description: Review an exact candidate through four independent, narrowly scoped reviewers covering behavior, architecture, tests and evidence, and specifications and instructions.
---

# Four Independent Review Lanes

Use this skill for a prepared batch, a required milestone review, or an explicit
review request. Ordinary edits alone do not trigger a preparation batch.
The coordinator arranges reviews; an assigned reviewer executes only its lane.

## Coordinator

1. Identify the exact baseline and candidate using native Git: a commit, staged
   tree, or digest of the selected diff and new-file contents. Preserve unrelated
   work. Supply the intended versions, not a mixture of staged and working copies.
2. Start four separate `code_reviewer` instances with `fork_turns: "none"`. Give
   each one lane from the table, the same candidate identity and baseline, relevant
   paths and requirement locations, and the proposed partition when preparing a batch.
   Do not pass the author's conversation, reasoning or implementation narrative.
3. Run lanes concurrently within available slots. With only three spare slots,
   start three and start the fourth when a slot becomes available. Four approvals
   are still required; coordinator self-review cannot replace a lane.
4. Send raw verification evidence and corrections to the owning reviewer. Keep
   candidate inputs stable during checks. Reuse each reviewer's context for rechecks
   only when the change affects that lane's reviewed content, substantive requirements, assessment criteria,
   partition assessment or evidence. Follow the approval-reuse procedure below.
5. Combine all four scoped verdicts. A blocker in any lane blocks approval. Resolve
   findings once and share boundary findings with the affected lane. Four approvals
   cover the combined candidate, not independently tested intermediate commits.

## Reuse Scoped Approvals

Keep the four initial independent reviews. After a correction, compare its exact
delta with the scope and inputs covered by each existing approval. Use native Git
comparisons or the reviewed scope's diff/content identity; no extra per-file catalog
or helper script is required.

- Descriptive documentation corrections for unchanged, already reviewed behavior
  return only to the documentation lane. That reviewer checks whether the edit
  changes a requirement, acceptance criterion or reviewer instruction, and identifies
  any additional affected lanes.
- Changes to behavior, architecture, requirements, review criteria, partition or
  verification evidence return to the lanes they actually affect. Unknown impact
  requires a conservative recheck. A different baseline does not inherit approvals.
- Carry an unchanged lane's approval without invoking that reviewer again when
  its relevant content, substantive requirements, domain criteria and evidence still match. Preserve
  the original verdict, reviewer and approved identity; record the comparison and
  why the verdict applies to the final candidate. Do not present it as a new review.
- A changed overall candidate fingerprint alone does not reopen unchanged lanes.
  Consolidate renewed and carried approvals for the final candidate. Never replace
  a missing or failed lane with a carried verdict or coordinator self-review.

| Lane | Required reviewer instructions | Owns |
| --- | --- | --- |
| Behavior and correctness | [behavior.md](references/behavior.md) | Data flow, state transitions, interactions and failure behavior |
| Architecture and code quality | [architecture.md](references/architecture.md) | Boundaries, authoritative definitions, numeric values and code structure |
| Tests and verification evidence | [tests-and-evidence.md](references/tests-and-evidence.md) | Behavioral protection, coverage, gates, approvals and evidence freshness |
| Specifications, documentation and governance | [documentation.md](references/documentation.md) | Requirements, procedure parity, links and reviewer-instruction synchronization |

## Assigned Reviewer: Common Contract

Read this contract and only your assigned lane checklist. Read `AGENTS.md` and
upstream requirements relevant to that lane; do not load the other lane checklists.
Review independently and read-only. Do not edit, stage, commit, delegate, run
competing shared-target checks, or request the author's conversation history.
If history was inherited, report the isolation failure and require a fresh instance.

Inspect the selected diff and needed surrounding callers before consulting author
results. Scale depth to concrete risk; avoid unrelated audits. Follow a finding
across a lane boundary far enough to explain it, then identify its owning lane.
Do not leave a demonstrated problem unreported because another lane may see it.

Report actionable findings by severity, with path/location, trigger, consequence,
and the requirement or evidence. Separate blockers from optional suggestions.
State the baseline, exact candidate identity, lane, inspected scope and evidence
limits. Use `APPROVED` only for your lane when its required checks are satisfied;
otherwise use `CHANGES REQUESTED`. An inapplicable domain needs a brief scope
explanation, not a fabricated audit. Final approval is the conjunction of all lanes.
Identify the reviewed paths and relevant inputs clearly enough for the coordinator
to compare later corrections with this scope. An unchanged approval may be carried
under the procedure above; it does not need a new verdict for a global fingerprint.

For commit preparation, the tests/evidence lane confirms passing canonical evidence
for the whole candidate; milestone gates retain their own trigger. Ordinary review
uses the relevant focused checks and does not demand a commit profile. Reuse valid
evidence rather than rerunning checks merely because a reviewer was invoked.
