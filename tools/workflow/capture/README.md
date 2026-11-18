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

Prepare the approved plan, design, decisions, attachment contents and inventory
under the [starting-commit policy](../../../.agents/rules/git-commits.md#approved-execution-inputs).
Preserve the resulting SHA in the collection input. The implementation commit
retains this parent in the task's history.

## Select the interval

Invoke the registration helper **inside the chosen thread**, with the selected
task checkout as its working directory. Pass the surviving primary checkout
to `--project`. A linked worktree has the same Git common directory. A separate
clone requires an independent selection. The primary checkout must already
ignore its archive. Use a unique execution ID for each trial.

```text
python tools/workflow/capture/register.py start --project <primary> --task <task> --execution <execution> --otel
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
read its native `task_complete` record after the CLI exits. Resume the selected thread for a control turn with the process-local log and trace exporters disabled. Pass the covered turn ID and its terminal timestamp to `stop`, then collect. If the receiver has already admitted later telemetry, it refuses that historical stop and leaves the range open. Record the discrepancy and choose the actual last included event before retrying. Stopping with the current time includes the interval up to the control command.

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

In that setup, start the receiver under the repository owner's account and run
the registration control turn from the primary checkout. This combination was
exercised with a real child, terminal CLI usage records and a stop in a later
control turn. The foreground service entry point accepts the same Collector
image used by the registration command. Use a second shell for the client.

```text
python tools/workflow/capture/receiver.py --project <primary> --codex-home <selected-codex-home> --port 4318 --upstream-port 4319 --collector-image <locally-available-pinned-image>
```

Make the pinned image available locally before launch. The tested image is `otel/opentelemetry-collector-contrib@sha256:e40d61f10c9a617cca1ffd1be4d7e2b3a4b0f0001798ff07e58cabe82e618cea`.
When using custom ports or an image override, pass the same values to `start`.
Each normal registration reuses a matching project service. The receiver reports
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
`final_commit`, `final_candidate`, `outcome` and `client`. Its `files` list
selects explicitly located CLI streams, OTLP segments or other artifacts. Use `kind: cli`, `otel` or `artifact` with destination paths under `cli/`, `otel/` or `artifacts/` respectively. By default, source paths are relative to the input file. To preserve a report from a linked worktree, explicitly list its report directory in `artifact_roots` and supply the report's full source path in `files`. Use a narrow directory containing the selected reports. Keep
unrelated workspace data and credentials outside this list. Closed registration
records supply the native time and thread boundaries. Read the collection receipt's `status` and `gaps` even when the command exits successfully. A partial revision can still preserve useful evidence. The manifest inventories
the selected files and their digests. Preserve earlier revisions when a producer
finishes a partial record or late telemetry arrives.

An independent observer records the expected starting inputs, registration and
lineage, communication, tools and skills, timing and progress, usage and
verification, change history, and integrity and survival. For each row give a
source locator, status and the contribution from telemetry. Missing attribution
and unavailable sources limit the verdict. Record any accepted gap separately
from the software outcome, verification status and backup availability.

Shut down the receiver explicitly after all active executions have closed and
the client has flushed its exporters. Use `register.py shutdown --project <primary> --port <port>`. Collect again if new eligible records arrived. Before
retiring a task worktree, verify that a surviving branch reaches the starting
commit and that the required evidence is readable from the primary checkout.


For example, keep the CLI file beside the input JSON and select one report
from a task worktree. Supply actual commit IDs and a terminal task outcome.

```json
{
  "start_commit": "<approved-input-commit>",
  "workflow_commit": "<governing-workflow-commit>",
  "final_commit": null,
  "final_candidate": "<identity of the reviewed working tree>",
  "outcome": "completed",
  "client": "<observed CLI version>",
  "artifact_roots": ["<absolute task report directory>"],
  "files": [
    {"kind":"cli","source":"covered.jsonl","path":"cli/covered.jsonl","turn_id":"<observed-turn-id>"},
    {"kind":"artifact","source":"<absolute task report directory>/result.md","path":"artifacts/result.md"}
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
Use `git_blob` for approved input bytes and `source_range` for selected original
JSONL byte ranges. Check exits with status 1 when the report contains gaps.
