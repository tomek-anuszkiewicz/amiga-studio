# Receive a result and offer the next choice

The receiving main session applies this procedure automatically after each selected
operation returns, whether the user named an agent, a skill or their combination.
It interprets the result; specialists retain their atomic scope.
Reading, explaining and presenting choices do not start another operation or grant
authorization. The worker's `recommendation` is advice, never a second active menu.

## Establish what happened

Apply [the shared explanation rule](../../AGENTS.md#explain-the-system-to-the-engineer)
to the first account of the result.

1. Read the actual JSON, matching text and relevant evidence under the
   [invocation contract](workflow-contract.md). Preserve the worker's original
   status, findings, verdict and recommendation. Separate a completed assessment
   requesting changes from successful implementation and verified batch readiness.
2. For a handoff, decision or evidence reuse, bind the task, work item, run and
   relevant inputs to the current task. Use the owning check and reuse its
   observation at that boundary. Inspect referenced command outcomes; schema
   validity alone proves neither execution nor approval. Keep canonical
   verification, independent scoped verdicts and repair claims distinct.
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

## Explain the result to the engineer

Use plain language for progress and final replies. Name the action and its
object in the user's language. For a check, explain what it established and
what that means for the task. Name an internal operation when the user needs
to select it or find its record. Use descriptive task labels. For example,
state the selected task branch and commit, the target branch and that any
authorized removal follows integration. Show individual digests for a mismatch,
an explicit request or a recipient who needs them.

Explain the claim that drives a decision. In the historical capture workflow,
a successful `register.py stop` closed one task's recording interval while the
shared receiver kept accepting events.
The client invoked a Python process inside the receiver with `docker exec`.
That process wrote `end_utc` to the task's mounted `registration.json`. During
the migration the newer launcher rejected the old receiver revision before that
write. The interval stayed open, so the upgrade refused to proceed. Stopping
the container with `docker stop` would also have interrupted other tasks. The
proposed REST stop request belonged to a later design.
[Historical capture discussion](codex://threads/01a1f324-6e6e-7b70-9908-9af290295dec).

## Save a formal plan

A request to save agreed behavior as a formal plan selects
`TA-planner` with `TA-plan-change` and any applicable `TA-test-designer` work
with `TA-design-tests`. This includes asking to save agreed behavior for later
after an interactive discussion. Ordinary edits to a working draft follow the
[shared draft rule](workflow-contract.md#working-drafts-and-version-checks).
Honor an explicit selection of this formal operation. Confirm [telemetry startup](workflow-contract.md#telemetry-startup-prerequisite)
where required. Give the planner the requested change, candidate, plan path,
scope and round budget.

On a valid result with agreed testable behavior, immediately dispatch the
designer for that behavior under the same request. Bind the saved plan revision
and SHA-256, exact candidate, affected scenarios, existing protection and an
owned design output. Preserve distinct assignments, rounds and result pairs.
A prose or research plan with only documentary or observational acceptance
evidence ends with the planner. Finish the planning request after applicable
design and concrete decisions. Ask about missing material behavior before
relying on it. A plan saved with pending decisions identifies the blocked
scenarios; unaffected work can still be designed when its inputs are complete.
A terminal formal planning result keeps its result pair and gets a short
conversational explanation when there is no choice to preserve.

Declare the design destination as a plan-relative link before handing off the
saved revision. The design cites the exact input revision and digest, and
records stable scenario IDs, independent expected observations, reusable
protection, gaps and decisions still open. This one-way digest binding also works
with a separately owned section if the unchanged input revision is preserved
and the assignment names that section as its only write scope. Amend only
affected requirements and interactions. Retain unaffected scenario IDs and
valid evidence, and identify superseded expectations. On execution recheck
the linked inputs against the active plan and current code and tests. Design
scenarios are planned observations until executable checks run.

## Record individual approvals

While planning, and again when design identifies affected tests, consult
[protected verification](../rules/unit-testing-policy.md#protected-verification)
and [reference integrity](../rules/spec-compliance.md#reference-artifact-integrity).
Present the concrete decisions found. Identify the affected file and symbol,
assertion or rule, current constraint, exact diff or before-and-after behavior,
rationale, alternatives, evidence and effect on acceptance. Say which specific
regression could pass if protection drops, or what a stricter rule would newly
reject. For moves and removals, identify both endpoints or the deleted item,
affected references and coverage.

Keep each decision beside its requirement in a versioned plan section. Record a
stable decision ID, plan revision, affected input identities, exact proposed
change, scope and conditions, disposition (approved, declined or pending) and
the user's reply or an unambiguous reference when supplied. Silence, a listed
proposal and permission to fix a class of tests do not grant individual
approval. Ordinary in-scope moves, fixture updates and equivalent assertions
preserving acceptance and coverage remain ordinary maintenance.

At the edit, compare the actual change and current inputs with the recorded
decision. Exact applicability authorizes that edit. Recheck the premises after
unrelated amendments and record the applicability check. A changed destination,
broader exception, newly lost protection or uncertain match requires a decision
about that difference. Diagnose a newly failed check before proposing a change
to its criterion. Prior consent for an edit supplies neither tool permission,
verification, independent verdict nor a [human checkpoint decision](#human-review-checkpoint).

## Amend an active plan

As soon as the user proposes a material change to approved scope, behavior or
acceptance, write a proposal in the authoritative plan's sibling
`<plan-stem>.amendments.md` in `.agent/tasks/`. Record the stable decision ID,
task ID, active plan revision and digest, affected requirement, proposed change,
effect on acceptance and tests, and the user's turn locator or attributed
excerpt. Record pending, approved or declined with its source. Keep the
approved plan in force while the proposal is pending. A repair restoring
agreed behavior or a request for another review round leaves the plan unchanged.
A review continuation authorizes only that round.

Use a heading beginning with the decision ID, such as `## D-03 — Change the
receipt destination`, and an initial `**Disposition.** Pending` in that
section. Preserve that history when a later checkpoint decides the proposal.
Add a separate dated decision with the same ID, for example
`- **D-03 approved.** <source and scope>` or
`- **D-03 declined for this delivery.** <source and excluded scope>`.
Keep decisions in chronological order so the last explicit disposition for
each ID is current. Record incorporation and affected input identities alongside
that decision.

When relying on prior work after a handoff or resumption, read the working plan,
proposals and linked design, then compare the selected files and diff with the
recoverable approved inputs once at use. On request, compare the working plan
with the selected approved text and explain affected decisions and acceptance. For an unexplained
material edit, record its observed effect with origin and intent unknown and
ask what belongs in the task. Use the task records and selected diff for recovery
without hooks or a transcript scan.

Group related pending decisions at the next natural checkpoint, such as a round
result or batch preparation. Resolve a decision sooner when the next action
depends on it. An explicit instruction to incorporate the specified change
selects the amendment. Before replacing approved bytes, confirm the prior plan
and relevant design are recoverable through their
[selected version records](../../tools/workflow/capture/README.md#starting-inputs).
Record the intended revision and affected updates under the approved decision ID. Link that decision
in the new revision and update the affected acceptance criteria and applicable
test design through [the planning handoff](#save-a-formal-plan). Record the
resulting refs when complete. On restart, check the saved refs and complete only
the missing updates under the same decision ID. Apply
[individual approval](#record-individual-approvals) separately to any protected edit.

Record a decline and the user's disposition, such as excluding affected hunks
or assigning them to separate work. Preserve local edits. At preparation,
compare the selected paths and hunks to the active plan and unresolved material
proposals. Resolve or exclude affected scope before freeze, while allowing
an independent selection to proceed. The
[readiness declaration](../rules/git-merge-commits.md#discover-and-select-deliveries)
binds the current amendment note or its absence. A changed note requires a
fresh declaration; other evidence can still be reused within its validity.

## Start or deliver a saved plan

Starting a saved plan selects [an isolated task worktree](../skills/TA-start-plan/SKILL.md).
Record its authoritative copied plan and linked design inputs. Resume from
that recorded checkout and copies. A plain planning request leaves execution
unselected.

An expressly scoped end-to-end delivery request selects the ordinary actions
needed to reach its agreed boundary. Record the plan revision and input hashes,
source checkout/branch, target local `master`, authorized commit and publication
scope, applicable integration and baseline-refresh procedure, cleanup targets
and any selected remote `master` push. If a stage establishes an identity,
record it before operating on it. Within this delegation, the main session runs
the applicable skills and direct commands for checks, reviews, in-scope fixes
and conflict resolution, integration, publication and other selected actions.
Each invocation keeps its exact inputs, write scope, result and round budget.
Apply [the human checkpoint](#human-review-checkpoint) and individual approvals
where required. A narrower request keeps its own boundary. Local delivery alone
supplies no remote push authorization.

At integration, use [the delivery selection rules](../rules/git-merge-commits.md#discover-and-select-deliveries)
to check eligible source records. For single-plan delivery record current
local master as `B` and the selected task tip as `S`. If master already
contains the selected delivery, check the publication and closure record and
ancestry before resuming only outstanding authorized work. Otherwise follow the
[integration policy](../rules/git-merge-commits.md) and
[verifier procedure](../../tools/verification/README.md#local-integration).
With `B` in the task's ancestry, a qualifying clean, unchanged tip with committed
closure and exact approved tested-tip evidence can use `integrate accept-tested`.
A working commit, missing acceptance evidence or outstanding closure changes
takes the full preparation, verification, review and sealing route. On
divergence merge recorded `B` into the task checkout and reconcile the combined
candidate before those checks. Combined Sources follows its separate assembly
procedure.

On every route, pass the
[full English scan of the sealed commit](../../tools/verification/README.md#scan-the-sealed-candidate)
before conditional publication. A moved master invalidates the stale write.
Record the fresh base, recheck the source selection and repeat applicable
synchronization, reconciliation, verification, review and the scan under the
recorded refresh procedure. Ask about changed requirements, sources or
consequences beyond it. Reuse evidence only within its declared validity.

After successful publication retain the record and SHA, then follow
[closure](../../tools/verification/README.md#post-publication-closure) for
graph refresh and record preservation before cleanup. An older publication
with an incomplete historical language stage follows its recorded procedure.
Report local publication, any authorized push and each authorized cleanup
separately. A failed stage preserves its observed outcome and recovery inputs.
Resume only outstanding authorized work.

### Record the source at implementation completion

For an invocation that completes implementation of a selected plan for delivery,
the assigned implementer captures the completed source after its focused checks
and before writing the immutable result pair. The main session supplies the
authoritative plan and matching active registration from the primary checkout.
Run from the task checkout:

```powershell
python tools/workflow/delivery.py snapshot --plan <authoritative-plan> --registration <registration.json> --output <result-run-directory>/source.json
```

Write the fresh receipt beside `result.json` and `result.md` under the
invocation's recorded run directory. In the implementation result add exactly one
executed evidence row with ID `delivery-source`, the actual command and status,
and `artifact_ref` with the repository-relative receipt path and SHA-256 digest.
Render and validate the pair after including that row. If the assigned completion
cannot be captured, report the missing input or error and leave readiness open.

The receipt binds the completed tree and branch, task and execution, plan and
amendment record. Include the plan and any applicable design inputs with their
digests in the result's plan inputs. A clean capture binds the exact completed tip.
A capture with pending edits can bind the immediate subsequent commit only if
its tree is identical and its parent is the captured tip. A later source change
requires current completion evidence. The receiving session uses this pair and
receipt in [delivery declaration](../rules/git-merge-commits.md#discover-and-select-deliveries)
once the source and index are clean. Keep this source evidence distinct from
canonical verification, independent reviews and publication readiness.

## Challenge required findings

At the repair handoff, use the [review router](../../tools/workflow/README.md) for
the frozen round. Record expected, returned and outstanding lanes and reports,
the required lane/ID pairs, current candidate, evidence and attributable scoped
risk decisions. Follow the human checkpoint first. Let healthy assigned work
return. Missing lanes and interrupted or stale evidence leave that handoff pending.

If the collection contains required findings, the selected review includes one
fresh [independent challenge](../skills/TA-challenge-findings/SKILL.md). Claim the
collection before launch. Resume a recorded assignment using its existing receipt.
A clean review or suggestions alone finishes without a challenge. If a late
report or genuinely new material evidence changes the collection, link the
assessment of only the affected claims and retain decisions for the other IDs.

Validate one final `uphold` or `reject` per lane and ID against the collected
reports and exact candidate. An upheld finding remains open for authorized
repair. Apply the [supported-sequence test](review-common.md#workflow-defects)
to the challenger evidence and preserve the actual checkpoint reply when required. Explain the supported use or requirement and the demonstrated
consequence. Group related findings for reference. Repair each upheld finding and verify
its own trigger. A related finding keeps its separate decision. A rejected finding closes on this candidate when
the advisory result is accepted. Retain its primary report, the binary decision
and the evidence. Treat a completed primary lane whose applicable required
findings were all rejected as resolved for this candidate. The lane's original
verdict stays in its report. An incomplete assessment leaves its handoff pending.

Check mandatory gates and required independent review evidence before readiness.
An accepted risk covers its recorded circumstances until a material new fact
changes the premise. A deferred upheld finding stays open. Apply the ordinary
review and evidence rules to a changed candidate.

### Working commit after declining repairs

When declining repairs would prevent the requested destination, explain the
consequence beside that choice. For a direct commit request, the first
user-visible message, including commentary before an inspection tool, starts
with the current warning from the recorded checkpoint already known to the
session. Reserve any inspection update for after the warning. Name the working
commit's local preservation boundary, each lane with unresolved findings and
their IDs, each lane that never started or is still running, and each failed
or missing gate and proof by name. Identify fresh VS Code capture proof when
that is an outstanding requirement. State the route back through the checkpoint.
Freeze and publication still require the outstanding approvals and current
passing evidence. Credit canonical verification only when its passing report
exists. An explicitly selected working commit can proceed under its existing
authority after the warning.

## Present one current set of choices

For a review sequence, apply the [human review checkpoint](#human-review-checkpoint)
before consulting skill transitions. Use the scoped lane advice from the
[review routing tool](../../tools/workflow/README.md#review-routing-and-human-checkpoint).

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

A successful TA-close-task hands off its publication record and SHA to
[the recorded delivery procedure](#start-or-deliver-a-saved-plan) for closure.

In the user's language, briefly state the outcome, its consequence, the recommended
next step and why when a choice remains. Offer normally one to three useful options
with their effects and prerequisites. An ordinary draft edit or a terminal
formal result with no pending selection can use a direct reply.

When offering another operation for selection, tying a decision to the displayed
version or retaining a handoff for resumption, persist the interpretation as
a fresh matching `guidance.json` and
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

Recheck any active [human review checkpoint](#human-review-checkpoint) before
consuming an option or executing an explicitly selected continuation.

Continue only within a recorded selection, including
[planning and design](#save-a-formal-plan) and
[end-to-end delivery](#start-or-deliver-a-saved-plan) where applicable. Otherwise
wait for a selection. Silence and the recommendation grant no permission.
Existing permission remains valid for its exact scope. If a user explicitly
delegates choices to a trial coordinator, record that provenance.

Resolve the choice against the current guidance ID. If a label is ambiguous, ask
for clarification. Recheck result/candidate identities, selected scope, prerequisites
and relevant evidence, including the matrix version and selected condition.
Use the owning tool or procedure once at this decision. Reuse any observation
that already checked the exact ready action against live inputs here.
If they changed materially, explain the change and replace the affected options.
Keep earlier evidence and unresolved finding IDs. No fresh tests, coverage or
reviewers are needed merely to check applicability.

For a ready selected operation, apply the [runtime settings](workflow-runtime.md#model-and-reasoning-effort)
to the current selection. Dispatch exactly the bound role and skill with its
scope and budget. Apply this guidance to the next result before another
dispatch. If that role is unavailable,
report the runtime prerequisite; do not silently substitute another role. An
explicitly authorized fallback records the actual executor and inherited access,
without claiming named-role execution or technical isolation. For conversation
choices, perform the action selected. An amendment approval selects its
incorporation as described above. An individual protected-edit approval supplies
consent for that edit when its operation is selected.

## Human review checkpoint

Count the sequence for the selected work item. The first frozen candidate is
round 1. Reviewing the next corrected candidate adds one round. Focused and
remaining waves, parallel lanes, report retries and new invocations on the same
candidate stay in that round. Preserve linked round and candidate identities
through handoffs and restarts. A changed baseline retains the count. Restoring
evidence in an authorized additional round counts even with identical source
bytes. Use the [read-only routing tool](../../tools/workflow/README.md#review-routing-and-human-checkpoint)
to derive the count and affected lanes. An incomplete history blocks routing.
Keep the work item's current routing reference up to date as reports and
decisions arrive. On resume, recheck the preserved guidance against that
reference and its linked history, including when the candidate is unchanged.

At the end of round 3 or later, stop for any known open defect needing
disposition, required repair still unverified after its assigned review, or any
failed or confirmed missing mandatory gate, independent verdict or evidence.
A healthy lane already running can return its result on the same candidate.
An ordinary pending lane in a clean focused or remaining wave stays in that
round and can run as planned. Once a known issue establishes the stop, launch
no new lane, evidence retry, repair, commit or downstream work on this candidate.
Silence, prior approvals, standing delegation and a coordinator choice leave
the stop in force. A clean result follows its ordinary readiness rules.

Tell the human user in their language how many rounds ran and what the last
round established. Name each open finding by qualified ID and lane or identify
the unavailable artifact. Explain its practical impact, its known or uncertain
history and what still blocks progression. Offer exactly these three decisions
with the named checkpoint and current guidance ID.

1. Accept the named issues and proceed within existing authority. Record each
   accepted disposition. Every required finding, failed gate and missing
   approval still blocks its affected stage. Changing a binding requirement
   needs its owner's separate decision.
2. Authorize one further repair and review round for the named issues or
   evidence gaps. Record the one-use continuation receipt before dispatch.
   Ask again if that round ends with an unresolved issue.
3. Pause and preserve the exact candidate, reports, gaps and count.

Preserve the real post-checkpoint reply and its conversation reference in the
same guidance history. Bind it to the guidance ID, round, candidate and named
items. If a choice is ambiguous or stale, request an unambiguous current reply.
Use the supplied checkpoint tokens to bind the preserved user message. Apply
each continuation once. On resume, validate the saved receipt and resume the
same selected round. A receipt supplies one round even if work stops before
another candidate is frozen.

If the recorded continuation covers a pending challenge on the frozen candidate,
first collect any due primary lanes required for that challenge. Dispatch only
the lanes advised by the router. Once the primary collection is complete, claim
the challenge on the same candidate and round. Apply the advisory decisions
within the continuation. Select an upheld repair once the required handoff is
complete and within the recorded scope. Existing gates retain their status.
If new binding issues invalidate the checkpoint identity, return to the user
for a current choice.
