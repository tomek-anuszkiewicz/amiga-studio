# Receive a result and offer the next choice

The receiving main session applies this procedure automatically after each selected
operation returns, whether the user named an agent, a skill or their combination.
It interprets the result; specialists retain their atomic scope.
Reading, explaining and presenting choices do not start another operation or grant
authorization. The worker's `recommendation` is advice, never a second active menu.

## Establish what happened

1. Read the actual JSON, matching text and relevant evidence under the
   [invocation contract](workflow-contract.md). Preserve the worker's original
   status, findings, verdict and recommendation. Separate a completed assessment
   requesting changes from successful implementation and verified batch readiness.
2. Check task, work item, run, source result hash and candidate identity against
   the current task. Inspect referenced command outcomes; schema validity alone
   proves neither execution nor approval. Keep canonical verification, independent
   scoped verdicts and repair claims distinct. Do not rerun checks while interpreting.
3. Explain uncertainty and missing evidence honestly. For interrupted work, inspect
   actual edits and surviving commands under the contract before offering a resume.
   For exhausted rounds, preserve the predecessor and cumulative work history;
   a continuation needs a concrete new approach, not a fresh counter for repetition.

For review results, apply the [review criteria](review-common.md#assigned-reviewer)
to the evidence behind each proposed correction. State which project use or
requirement it affects and what would go wrong. If the evidence establishes
only a constructed scenario, say what connection is missing and leave it out
of recommended repairs, tests and further rounds. Preserve the reviewer's
verdict and identify any disagreement in the receiving session's assessment.

## Present one current set of choices

Use the [official skill matrix](../../docs/workflow-skill-matrix.md) before choosing
operation options. Its rows are completed/returned skills and its columns are
possible next skills. Read only the relevant row with
`python tools/workflow/result.py --next-skills <source-skill>`, using the actual
source result's skill. The authoritative data is
[`skill-matrix.json`](../../tools/workflow/skill-matrix.json); the Markdown matrix
is a generated view. Role ownership remains in the existing result schema.

For each nonempty cell, assess its named conditions against the real result,
the current task and still-valid evidence. A code describes a condition to assess,
not a successful status or an instruction to run. A completed review can require
changes; one approved lane is not complete batch approval. Select only useful,
applicable transitions from this row. State the chosen condition and its concrete
evidence basis in each operation option, and bind the matrix file by hash. The
validator checks the declared source/destination cell and condition membership;
the receiver remains responsible for whether the condition is actually satisfied.
Missing prerequisites remain visible rather than turning a marked cell into a
ready assignment. Reuse existing coverage and checks when valid.

An empty cell means no official recommendation for that pair. It does not forbid
the user from explicitly choosing any other skill. A direct request outside the
matrix goes through the ordinary invocation contract, scope/authorization and
prerequisite checks; do not demand approval merely because the cell is empty, add
an invented matrix edge, or disguise the request as an official recommendation.
Do not put such an operation in the official option set on the agent's initiative.
After that directly selected operation returns, resume guidance from its own row.
Clarification, approval, stop, defer and finishing need no skill-transition cell.

In the user's language, briefly state the outcome, its consequence, the recommended
next step and why. Offer normally one to three useful options with their effects
and prerequisites. Use no padding and invent no next task after completion: an
empty option set can correctly report that the requested work is finished.

Persist the caller's interpretation as a fresh matching `guidance.json` and
`guidance.md` pair under the source run's `.agent/tmp/workflow-TA/` directory, in
a uniquely named guidance subdirectory. Use the
[existing data and rendering tool](../../tools/workflow/README.md); it validates
and presents data, not permission or execution. Link the exact source result and
both its candidate and the current candidate. Record gaps instead of invented
identities. Never modify a consumed worker result to add caller decisions.

Name the current `guidance_id` in the conversation. When replacing it, retain the
old pair and identify it with `supersedes`; only the replacement is selectable.
There is one current set per receiving work item, not parallel worker/caller menus.
A later result replaces that work item's choices with a new source-bound set.
This is conversation state with small artifacts, not a decision registry.

Each executable option names the executor and skill from the schema's existing
skill contract, exact input references, selected finding IDs, read/write scope,
round budget and continuation history where applicable. Its generated instruction
must be complete and copyable, including the candidate and guidance identity.
Use actual references, never unresolved placeholders. The user may select its
label/number in this conversation or paste that instruction; no new slash command
or installed execution CLI is implied.

If inputs or authorization are missing, label the proposal accordingly and explain
what is needed. It is not a ready operation. Clarification, a concrete approval
decision, stop and defer are conversation choices with no invented executor/skill.
Persist artifacts in English; translate the brief explanation and option labels
for the user without changing IDs, scope or evidence.

## Interpret failures without changing acceptance

Recommend a bounded repair or investigation when evidence supports it. Distinguish
a code defect, faulty expectation, environment failure and unknown cause. Offer
a test audit only when test validity is genuinely unresolved. A worker's inability
to fix a problem does not establish that the test or requirement is wrong.

A justified criterion-change proposal identifies the exact change, evidence,
alternatives, affected protection and what would be lost. Offer the approval
decision separately from repair selection. Keep hardware and golden protections.
Generic continuation or repair permission is not permission to weaken criteria;
the implementation task's exceptional authority is not a future default.

Allow deferral or identified independent work while preserving required unresolved
finding IDs and failed gates. Deferral cannot permit a blocked approval, commit or
publication. Do not offer ignoring required failures as a route to success.

## Consume the selection

Wait for a selection. Silence and the recommendation grant no permission. Existing
permission remains valid for its exact scope and need not be requested twice, but
permission alone does not select a next operation. Only explicit delegation lets
a trial coordinator choose on the user's behalf; record that provenance instead
of inventing a user reply. Such delegation does not change the default workflow.

Resolve the choice against the current guidance ID. If a label is ambiguous, ask
for clarification. Recheck result/candidate identities, selected scope, prerequisites
and relevant evidence, including the matrix version and selected condition, with
lightweight comparisons. If they changed materially,
explain the change and replace the affected options; do not run a stale instruction.
Keep earlier evidence and unresolved finding IDs. No fresh tests, coverage or
reviewers are needed merely to check applicability.

For a ready selected operation, apply the [runtime settings](workflow-runtime.md#model-and-reasoning-effort)
to the user's current selection. Dispatch exactly the bound role and skill with
the displayed scope and budget, then stop dispatching. If that role is unavailable,
report the runtime prerequisite; do not silently substitute another role. An
explicitly authorized fallback records the actual executor and inherited access,
without claiming named-role execution or technical isolation. The next real result
returns here for a new explanation and choice. For conversation choices, perform
only the stated conversation action; an approval does not automatically dispatch
its proposed repair.
