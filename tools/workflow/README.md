# TA result and guidance validation

This small tool validates one invocation's structured handoff and renders matching
text, and validates the receiving session's separate guidance. It does not invoke
agents, run tests, manage rounds, authorize repairs, certify canonical evidence or
publish Git changes. A user's selection determines the next operation.

Prerequisite: `python -m pip install -r tools/workflow/requirements.txt` in the
selected environment, when installation is authorized. The migration environment
already supplied the dependency. No hook or model API is required.

```powershell
python tools/workflow/result.py --describe TA-plan-change
python tools/workflow/result.py .agent/tmp/workflow-TA/<task>/<item>/<run>/result.json --render
python tools/workflow/result.py .agent/tmp/workflow-TA/<task>/<item>/<run>/result.json
python tools/workflow/result.py --describe-guidance
python tools/workflow/result.py --next-skills TA-implement-change
python tools/workflow/result.py --matrix
python tools/workflow/result.py .agent/tmp/workflow-TA/<task>/<item>/<run>/guidance-01.json --guidance --render
```

[result.schema.json](result.schema.json) owns required fields, typed payloads and
role/skill pairing. `--describe` selects a schema view, not an operation to run.
Validation also checks round/status relationships, review prerequisites, reference
content hashes and portable paths. `--render` creates the text representation from
the same claims and refuses to overwrite a different existing report. Reconcile a
formatting error explicitly within the same invocation; never rerun expensive
successful checks merely to rewrite reports. Use a fresh run directory for a
continuation and preserve the previous result.

## Receiving-session guidance

Guidance schema version 2 binds the authoritative
[skill matrix](skill-matrix.json) by path and SHA-256. Every operation option has
`transition.condition` naming one code in the source-skill/destination-skill cell,
and `transition.basis` explaining the actual evidence for that condition. Conversation
options use a null transition. `--next-skills` reads one row and shows the conditions,
executor and reasoning effort for each possible destination.
`--matrix` renders all skill axes and cells into
[the readable matrix](../../docs/workflow-skill-matrix.md), grouped into
destination-column blocks. The result schema owns the executor, and the
[runtime tables](../../.agents/references/workflow-runtime.md#model-and-reasoning-effort)
own the effort. Main-session suggestions leave its settings up to the user.
Regenerate the view after changes to transitions, skill ownership or effort.

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

After reading the actual result, the receiver authors a guidance record using
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
it cannot support a ready operation. A finished task can have no options.

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
