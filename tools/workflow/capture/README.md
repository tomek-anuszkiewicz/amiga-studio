# Capture a selected task execution

The primary checkout keeps private evidence in `.agent/task-archives/<task>/<execution>/`.
An execution groups expressly selected turns and verified descendant threads. The
workflow's task, work item, run and round identify an assignment within it.
Codex's thread ID joins the rollout to the OTLP `conversation.id`. The CLI pilot
also found `turn.id` on selected spans. Preserve the separate source fields.

## Starting inputs

Use `TA-capture-environment` to record the actual environment before implementation.
From the task checkout, run the environment command with a fresh destination.
Select the known revision that governs execution and the relevant local tools. Identify the actual shell with `--shell <name>` and inspect a locally available Collector image with `--docker-image <reference>`.
To record observed client settings, supply `--rollout <selected-session.jsonl> --turn-id <observed-turn-id>` to the capture command. Without that source the inventory reports the client observation as unavailable. The executable probe records the version of Codex found on `PATH`. Use `--node-project <repository-relative-directory>` for each selected local Node project. The comparison operation takes two saved inventories.

```text
python tools/workflow/capture/environment.py capture --output <new-inventory.json> --governing-commit <revision>
python tools/workflow/capture/environment.py compare <saved-inventory.json> <fresh-inventory.json> --output <differences.json>
```

Keep the editable approved inputs under `.agent/tasks/` or another selected
ignored location. At approval, create a version map under
`.agent/tmp/task-inputs/<task>/` and copy each selected starting file byte for
byte under `start/` before editing. Use ordinary file copy and SHA-256 commands.
Check the source and destination digests and preserve any existing copy if they
differ. Do the same under a new named version directory before replacing each
selected substantive revision. Verify again before resuming a task.

For every version, record the task, original relative path, version, approval or
run identity, snapshot path and SHA-256. Record the selected final version and
its links to the starting and intermediate versions. Identical bytes may use a
single preserved copy. Record unavailable earlier versions as gaps. Keep any
historical source locators and the limits of archive-only inputs in the map.
For a separately authorized Git input commit, record the commit and original
blob locators and verify reachability under the
[commit policy](../../../.agents/rules/git-commits.md#approved-execution-inputs).

## Select the interval

Invoke the registration helper **inside the chosen thread**, with the selected
task checkout as its working directory. Pass the surviving primary checkout
to `--project`. A linked worktree has the same Git common directory. A separate
clone requires an independent selection. The primary checkout must already
ignore its archive. Use a unique execution ID for each trial. Start without
`--otel` to register native records and explicitly selected artifacts. Add
`--otel` to `start` when selecting a receiver.

```text
python tools/workflow/capture/register.py start --project <primary> --task <task> --execution <execution>
python tools/workflow/capture/register.py include --project <primary> --task <task> --execution <execution>
python tools/workflow/capture/register.py stop --project <primary> --task <task> --execution <execution> --at <covered-task-complete-utc> --turn-id <covered-turn-id>
```

To exclude the complete setup turn, add `--after-turn <setup-turn-id>` to
`start`. After its final answer, run the following from the project shell before
launching the first covered turn. The helper verifies the completion event for
the saved root thread and arms the interval immediately after its timestamp.

```text
python tools/workflow/capture/register.py arm --project <primary> --task <task> --execution <execution>
```
Use the actual thread's
`CODEX_THREAD_ID` and record the selected boundary in UTC, with the turn ID when
available. Include a continuation only when explicitly selected. For a stopped
or failed task, retain the actual last included event. If it is the final answer,
read its native `task_complete` record after the CLI exits. Resume the selected thread for a control turn. If OTLP was enabled, disable the process-local log and trace exporters for that turn. Pass the covered turn ID and its terminal timestamp to `stop`, then collect. If the receiver has already admitted later telemetry, it refuses that historical stop and leaves the range open. Record the discrepancy and choose the actual last included event before retrying. Stopping with the current time includes the interval up to the control command.

With OTLP selected, wait for a successful registration receipt before covered
work. Configure the dedicated CLI process to use its returned local endpoint.
For example, pass these **separate** `-c` arguments to `codex exec` or its
`resume` command, substituting the listener port. Preserve the JSONL from
`--json` as bytes. This trial uses OTLP/HTTP JSON.

```toml
otel.exporter = { otlp-http = { endpoint = "http://127.0.0.1:<port>/v1/logs", protocol = "json", headers = {} } }
otel.trace_exporter = { otlp-http = { endpoint = "http://127.0.0.1:<port>/v1/traces", protocol = "json", headers = {} } }
otel.metrics_exporter = "none"
otel.log_user_prompt = false
```

The Collector serializes accepted records to files. Preserve the config, image
identity, source IDs and a record of any data removed or encoded again. Logs can
include private tool arguments and outputs even when prompt logging is disabled.
Use the standard Collector with a fixed ID filter for a dedicated bounded probe.
Routine registration uses the local intermediary when active selections or child
threads change. It checks individual records against the registry before passing
them to the Collector. The probes retained three identifiable spans and no
identifiable metric points in a separate short CLI turn. The IDE needs its own
delivery evidence before relying on its configuration.

### Windows host and CLI

The running user must be able to verify both the selected Git repository and
the archive ignore rule. On the tested Windows installation the CLI's
`workspace-write` helper used a separate OS identity. Git allowed access to its
own working checkout and denied the primary checkout when invoked from a linked
worktree. The driver also could not start Docker from that isolated process.

When using a receiver in that setup, start it under the repository owner's
account and run the registration control turn from the primary checkout. This
combination was
exercised with a real child, terminal CLI usage records and a stop in a later
control turn. The foreground service entry point accepts the same Collector
image used by the registration command. Use a second shell for the client.

```text
python tools/workflow/capture/receiver.py --project <primary> --codex-home <selected-codex-home> --port 4318 --upstream-port 4319 --collector-image <locally-available-pinned-image>
```

Make the pinned image available locally before launch. The tested image is `otel/opentelemetry-collector-contrib@sha256:e40d61f10c9a617cca1ffd1be4d7e2b3a4b0f0001798ff07e58cabe82e618cea`.
When using custom ports or an image override, pass the same values to `start`.
Registration with `--otel` reuses a matching project service. The receiver reports
readiness and rejects a conflicting service or active selection at shutdown.

## Capture a stage

Record the task, work item, run, round and stage alongside the available thread,
turn and call identities. The result envelope accepts this association under
`observations.capture`. Save the actual paired results and separately produced
verification reports as selected artifacts, including the status and complete
output of a failed attempt. Use a scoped Git diff at the stage boundary if
native patch events omit the required content. Include newly created files and
binary inputs separately when the diff does not preserve their bytes. A script
that writes and restores a file inside one command can finish with an empty
stage diff. The accepted history records tool events and observable changes at command and stage boundaries. Internal writes that a script restores without emitting their contents remain unobserved.

## Collect and assess

Use `TA-collect-task-records` to create a private manifest revision and
`TA-check-task-records` to assess it against the observer's expected evidence.
Keep inputs for those commands outside the evidence they evaluate.

```text
python tools/workflow/capture/records.py collect --primary <primary> --task-id <task> --execution-id <execution> --input <collection-input.json>
python tools/workflow/capture/records.py check --primary <primary> --task-id <task> --execution-id <execution> --revision <revision> --expect <observer.json>
```

The collection input identifies `start_commit`, `workflow_commit`,
`final_commit`, `final_candidate`, `outcome` and `client`. Use the actual starting
code commit for `start_commit`, the governing workflow commit for
`workflow_commit`, and the published commit for `final_commit` when available.
Record archive-only input limits in `initial_input_limits`.

The `files` list selects explicitly located CLI streams, OTLP segments and
artifacts. Use `kind: cli`, `otel` or `artifact` with destination paths under
`cli/`, `otel/` or `artifacts/` respectively. Source paths are relative to the
input file by default. For linked worktree sources list narrow directories in
`artifact_roots` and give each selected file's full source path. Include the
version map and individually selected copies under `artifacts/worktree/inputs/`,
with input bytes under `<version>/<relative-path>`. Within each version retain
relative subpaths to distinguish duplicate filenames. Use
`artifacts/verification/<profile>/<attempt>/` for the selected timing report and
its selection record. Give the independently acquired expected `sha256` for
each selected input or report. Keep
unrelated workspace data and credentials outside this list. Closed registration
records supply the native time and thread boundaries with or without OTLP.
Read the collection receipt's `status` and `gaps` even when the command exits
successfully. A partial revision can still preserve useful evidence. The manifest inventories
the selected files and their digests. Preserve earlier revisions when a producer
finishes a partial record or late telemetry arrives.

An independent observer records the expected starting inputs, registration and
lineage, communication, tools and skills, timing and progress, usage and
verification, change history, and integrity and survival. For each row give a
source locator, status and the contribution from telemetry. Missing attribution
and unavailable sources limit the verdict. Record any accepted gap separately
from the software outcome, verification status and backup availability.

If the execution used a receiver, shut it down after all active executions have
closed and the client has flushed its exporters. Use
`register.py shutdown --project <primary> --port <port>`.
Collect again if new eligible records arrived. Record the checked revision,
manifest digest, checker report and the effect of any telemetry gaps. Before
retiring a task worktree, verify the required surviving byte copies and Git
ancestry from the primary checkout.


For example, keep the CLI file beside the input JSON and select a timing report
and its selection record from a task worktree. Use the SHA-256 from the verifier
for the report and the digest of the saved selection record for that record.
Supply actual commit IDs and a terminal task outcome.

```json
{
  "start_commit": "<starting-code-commit>",
  "workflow_commit": "<governing-workflow-commit>",
  "final_commit": null,
  "final_candidate": "<identity of the reviewed working tree>",
  "outcome": "completed",
  "client": "<observed CLI version>",
  "artifact_roots": ["<absolute attempt directory>", "<absolute selection record directory>"],
  "files": [
    {"kind":"cli","source":"covered.jsonl","path":"cli/covered.jsonl","turn_id":"<observed-turn-id>"},
    {
      "kind": "artifact",
      "source": "<absolute attempt directory>/test-runtime-report.md",
      "path": "artifacts/verification/commit/<attempt>/test-runtime-report.md",
      "sha256": "<producer-digest>"
    },
    {
      "kind": "artifact",
      "source": "<absolute selection record directory>/selection.json",
      "path": "artifacts/verification/commit/<attempt>/selection.json",
      "sha256": "<selection-record-digest>"
    }
  ]
}
```

Pass an observer JSON object with `identity`, the expected `threads` list, and
`rows` keyed by `starting_inputs`, `registration_lineage`, `communication`,
`tools_skills`, `timing_progress`, `usage_verification`, `change_history`, and
`integrity_survival`. A row can carry independent `checks` and known `gaps`, or
an explicit reason under `not_exercised` or `unavailable`. An exact file check
uses the expected archive-relative path and the digest of its independently held
source, for example `{"kind":"archive","path":"cli/covered.jsonl","sha256":"<digest>"}`.
Use `git_blob` for committed approved input bytes, `commit_ref` for required
ancestry and `source_range` for selected original JSONL byte ranges. Check exits
with status 1 when the report contains gaps.

## Selected timing reports

When accepting a final verification profile, save a fresh task-local selection
record from the actual completion JSON. Record both final profiles when both were
required. Select a failed attempt or a diagnostic only when explicitly required,
using its actual status and, for diagnostics, exact invocation. Record explicitly
when the task had no report-producing verification.

The selection record identifies the task, profile, attempt, reviewed candidate,
actual outcome and the source checkout-relative report path. Take that path from
the completion record's results directory and `timing_report.path`. Take
`sha256` from `timing_report.sha256`, or the verifier's console for a selected
diagnostic. Keep the selected record with the attempt evidence. Later completion
records describe later attempts, including attempts that reuse all groups.
A record without the producer path or digest cannot establish an archival copy.

For a registered execution, include the selection record and report in the
collection above. For an unregistered execution, copy just the named report and
receipt using the explicit surviving primary and source checkouts:

```text
python tools/workflow/capture/records.py preserve-timing --primary <primary> --source-checkout <task-checkout> --task-id <task> --input <selection.json>
```

```json
{
  "task_id": "<task>",
  "profile": "commit",
  "attempt": "<attempt>",
  "candidate": "<verified-tree>",
  "outcome": "passed",
  "path": ".agent/tmp/verification/commit/verify/<attempt>/test-runtime-report.md",
  "sha256": "<producer-digest>"
}
```

Use profile `milestone` for that completion profile. A diagnostic uses profile
`diagnostic`, also supplies `invocation`, and uses its invocation directory
name as the attempt. Supply the actual candidate identity and outcome for either
route. The source checkout may also be the primary.

The copy procedure checks Git identity and the ignored source and destination
paths. It hashes the selected bytes, writes the report and receipt to a temporary
directory and publishes them together into
`.agent/task-archives/<task>/.timing-reports/<attempt>/`. It reuses only an
identical complete entry. On collision, unreadable source or hash mismatch, keep
the originals for resolution. Verify the receipt and report digest from the
primary before cleanup. This receipt preserves one report. If unregistered input
preservation is also required, retain the task checkout and snapshots pending
an explicit disposition.
