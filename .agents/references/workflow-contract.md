# One bounded invocation

The caller selects one role, one skill and one outcome. Apply the
[runtime settings](workflow-runtime.md#model-and-reasoning-effort) before dispatch.
Announce task, work-item, run ID, read/write scope, selected findings, required
checks and round limit.
Default to three rounds; a declared limit may be one to four, never more than four.
Count the first pass. A round is an implementation/repair pass with its checks,
a substantive planning revision, or one scoped investigation pass. Announce each
further pass and its reason. Compaction, retries and resume do not reset the count.
Stop at success, missing input, scope expansion, no justified progress, unrecovered
failure or the limit. Do not consume spare rounds after success or delegate
recursively. A continuation is a new user-authorized run (or one covered by an
explicit recorded delegation to the caller) linked to its predecessor,
with a concrete new approach and cumulative work history. Recommendations grant
no authorization. A failed prerequisite never selects another skill.

Read only the selected operation, relevant advance requirements and needed source
definitions. Review-owned criteria remain acceptance standards; authors consult
them when relevant, including selected repairs, rather than loading all lanes.
Fresh reviewers must not inherit the author's conversation. Higher-priority runtime
instructions remain binding. Record inherited guidance separately when observable.

For cumulative review rounds, apply the receiving [human decision checkpoint](workflow-guidance.md#human-review-checkpoint)
before any continuation, including work delegated for autonomous execution.

## Telemetry startup prerequisite

Before dispatching planning for a new task or starting execution of an existing
plan, the main session identifies the task, execution and actual Codex thread.
Follow [capture startup](../../tools/workflow/capture/README.md#confirm-telemetry-before-work)
and obtain confirmation of a fresh archived OTLP record from that thread. A
registration or a healthy endpoint alone does not satisfy this prerequisite.
Give the confirmed execution and receipt to the selected operation. Until
confirmation, limit work to startup and diagnosis within the recorded deadline.
If export requires a completed turn, run a short control turn and resume
confirmation with the same registration and deadline. Report the failed stage
when startup cannot be confirmed. Preserve the existing capture after startup.

## Inputs and outputs

The assignment supplies requirements/plans, authorized operations, exact candidate,
read/write scope, evidence, selected findings and configuration/instruction identities.
Use repository-relative slash paths and SHA-256 file content hashes. For candidates,
record base commit, SHA-256 of the selected binary Git diff and hashes of selected
untracked files, plus scope. A branch or HEAD alone cannot identify dirty work.
State unavailable identities explicitly; they cannot support approval.

Each invocation persists fresh `result.json` and matching `result.md` under
`.agent/tmp/workflow-TA/<task-id>/<work-item-id>/<run-id>/`. Never overwrite a prior
run. Plans belong under `.agent/tasks/`. The receiving session may persist a
read-only worker's returned data, preserving its claims and identity.
The ordinary `records.py preserve` copy under TA-collect-task-records reports its
observed outcome directly. It uses the existing publication control record for
closure state and needs no result pair. This exception applies only to the
ordinary copy. Separately selected archive analysis retains its typed result.

[The schema](../../tools/workflow/result.schema.json) defines the envelope, typed
payloads and skill/role pairing. It is the result-format owner, not a dispatcher.
Read only the common envelope and the selected payload, using
`python tools/workflow/result.py --describe <TA-skill>` for the relevant schema.
The specialist supplies JSON; generate and validate its text using
`python tools/workflow/result.py <result.json> --render`, then validate both before
consumption. Missing validator dependencies return a prerequisite gap; installation
requires a separately supplied authorization. Formatting repair does not rerun
successful checks.
The human report includes the outcome, invocation/rounds, typed details, findings,
checks and remaining decisions. The worker supplies a non-authoritative
`recommendation`, not a menu or permission decision. The receiving main session
automatically applies [result interpretation and next-step guidance](workflow-guidance.md),
preserving this result and presenting one source-bound option set. It waits for
the user's choice before another operation; validation/rendering never dispatches.
Official operation suggestions come from the source skill's conditional matrix row
under that receiving procedure. The matrix does not restrict a user's direct skill
selection or change the selected skill's authority, required inputs or round limits.

Statuses are `completed`, `needs_input`, `limit_reached`, `failed`, `interrupted`.
They describe this operation, not batch readiness. A completed assessment may
request changes; an unfinished assessment cannot approve. Findings keep stable IDs,
expected/observed behavior, evidence and disposition across repairs. A repair claim
is `repaired_pending_verification`, not `verified`. Deferred required findings still
block readiness. Unknown cause remains unknown. Only canonical tool evidence can
establish canonical verification; JSON links it, never substitutes for it.

Separate newly executed, retained and inspected evidence. Record actual command,
exit status, artifact identity, candidate and validity basis. Unknown time, model,
configuration or token use is null/unavailable; never invent it. Report waiting and
overlap honestly, and give a concrete reason for repeating expensive work.

## Evidence for a selected execution

Use the optional `observations.capture` object when capture is active. Keep the
exact result pair and source task, work item, run, round and stage with observed
thread, turn and call IDs. At an edit boundary, preserve a scoped diff or report
only when native events omit the required evidence. Identify reversals and
unknown attribution explicitly. Use the archive tooling from the
[capture guide](../../tools/workflow/capture/README.md) at the selected boundaries.

## Runtime limits and interruption

Round and write boundaries are behavioral instructions, not hard runtime guards.
Custom roles inherit parent permissions; a read-only role label or TOML sandbox
field is not proof of enforcement. Use a separately read-only parent context when
available and appropriate; otherwise report inherited access and obey the assigned
write boundary. Do not change global permissions or imply sandbox isolation.

After termination the receiver records interrupted/missing output without inventing
a worker verdict. Inspect surviving commands and actual edits before conflicting
work. A late result cannot approve changed input. Keep unresolved findings and
continue only independently authorized work. No process manager, workflow engine,
automatic next phase or hard budget enforcement is supplied by this contract.
