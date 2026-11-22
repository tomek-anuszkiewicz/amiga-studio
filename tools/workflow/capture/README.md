# Capture a selected task execution

The primary checkout keeps private evidence in `.agent/task-archives/<task>/<execution>/`.
An execution groups expressly selected turns and verified descendant threads. The
workflow's task, work item, run and round identify an assignment within it.
Codex's thread ID joins the rollout to the OTLP `conversation.id`. The CLI pilot
also found `turn.id` on selected spans. Preserve the separate source fields.


## Case reports

Test and benchmark launchers reserve `test-runs/<UTC-milliseconds>_<invocation-id>.md`
within the task and execution archive. Standalone runs use
`.agent/task-archives/standalone/<execution>/test-runs/` with a fresh execution
ID. Both routes use the primary checkout and work without a receiver. Execute
commands directly; the selected-command wrapper has been removed.
When the task context is known, pass `AMIGA_TEST_TASK_ID` and `AMIGA_TEST_EXECUTION_ID`.
Launchers propagate parent invocation linkage and accept
`AMIGA_TEST_WORK_ITEM_ID` and `AMIGA_TEST_ROUND` when supplied.

Open the printed Markdown path and follow the links to its native journal,
logs and child reports. The journal persists case events during execution.
A missing finish leaves the invocation's final outcome unavailable. A started
case without a result has an unavailable duration and outcome. Reused groups
identify the earlier evidence and add zero new attempts.

## Starting inputs

Use `TA-WF-Step-capture-environment` to record the actual environment before implementation.
From the task checkout, save the JSON inventory in the primary checkout at
`.agent/task-archives/<task>/<execution>/logs/environment.json`, using the task's
existing IDs. Create the parent directory first. If that filename exists, use a
fresh name in the same directory and preserve the earlier inventory. Without
task context, use an explicitly selected archive location. This file stays with
the task's session logs, telemetry and test reports without requiring a running
Receiver or Collector.
Select the known revision that governs execution and the relevant local tools.
Apply the [capture skill's governing revision rule](../../../.agents/skills/TA-WF-Step-capture-environment/SKILL.md).
Identify the actual shell with `--shell <name>` and inspect a locally available
Collector image with `--docker-image <reference>`.
To record observed client settings, supply `--rollout <selected-session.jsonl> --turn-id <observed-turn-id>` to the capture command. Without that source the inventory reports the client observation as unavailable. The executable probe records the version of Codex found on `PATH`. Use `--node-project <repository-relative-directory>` for each selected local Node project. The comparison operation takes two inventories saved in the current format (version 2).

```text
python tools/workflow/capture/environment.py capture --output <primary>/.agent/task-archives/<task>/<execution>/logs/environment.json --governing-commit <revision>
python tools/workflow/capture/environment.py compare <saved-inventory.json> <fresh-inventory.json> --output <differences.json>
```

Keep approved task-specific plans, test designs, acceptance criteria, decisions
and attachments under `.agent/tasks/` or another selected ignored location.
Use a version map under `.agent/tmp/task-inputs/<task>/` for approved inputs,
selected handoffs and versions explicitly retained for history. Include each
attachment the decision or recipient actually depends on. Working draft edits
need no intermediate versions.

At approval, bind each selected input to recoverable bytes. Use the path at an
immutable Git commit if it contains the exact bytes. For ignored or dirty inputs,
keep one verified copy before changing the selected bytes. Use ordinary file
copy and SHA-256 commands and verify the destination against the source.
Reuse an existing verified copy of identical content. Keep previous accepted
versions and existing archive records. If the recipient needs a worktree copy,
reference its existing provisioning copy when it provides the selected bytes.

For each selected version, record the task, original relative path, version,
approval or handoff identity, SHA-256 and the copy or Git object locator.
At a decision that depends on an earlier version, compare its selected inputs
once at use. Record missing required versions as gaps. A hash without recoverable
bytes cannot supply a comparison. When asked what changed since approval, read
that approved input and compare it with the current working file.
### Explicit starting input commits

An explicitly selected starting input commit may retain named ignored paths
in Git. Apply [ordinary commit preparation](../../../.agents/skills/TA-WF-Main-commit-changes/references/commit-preparation.md)
and verify recovery from that commit. Preserve its reachability from delivered
history and a surviving authorized branch. For either retention route, record
governing workflow, starting code, final code and reviewed candidate identities
separately. For Git retention also record the original blob locators.

## Confirm telemetry before work

Use the primary checkout for storage even when invoking the procedure from a
linked worktree. Run the commands under the host account that can access Docker,
the selected Git checkout and the Codex sessions. The primary checkout must
ignore the entire archive and contain no tracked files there. Use unique task
and execution IDs. For a new task, preserve those IDs in the plan after startup.

### Provision the shared services and configure Codex

In Docker Desktop on Windows, enable startup at sign-in and use the Linux
container engine. Provision once with Docker running. Supply the actual
Codex user directory if different from the tool's default. It needs the
`sessions` and `archived_sessions` directories, even if the archive is empty.

```text
python tools/workflow/capture/register.py provision --project <primary>
```

Compose publishes the receiver on `127.0.0.1:4318` and keeps the Collector on the
private project network at `http://collector:4318`. Both use `unless-stopped`.
The primary archive stores task registrations and each execution's Collector output.
At startup, the receiver reads registrations into a thread index in memory.
Each successful registration change or admission timestamp write updates that
index after the file is saved. Incoming log and trace requests use the index
without scanning registration files again, including for closed intervals and
delayed events. Registration commands still validate the selected files on disk.
Direct archive edits require a receiver restart to rebuild the index.
The primary checkout's ignored, untracked `.agent/tools/workflow-capture/`
stores private Compose settings, deployed service files, the host's Git
validation, Receiver state in `receiver/` and Collector writes in `collector/spool/`.
The settings file stays on the host. The receiver mounts its code, the Git
validation directory and the two required Codex session directories read-only,
alongside the writable archive, Receiver state and Collector spool. Mounting the Git validation
directory lets a running receiver pick up the host's atomic updates when a
linked worktree is discovered. The Collector mounts its configuration
read-only and writes only to the dedicated `collector/spool/` directory.
The receiver publishes complete Collector writes as individual JSONL files
under the matching execution's `otel/` directory. It checks the output route,
keeps the original bytes, and retries complete spool files after a service restart.
Existing `logs.jsonl` and `traces.jsonl` files remain readable.

#### Update a host-side capture installation

From the published primary checkout, inspect the installation before choosing a
maintenance window. The report gives the installation and port, saved and
published revisions, open task and execution IDs with their root threads, and
the current state of each capture container and Receiver/Collector readiness.
Inspection leaves the containers and registry untouched. When either service is
missing, stopped or unavailable, the report includes `ready: false` and the
command exits with status 1. Publication itself starts no service action.

```powershell
python tools/workflow/capture/register.py status --project .
```

Use the private host account that provisioned capture. In the agreed window,
start any stopped installed Receiver or Collector container through Docker
Desktop. Run status to confirm both services are ready. During normal task
startup, `start --otel` restores missing or stopped services from the saved
installation before registering the task. Maintenance requires both services
to be ready before closing existing registrations or upgrading.

```powershell
python tools/workflow/capture/register.py status --project .
```

Have each registered owner end the selected interval at its actual last
included time. Supply the observed turn ID when available. A task in one
session needs one bounded interval. Include a continuation only when explicitly
selected. Keep both services available while exporting any remaining records
and processing delayed writes.

If the report shows "control: http", use the [normal stop command](#continue-and-close-an-interval)
from the selected thread. Registration uses authenticated loopback HTTP at
the installed revision even when the checkout has changed. The exporter
credential alone grants no registration authority.

For the first cutover only, a report of "control: legacy" selects the
pre-REST procedure. Run the following command from the selected root or
continuation thread after both old services are ready. It validates the private
saved code and installed container ownership before running that saved
controller in the exact receiver container.

```powershell
python tools/workflow/capture/register.py stop --legacy-installed --project . --task <task> --execution <execution> --at <covered-utc> --turn-id <covered-turn-id>
```

Stop on authentication or ownership errors. After an uncertain HTTP reply,
inspect the selected registration before retrying. Use the legacy flag only
for a verified snapshot that predates the HTTP API.

Once all owners have closed their intervals, check the status report again.
Let the exporters flush and verify both old services are ready before applying
the published revision.

```powershell
python tools/workflow/capture/register.py status --project .
python tools/workflow/capture/upgrade.py --project .
python tools/workflow/capture/register.py status --project .
```

The upgrade verifies the installed revision and recreates only `receiver` and
`collector`. It retains their Compose project name, port, credentials, spool,
registry and archives. Other services in the same Compose project keep running.
It checks readiness before accepting the new revision. If the switch fails,
the command attempts to restore the previous capture revision and verifies that
it is ready. Recovery gives Docker commands and readiness checks a fresh shared
180-second deadline. Copying pending output back runs synchronously and is not
interrupted by that deadline; its elapsed time reduces the budget available for
restarting the previous services. Time spent on the failed upgrade does not consume
the recovery budget. An installation at the selected
revision returns `already_current` after readiness and any pending old-layout
retention have completed.

After a successful upgrade, [confirm a fresh event](#register-and-read-the-fresh-event)
from the actual Codex client using a new proof execution. Close its registration
after confirmation. Retain the upgrade, status and proof results with the maintenance record.

If an interruption leaves the previous revision in the saved settings and the
services unavailable, keep the private service directory. From the same source
checkout and maintenance window, restore the revision recorded in the settings.
The command checks the old snapshot and accepts only containers belonging to
the two known revisions of this deployment.

```powershell
python tools/workflow/capture/upgrade.py --project . --rollback
```

Record the rollback outcome. Confirm a fresh event from the restored revision
and close the proof interval. For a restored pre-REST revision, use its matching
saved controller during the agreed window.

A terminated controller can leave an empty `receiver/control-mutation`
directory in the private service root. Capture control and upgrades report
busy while it exists. Verify the previous operation has ended and no other
host controller or receiver request is in flight. With the operator's
approval, remove only that empty directory before retrying the selected
operation. For an installed archive layout, the marker remains at
`.agent/task-archives/_receiver/control-mutation` until migration.

#### Move service state out of task archives

The installed snapshot determines the active runtime paths. Host commands keep
using the previous `_receiver/` paths while that snapshot is installed. A new
installation uses the service directories below immediately.

```text
.agent/tools/workflow-capture/
  compose-settings.json
  compose/<revision>/
  authority/
  receiver/                 # lineage cache, locks and control state
  collector/spool/          # pending Collector writes
```

Use the normal upgrade command in the agreed maintenance window after all task
registrations are closed. Pause other capture controllers until it finishes.
The upgrade claims both old and new control directories, stops only the two
capture services, copies the lineage cache and pending Collector writes, and
recreates the services with their new mounts. It retains the Compose project,
port, credentials, image and existing task archives. The archive directories for
individual tasks and executions keep their existing paths.

Each copied runtime file is fully written and closed in an ignored temporary file
outside the destination runtime directory, on the same filesystem. Only then is it
published under its final name by an atomic rename that refuses to overwrite an
existing entry. An interrupted write can leave a temporary file outside the queue;
it leaves no incomplete queue input and preserves the original. A platform or
filesystem without the required publication operation fails rather than overwrites.

After the new services are ready, the old `_receiver/` directory moves to
`previous-layout/<old-config-id>/receiver/` under the private service root.
This host-only copy retains historical files and credentials without mounting
them into either container. Keep it for recovery. A conflicting destination stops
the operation and preserves both copies. An ordinary failed switch restores the
old services and returns pending new spool files to the old spool before restart.
For an interrupted switch, use the documented `upgrade.py --rollback` operation
from the same checkout; preserve both runtime directories until it succeeds.

The settings save records `previous_layout_config_id` with the newly installed
revision until the old directory is retained. If the switch succeeds but the move
fails, retry the normal upgrade or `--rollback` from the same installed revision's
checkout. Both routes check readiness and finish that pending move before reporting
`already_current`. If the move completed before interruption, the retry recognizes
its recorded destination and clears the pending field. A collision, missing source
and backup, or a different selected revision leaves the operation incomplete and
preserves the files. An older failed attempt without previous-revision attribution
also reports incomplete retention rather than guessing which backup to use.

An older installation that still stores Compose settings or deployed code under
`_receiver/` needs its matching earlier migration procedure first. The current
commands reject those legacy aliases rather than mounting them into a service.

#### Configure Codex exporters

Back up the global user `config.toml`, then configure permanent exporters there
for the Codex application in use. Enable both logs and traces. At first
provision, obtain `client_token` from
`<primary>/.agent/tools/workflow-capture/compose-settings.json` inside the
configuration writer. Use its value for `<private-client-token>` in both
headers and keep the value out of output and evidence. Adjust the endpoint
for an explicitly selected receiver port.

```toml
[otel]
exporter = { otlp-http = { endpoint = "http://127.0.0.1:4318/v1/logs", protocol = "json", headers = { x-amiga-codex-capture = "<private-client-token>" } } }
trace_exporter = { otlp-http = { endpoint = "http://127.0.0.1:4318/v1/traces", protocol = "json", headers = { x-amiga-codex-capture = "<private-client-token>" } } }
metrics_exporter = "none"
log_user_prompt = false
```

Reload the client if needed. Confirm actual recording below in the desktop or
IDE client that will do the work and after any client restart during the
initial acceptance. A process-local CLI override supplies evidence for that
CLI process only. Logs can include private tool arguments and outputs even
when prompt logging is disabled.

### Register and read the fresh event

Run from the actual initiating thread. Establish its `CODEX_THREAD_ID`
independently of the archive. Provide the stable task and execution IDs.
The start command checks Docker, recovers missing or stopped services from
the installed revision, and checks both services and new errors. The receiver
validates the actual thread's rollout before registering it over HTTP. Preserve the registration receipt and UTC
time. Use an ordinary short activity in the active client to emit telemetry.

```text
python tools/workflow/capture/register.py start --otel --project <primary> --task <task> --execution <execution>
python tools/workflow/capture/register.py confirm --project <primary> --task <task> --execution <execution>
```

A live start supplies one 180-second deadline measured before the first Docker
check. `confirm` polls the selected host archive every 250 ms while waiting.
It normally waits up to five seconds in a call and returns exit 3 with
`status = pending` if the original budget remains. Finish a short verification
turn if the client flushes at turn end, then invoke the same confirmation
command in the same thread. The registration and original deadline remain in
force. Exit 1 reports a failed prerequisite, an archive error or expiration.
Resolve the reported cause and use a fresh execution ID for a new attempt,
closing an earlier open registration through the normal stop command.

Require exit 0 and `status = confirmed` before planning or executing the task.
Keep the proof's archive path, byte range, record indexes, digest, event time and
observation time with the registration receipt. It identifies a complete new
Collector record for the initiating thread whose event time is strictly after
registration. Select only records generated by ordinary activity in the active
client. An earlier file, native rollout, HTTP success and a manually submitted
sample supply diagnostic information. Mark manual OTLP requests with
`X-Capture-Diagnostic: 1` and omit the private application header. The receiver
also marks uncredentialed requests as diagnostics. Diagnostic events remain
available for testing admission and archive delivery and cannot confirm startup.

### Continue and close an interval

Use the actual thread's ID and record the selected time in UTC. Include a
continuation only when explicitly selected. For a stopped or failed task
preserve the last included event and the turn ID where available. If new
telemetry falls after a requested historical stop, the tool refuses that
boundary and leaves the range open. Record the discrepancy and choose the
actual last included boundary before retrying. A stop without `--at` includes
the interval through the host's current time at the command boundary.

```text
python tools/workflow/capture/register.py include --project <primary> --task <task> --execution <execution>
python tools/workflow/capture/register.py stop --project <primary> --task <task> --execution <execution> --at <covered-utc> --turn-id <covered-turn-id>
python tools/workflow/capture/register.py status --project <primary>
```

A stop closes only the selected registration. Both services continue to handle
other open registrations and eligible delayed events. Close active
registrations and let the client flush before any separately selected service
maintenance.

### Historical and dedicated CLI capture

Use `start` without `--otel` for dedicated native evidence and explicitly
selected artifacts. For a dedicated CLI probe with OTLP, select `--otel` and use
a unique execution ID in the project. `--at` selects a historical interval.
`--after-turn <setup-turn-id>` leaves a selected interval pending until a control
turn finishes. Run `arm` after the control turn so the saved interval starts
immediately after its recorded completion.

```text
python tools/workflow/capture/register.py start --project <primary> --task <task> --execution <execution>
python tools/workflow/capture/register.py arm --project <primary> --task <task> --execution <execution>
```

These historical and native-only selections remain valid for evidence
collection. Ordinary task startup requires the fresh active-client proof
above. Use the same OTLP/HTTP JSON endpoint in any dedicated CLI probe and
preserve its `--json` output as bytes. The earlier dedicated pilot found
identifiable spans and no identifiable metric points. Preserve the chosen
Collector image identity, configuration, source IDs and any transformation
when interpreting a dedicated probe.

## Capture a stage

Keep the task and execution association in the existing registration and archived
session or command records. Return selected archive paths in the common result's
existing `summary` or relevant check summaries. The
[skill result schema](../result.schema.json) defines its permitted fields; do not
add an `observations.capture` object. Save the actual paired results and separately produced
verification reports as selected artifacts, including the status and complete
output of a failed attempt. Use a scoped Git diff at the stage boundary if
native patch events omit the required content. Include newly created files and
binary inputs separately when the diff does not preserve their bytes. A script
that writes and restores a file inside one command can finish with an empty
stage diff. The accepted history records tool events and observable changes at command and stage boundaries. Internal writes that a script restores without emitting their contents remain unobserved.

## Preserve selected files at task closure

Preserve selected session files, inputs, results and logs in their existing
private execution directory when requested. This copies whole original files
without a publication record or closure stage. Existing telemetry stays in place.
Use the task and execution IDs from the selected registration.

```text
python tools/workflow/capture/records.py preserve --primary <primary> --task-id <task> --execution-id <execution> --input <selected-files.json>
```

For ignored test outputs before task worktree removal, the independent
[artifact tool](../../artifacts/README.md) also works without telemetry registration.

Use stable IDs for the selected files. A path into the execution directory names
an already preserved file; otherwise use the known absolute source path and the
desired path relative to the execution directory. Set `complete` after observing
the producer finish writing, such as after it exits. A producer already writing
to the execution directory can continue at the same path.

```json
{
  "files": [
    {"id": "main-session", "source": "<absolute source path>", "path": "sessions/main.jsonl", "complete": true},
    {"id": "verification-log", "source": "<absolute source path>", "path": "logs/verification.log", "complete": true}
  ]
}
```

The command prints the actual archived path for each copy or retained file.
If a requested name is occupied, use the alternate path printed by the command.
The copy does not compare payloads. On a write failure or a pending external
producer, keep the source and repeat with the same selected IDs after resolving
the specific condition. Earlier completed copies remain available. The existing
copy command reports preserved paths; reuse those paths for a retry. It does not
generate an archive revision, manifest or closure report. An ordinary copy does
not select archive analysis or assessment.

## Inspect an existing archive revision

`records.py check` remains available for historical revisions that already have
manifests. New tasks preserve whole session files through `records.py preserve`;
this copying does not create revisions or manifests. Keep the observer input
outside the evidence it evaluates.

```text
python tools/workflow/capture/records.py check --primary <primary> --task-id <task> --execution-id <execution> --revision <revision> --expect <observer.json>
```

Pass an observer JSON object with `identity`, the expected `threads` list, and
`rows` keyed by `starting_inputs`, `registration_lineage`, `communication`,
`tools_skills`, `timing_progress`, `usage_verification`, `change_history`, and
`integrity_survival`. A row can carry independent `checks` and known `gaps`, or
an explicit reason under `not_exercised` or `unavailable`. An exact file check
uses the expected archive-relative path and the digest of its independently held
source, for example `{"kind":"archive","path":"cli/covered.jsonl","sha256":"<digest>"}`.
In the starting-input row, check only the required approved, handed-off or
explicitly retained versions. Use `git_blob` for approved input bytes retained
in Git and `archive` for selected copies. A missing expected approved version
remains a gap. Use `commit_ref` for required ancestry and `source_range`
for selected original JSONL byte ranges. Check exits with status 1 when the
report contains gaps.

## Selected timing reports

When selecting a verification timing report for preservation, save a task-local
selection record using the actual invocation's output. The verifier already writes
the report and prints its path and SHA-256 digest; this operation makes a selected
archival copy and receipt. It does not create the original report. Select both final
profiles when both were required. Select a failed attempt or diagnostic only when
requested, using its actual outcome and, for a diagnostic, exact invocation.

The selection identifies the task, profile, attempt, reviewed candidate, actual
outcome and report path relative to the source checkout. Use the path printed on
the verifier's `[TIMING] Report generation` line and the digest printed on its
`[TIMING] Report SHA-256` line. The report's immediate parent directory supplies
the attempt ID. Current commit and milestone reports use the diagnostics directory
too. Keep their actual profile names. There is no completion JSON to read.
Keep the selection with that invocation's evidence; do not substitute a guessed
path, a later attempt or a digest calculated only from an unverified archive copy.

Copy the named report and receipt using the explicit surviving primary and
source checkouts. This command works with or without a telemetry registration:

```text
python tools/workflow/capture/records.py preserve-timing --primary <primary> --source-checkout <task-checkout> --task-id <task> --input <selection.json>
```

```json
{
  "task_id": "<task>",
  "profile": "commit",
  "attempt": "<attempt>",
  "candidate": "<reviewed-candidate>",
  "outcome": "passed",
  "path": ".agent/tmp/verification/diagnostics/<attempt>/test-runtime-report.md",
  "sha256": "<producer-digest>"
}
```

Use profile `milestone` for that profile's report. A selected diagnostic uses
profile `diagnostic` and additionally supplies its exact `invocation`. Its directory
name is also the attempt ID, which is 32 lower-case hexadecimal characters. Supply
the actual reviewed candidate and outcome for either route. The source checkout
may also be the primary.

The copy procedure checks Git identity and the ignored source and destination
paths. It hashes the selected bytes, writes the report and receipt to a temporary
directory and publishes them together into
`.agent/task-archives/<task>/.timing-reports/<attempt>/`. It reuses only an
identical complete entry. On collision, unreadable source or hash mismatch, keep
the originals for resolution. Verify the receipt and report digest from the
primary before cleanup. This receipt preserves one report. If unregistered input
preservation is also required, retain the task checkout and snapshots pending
an explicit disposition.
