# TA result and guidance validation

This small tool validates one invocation's structured handoff and renders matching
text, and validates the receiving session's separate guidance. It does not invoke
agents, run tests, manage rounds, authorize repairs, certify canonical evidence or
publish Git changes. A user's selection can cover one operation or the explicitly delegated planning and delivery sequences in receiving guidance.

Prerequisite: `python -m pip install -r tools/workflow/requirements.txt` in the
selected environment, when installation is authorized. The migration environment
already supplied the dependency. No hook or model API is required.

```powershell
python tools/workflow/result.py --describe TA-plan-change
python tools/workflow/result.py .agent/tmp/workflow-TA/<task>/<item>/<run>/result.json --render
python tools/workflow/result.py --describe-guidance
python tools/workflow/result.py --next-skills TA-implement-change
python tools/workflow/result.py --matrix
python tools/workflow/result.py .agent/tmp/workflow-TA/<task>/<item>/<run>/guidance-01.json --guidance --render
```

[result.schema.json](result.schema.json) owns required fields, typed payloads and
role/skill pairing. `--describe` selects a schema view, not an operation to run.
Validation also checks round/status relationships, review prerequisites, reference
content hashes and portable paths. `--render` also checks existing Markdown or
creates it from the same claims. It refuses to overwrite a different report.
Reuse one successful check for rendering and consumption at the same decision
if it verifies the required live references and matching text. After a delay
or changed input, recheck at the next decision that depends on those bytes. Reconcile a
formatting error explicitly within the same invocation; never rerun expensive
successful checks merely to rewrite reports. Use a fresh run directory for a
continuation and preserve the previous result.

## Receiving-session guidance

Guidance schema version 2 binds the authoritative
[skill matrix](skill-matrix.json) by path and SHA-256. Every operation option has
`transition.condition` naming one code in the source-skill/destination-skill cell,
and `transition.basis` explaining the actual evidence for that condition. Conversation
options use a null transition. `--next-skills` reads one row and shows the conditions,
executor, default model, effort and assignment condition for each possible
destination. `--matrix` renders the same settings and all cells in
[the readable matrix](../../docs/workflow-skill-matrix.md). The result schema
owns the executor. The
[runtime procedure](../../.agents/references/workflow-runtime.md#resolve-one-invocation)
owns defaults, crate-write selection and main-session prerequisites.
Regenerate the view after changing transitions, skill ownership or settings.

The validator enforces matrix identity and cell/code membership for official
operation suggestions, including proposals with missing prerequisites. It cannot
prove the caller's explanation or that a semantic condition holds. A missing source
result supports clarification, not invented operation recommendations. Check the
actual task and evidence under the receiving procedure before presenting options.
An empty matrix cell never blocks a user's direct skill request: standalone worker
results do not consult the matrix, and normal invocation permissions still apply.
Earlier consumed guidance remains historical under its original schema; do not
rewrite it or add a compatibility parser. Refresh choices using a new guidance ID.

Worker results use schema version 2. Their `recommendation` is an input to the
caller, with no dispatch or authorization meaning. Version 1's `next_action`
format is retired; historical results remain preserved historical evidence and
are not rewritten as new runs or accepted through a compatibility parser.

An ordinary draft edit needs no formal artifact. A completed formal plan with
no remaining choice keeps its result pair and a conversational reply.
For a selectable operation, a decision tied to the displayed version or a
handoff retained for resumption, the receiver authors guidance using
`--describe-guidance`. It binds the source result by path and hash, retains its
original candidate, and separately identifies the current candidate. The schema
and existing skill-to-role mapping both live in `result.schema.json`; there is no
second role registry. Keep guidance beside the source run's result under the
existing `.agent/tmp/workflow-TA/` tree.

An operation option supplies its executor, skill, identified input references,
selected finding IDs, read/write scope and round limit. Its `instruction` is the
short intent. The renderer constructs the full copyable conversational assignment
from those fields, including the source and candidate identities. It does not
accept a separately authored executable command that could disagree with the
structured assignment. Clarification, approval, stop and defer options have a
null assignment and use their conversational instruction directly. Neither a
made-up slash command nor a new execution CLI is provided.

Unavailable operations name missing inputs or the approval prerequisite and are
rendered as proposals, without an executable invocation. A source result that is
unavailable is represented by `source_result: null` with an identity limitation;
it cannot support a ready operation. Earlier guidance with no options remains
readable.

Guidance validation checks the source JSON and matching text, source identities,
option uniqueness, executor binding, finding IDs and continuation fields. It
does not rehash the archived source's old input references: changed input bytes do
not rewrite its historical claim. For ready operations it checks current input
hashes and compares the declared current candidate using read-only native Git:

```powershell
git diff --binary --no-ext-diff --no-textconv <base-commit> -- <scope-paths>
git ls-files --others --exclude-standard -z -- <scope-paths>
```

Hash the exact binary diff bytes with SHA-256 and include the current nonignored
untracked paths and their hashes within the declared scope. A stale candidate
requires fresh guidance, not rerunning successful tests. Ignored artifacts used
as inputs need their own explicit hash references. These comparisons do not
establish the adequacy of the chosen scope, claim freshness or permission to act;
the receiver still reviews those meanings.

Keep each consumed guidance revision immutable. A refreshed record uses a new
`guidance_id` and a `supersedes` reference; the receiving session explicitly names
the single current set. This is a conversational responsibility, not a current-set
registry. `--render` creates matching Markdown but refuses to overwrite different
existing text. Earlier options remain historical and cannot authorize stale work.
Before dispatch after a choice, the receiver rechecks applicability using the
[shared receiving procedure](../../.agents/references/workflow-guidance.md).

The receiver must still inspect claims, scope, authorization and canonical evidence.
A valid JSON object is not a trusted verdict or proof that commands executed. Input
hashes identify bytes available at consumption; if an old instruction changes,
revalidation can report that limit without erasing the original consumed result.
Candidate identity describes the selected snapshot; readiness requires comparison
against the current candidate under existing review and verification policy.

The [invocation contract](../../.agents/references/workflow-contract.md) describes
normal manual use and interruption. Runtime discovery and its limits are in
[the runtime reference](../../.agents/references/workflow-runtime.md).

## Review routing and human checkpoint

The receiving session records the actual required lanes and gates for the
selected scope. Run the read-only tool before choosing a lane or presenting
an operation. The table and lane effects come from
[review-routing.json](review-routing.json). Role and skill names come from
the existing result schema.

```powershell
python tools/workflow/review_routing.py --table
python tools/workflow/review_routing.py --describe
python tools/workflow/review_routing.py <review-request.json>
```

Keep the request and its portable hashed references under the work item's
guidance history. The request identifies the selected operation, the current
round and the exact reporting lane and local ID for each finding. The tool
qualifies those keys with task and work item. An assessment identifies the
owning criterion, disposition, impact and evidence. Link a duplicate directly
to its original qualified ID. Use the local ID for the source report only.

The round record links to its preceding round and the native binary Git diff
whose digest identifies its frozen candidate. The tool compares the native
tracked trees in disposable Git data and checks the recorded untracked hashes.
Preserve superseded round
snapshots when collecting later reports on the same candidate. The current
head is the latest actual state. On a correction, identify both round records,
the actual changed paths and evidence for the assessed effects. Unknown impact
selects every applicable lane. For a batch, include the canonical commit gate.
Use the paired verification result and the actual exit and report artifacts for
a passed or failed gate. If required evidence cannot be read with its recorded
identity, record the named gate and the unavailable artifact. At round three or
later the confirmed gap triggers the checkpoint. A confirmed missing current
review JSON or its paired text also triggers the checkpoint. The tool records
the missing file and gathers the other returned lane and gate results. An
inconsistent report identity suspends routing pending refreshed inputs.

Give each reviewer a hashed lane input that binds scope, requirements, criteria,
partition and applicable evidence. The receiving session records the independent
runtime context and whether it inherited author history. The review result links
those inputs in `result.routing`. The report's inspected scope covers the lane
input. Bind actual evidence to the fingerprint of the canonical candidate object,
available as `candidate_ref` in the advice. An authorized evidence-only round
may refer to the same qualifying report and context when all of its review
inputs still match. Credit that approval to its original reviewer. Finding
comparisons name the original trigger and evidence for each known outcome on
the specified candidate. When reading a past round, use its recorded review
and comparison outcomes. The plan and other files the reviewer read may have
changed or been removed. Check the current review against its live inputs.
When a current comparison claims a repair, verify the evidence it cites for
the original failure and the corrected behavior.
Apply the [readiness policy](../../.agents/rules/git-commits.md#readiness-before-freeze)
to the current review evidence.

After a stop, the tool returns the round, candidate and named issues without
lane recommendations or executable options. Use the
[human review checkpoint](../../.agents/references/workflow-guidance.md#human-review-checkpoint)
for the explanation and three conversational decisions. The receiver preserves
each request with its hashed round records. Before offering guidance and as
new reports or decisions arrive, the receiver atomically updates
`.agent/tmp/workflow-TA/<task-id>/<work-item-id>/review-routing/current.json`
with the latest request's `{"path": "...", "sha256": "..."}` reference. Publish
the updated reference even when the candidate bytes and round number are
unchanged. On resume, read that reference and its history before rechecking
any saved operation option.

Attach the same reference as `review_route_ref` in guidance. The validator
reads the current reference at the fixed work-item path, checks the source
against its linked history and recomputes the advice. A review source must
appear in a lane state or the round's `earlier_reports`. A challenge source on
the current candidate must appear in `challenge_report` or
`prior_challenge_reports`. A gate report may be referenced by its gate. The
round's `source_results` can hold hashed references to other sources on that
candidate, such as the repair result. Missing history or an older reference
suspends operation options.

Preserve the actual offer and user reply as hashed native conversation rows
from the same thread, with the session header and their ordinals. The offer
contains the following three selectable reply tokens for that guidance and
checkpoint. The preserved user reply selects one exact token.

```text
review <guidance-id> <checkpoint-id> accept
review <guidance-id> <checkpoint-id> continue-one
review <guidance-id> <checkpoint-id> pause
```

The receiver checks the conversation provenance and remains responsible for
actual user authorization. The tool checks the preserved role, message order,
choice, candidate and issues. For one additional round, the receiver records
the returned continuation at its canonical path with exclusive file creation
before dispatch. Preserve that receipt as `started_by` on the next round.
Replaying the same reply reports the recorded continuation and supplies no
additional round. Advice, validation and table rendering launch nothing.

### Challenge a collected review round

Run the review router with the current request. The output lists outstanding primary
lanes and required IDs. A due, running, failed or missing lane keeps the collection
pending. Before a human stop, the router recommends due lanes needed to finish
collection. At a stop it reports pending IDs without dispatching new work.
When a recorded continuation covers the pending challenge, collect the advised
primary reports still due, then claim the challenge on the same candidate and
round.

To reserve a ready challenge, invoke `python tools/workflow/review_routing.py REQUEST --claim-challenge`.
Only the successful reservation starts a fresh reviewer. Resume its recorded dispatch
after interruption. The immutable collection beside the round binds reports, finding
IDs, decisions and candidate. Attach the resulting paired report as `challenge_report`
in the request. The result uses `uphold` or `reject` per lane and ID. The returned
advice lists them as `eligible_repairs` and `rejected`. Each `resolved_lanes` entry
links the original completed primary report and the advisory evidence that
rejected its applicable required findings. The `approvals` entries still identify
actual primary approvals. When only gate evidence changes and the candidate,
primary reports and finding evidence are reused, carry the recorded rejection
and resolved lane forward. A commit option requires routing phase `done` and
the ordinary batch prerequisites.

Supply material new evidence for affected IDs in `risk_decisions` with fields
`finding` and `evidence`. Preserve earlier returns in `prior_challenge_reports`
in dispatch order for a linked collection of affected IDs. Earlier linked rounds
can preserve why a rejected ID closed on its earlier candidate. Retain decisions
for unaffected IDs and resume an open reservation with its original dispatch.
A changed candidate still requires its applicable primary review and gate evidence. The
[receiving procedure](../../.agents/references/workflow-guidance.md#challenge-required-findings)
owns the finding handoff and the human checkpoint.

### Run a command for a selected execution

From the selected checkout run `python tools/workflow/run_selected.py --registration
PATH --work-item ITEM --round N -- COMMAND` on one line. Give every implementer,
reviewer and verifier the same authoritative registration path for this execution.
The launcher recovers the task and execution pair from that specific receipt,
checks conflicting process bindings before starting the command, and supplies its
child environment. Reports keep their native outcome and parent invocation links.
Use the assembly execution for combined verification and retain its source manifest.
The local receipt and archive work with the receiver offline. Direct ad hoc commands
outside a selected plan retain the reporter's standalone behavior.
