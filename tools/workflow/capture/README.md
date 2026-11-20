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
ID. Both routes use the primary checkout and work without a receiver. For planned work [launch with the selected registration](../README.md#run-a-command-for-a-selected-execution).
When the task context is known, pass `AMIGA_TEST_TASK_ID` and `AMIGA_TEST_EXECUTION_ID`.
Launchers propagate parent invocation linkage and accept
`AMIGA_TEST_WORK_ITEM_ID` and `AMIGA_TEST_ROUND` when supplied.

Open the printed Markdown path and follow the links to its native journal,
logs and child reports. The journal persists case events during execution.
A missing finish leaves the invocation's final outcome unavailable. A started
case without a result has an unavailable duration and outcome. Reused groups
identify the earlier evidence and add zero new attempts.

## Starting inputs

Use `TA-capture-environment` to record the actual environment before implementation.
From the task checkout, run the environment command with a fresh destination.
Select the known revision that governs execution and the relevant local tools. Identify the actual shell with `--shell <name>` and inspect a locally available Collector image with `--docker-image <reference>`.
To record observed client settings, supply `--rollout <selected-session.jsonl> --turn-id <observed-turn-id>` to the capture command. Without that source the inventory reports the client observation as unavailable. The executable probe records the version of Codex found on `PATH`. Use `--node-project <repository-relative-directory>` for each selected local Node project. The comparison operation takes two saved inventories.

```text
python tools/workflow/capture/environment.py capture --output <new-inventory.json> --governing-commit <revision>
python tools/workflow/capture/environment.py compare <saved-inventory.json> <fresh-inventory.json> --output <differences.json>
```

Keep approved plans, test designs, acceptance criteria, decisions, attachments
and inventories under `.agent/tasks/` or another selected ignored location. At approval, create a version map under
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
### Explicit starting input commits

An explicitly selected starting input commit may retain named ignored paths
in Git. Apply [ordinary batch readiness](../../../.agents/rules/git-commits.md)
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
The primary archive stores the registry and each execution's Collector output.
The primary checkout's ignored, untracked `.agent/tools/workflow-capture/`
stores private Compose settings, deployed service files and the host's Git
validation. The settings file stays on the host. The receiver mounts its code,
the Git validation directory and the two required Codex session directories
read-only, alongside the writable archive. Mounting the Git validation
directory lets a running receiver pick up the host's atomic updates when a
linked worktree is discovered. The Collector mounts its configuration
read-only and writes only to the dedicated `_receiver/collector-output/` spool.
The receiver publishes complete Collector writes as individual JSONL files
under the matching execution's `otel/` directory. It checks the output route,
keeps the original bytes, and retries complete spool files after a service restart.
Existing `logs.jsonl` and `traces.jsonl` files remain readable.

#### Update a host-side capture installation

From the published primary checkout, inspect the installation before choosing a
maintenance window. The report gives the installation and port, saved and
published revisions, open task and execution IDs with their root threads, and
the current state of each capture container. Inspection leaves the containers
and registry untouched. Publication itself starts no service action.

```powershell
python tools/workflow/capture/register.py handoff --project .
```

Use the private host account that provisioned capture. In the agreed window,
recover either stopped service using the verified installed snapshot and saved
Compose settings. Run status to confirm both services are ready.

```powershell
python tools/workflow/capture/register.py recover --project .
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

Once all owners have closed their intervals, check the handoff report again.
Let the exporters flush and verify both old services are ready before applying
the published revision.

```powershell
python tools/workflow/capture/register.py handoff --project .
python tools/workflow/capture/register.py status --project .
python tools/workflow/capture/upgrade.py --project .
python tools/workflow/capture/register.py status --project .
```

The upgrade verifies the installed revision and recreates only `receiver` and
`collector`. It retains their Compose project name, port, credentials, spool,
registry and archives. Other services in the same Compose project keep running.
It checks readiness before accepting the new revision. If the switch fails,
the command attempts to restore the previous capture revision and verifies that
it is ready. An installation at the selected revision returns `already_current`
after a readiness check.

After a successful upgrade, [confirm a fresh event](#register-and-read-the-fresh-event)
from the actual Codex client using a new proof execution. Close its registration
after confirmation. Retain the upgrade and proof results with the handoff.

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

A terminated controller can leave an empty `_receiver/control-mutation`
directory in the private capture archive. Capture control and upgrades report
busy while it exists. Verify the previous operation has ended and no other
host controller or receiver request is in flight. With the operator's
approval, remove only that empty directory before retrying the selected
operation.

#### Migrate an existing Compose installation

Use a maintenance window if the primary archive's `_receiver/` directory
still contains the old Compose files. Close active capture registrations,
let exporters flush, and pause other capture commands until the migration
or rollback finishes. Keep the same checkout and source revision available
throughout the window. Use the same private host account as at provision.

From the checkout containing this guide, save the following as
`.agent/tmp/migrate-capture.py`. The script preserves the original deployment
and its settings under a named private directory in the ignored service root.
Keep that directory private. It contains the original credentials. The
migration changes only the service revision in the settings and keeps both
exporter credentials, the Compose project, port, user, image, and Codex home.
The registry, spool, and execution archives stay in place.

```python
from datetime import UTC, datetime
import hashlib
import json
import os
from pathlib import Path
import re
import sys
from types import SimpleNamespace

cwd = Path.cwd().resolve()
sys.path.insert(0, str(cwd / "tools/workflow/capture"))
import compose_service as capture
from control import status
from registry import CaptureError, Project, read_json, write_json


def require(condition, message):
    if not condition:
        raise CaptureError(message)


def aliases(directory):
    return sorted(p for p in directory.iterdir() if p.name in {
        "compose", "compose-authority.json",
    } or (p.name.startswith("compose-settings") and p.name.endswith(".json")))


def next_settings(previous):
    settings = {key: value for key, value in previous.items() if key != "config_id"}
    identity = hashlib.sha256(json.dumps(settings, sort_keys=True).encode())
    for name in capture.SERVICE_FILES:
        data = (capture.SOURCE / name).read_bytes().replace(b"\r\n", b"\n")
        identity.update(name.encode() + b"\0" + hashlib.sha256(data).digest())
    return {**settings, "config_id": identity.hexdigest()}


def owned_containers(call, name, project_id, revisions):
    ids = call("ps", "-aq", "--filter", f"label=com.docker.compose.project={name}").split()
    services = {}
    for item in json.loads(call("inspect", *ids)) if ids else []:
        labels = item["Config"]["Labels"]
        service = labels.get("com.docker.compose.service")
        require(service in {"receiver", "collector"} and service not in services
                and labels.get(capture.PROJECT_LABEL) == project_id
                and labels.get(capture.CONFIG_LABEL) in revisions,
                "Unexpected Compose ownership. Stop and resolve the conflict.")
        services[service] = item["Id"]
    return services


def verify_mounts(service, items):
    env = service.env
    expected = {
        "receiver": {
            "/capture/code": (env["CAPTURE_CODE"], False),
            "/capture/authority": (env["CAPTURE_AUTHORITY"], False),
            "/capture/project/.agent/task-archives": (env["CAPTURE_ARCHIVE"], True),
            "/capture/codex/sessions": (env["CAPTURE_SESSIONS"], False),
            "/capture/codex/archived_sessions": (env["CAPTURE_ARCHIVED_SESSIONS"], False),
        },
        "collector": {
            "/capture/collector.yaml": (env["CAPTURE_CODE"] + "/collector.yaml", False),
            "/capture/output": (env["CAPTURE_COLLECTOR_OUTPUT"], True),
        },
    }

    def host_path(path):
        value = os.path.normcase(path).replace("\\", "/").rstrip("/")
        if os.name == "nt":
            value = re.sub(r"^/(?:run/desktop/mnt/host|host_mnt)/([a-z])/", r"\1:/", value)
        return value

    for name, item in items.items():
        mounts = item["Mounts"]
        actual = {m["Destination"]: (host_path(m["Source"]), m["RW"]) for m in mounts}
        require(len(mounts) == len(expected[name])
                and all(m["Type"] == "bind" for m in mounts)
                and actual == {dest: (host_path(path), writable)
                               for dest, (path, writable) in expected[name].items()},
                f"Unexpected {name} mount source, destination, or mode.")
    probe = """
import os
import stat
from pathlib import Path
archives = Path("/capture/project/.agent/task-archives") / "_receiver"
names = {p.name for p in archives.iterdir()}
if (names & {"compose", "compose-authority.json"}
        or any(n.startswith("compose-settings") and n.endswith(".json") for n in names)):
    raise SystemExit("Legacy alias still visible inside the receiver")
for name in ("/capture/code/receiver.py", "/capture/authority/compose-authority.json"):
    if not stat.S_ISREG(os.stat(name).st_mode):
        raise SystemExit("Missing deployed file")
    if not os.statvfs(name).f_flag & os.ST_RDONLY:
        raise SystemExit("Deployed mount is writable")
"""
    service.call("Receiver filesystem verification", "exec", items["receiver"]["Id"],
                 "python", "-B", "-c", probe)


def migrate(action, project, backup):
    root = capture.verify_service_root(project)
    active = root / "compose-settings.json"
    legacy = project.control / active.name
    snapshot = backup / "original-settings.json"
    parked = backup / "archive-aliases"
    project.require_ignored(active)
    project.require_ignored(snapshot)
    previous = read_json(project.plain_path(legacy if action == "apply" else snapshot))
    name = f"amiga-capture-{project.identity[:24]}"
    old_id = previous["config_id"]
    require(re.fullmatch(r"[0-9a-f]{64}", old_id)
            and (previous["project_id"], previous["compose_project"]) == (project.identity, name),
            "The legacy settings do not belong to the selected checkout.")
    updated = next_settings(previous)
    new_id = updated["config_id"]
    require(old_id != new_id, "Select the checkout with the new mount layout.")
    deadline = capture.Deadline.start(180)
    stage = "Capture migration"
    home = Path(previous["codex_home"])
    old_code = project.control / "compose" / old_id
    old_env = dict(os.environ, CAPTURE_PROJECT_ID=project.identity, CAPTURE_CONFIG_ID=old_id,
                   CAPTURE_COMPOSE_PROJECT=name, CAPTURE_PORT=str(previous["port"]),
                   CAPTURE_CODE=str(old_code), CAPTURE_ARCHIVE=str(project.archive),
                   CAPTURE_COLLECTOR_OUTPUT=str(project.control / capture.COLLECTOR_OUTPUT),
                   CAPTURE_SESSIONS=str(home / "sessions"),
                   CAPTURE_ARCHIVED_SESSIONS=str(home / "archived_sessions"),
                   CAPTURE_COLLECTOR_IMAGE=previous["collector_image"],
                   CAPTURE_USER=previous["user"], CAPTURE_COLLECTOR_TOKEN=previous["token"],
                   CAPTURE_CLIENT_TOKEN=previous["client_token"])

    def call(*args):
        result = capture.docker(deadline, stage, args, env=old_env)
        require(not result.returncode, f"Docker {args[0]} failed with exit {result.returncode}.")
        return result.stdout

    require(call("info", "--format", "{{.OSType}}").strip() == "linux",
            "Select the Linux Docker engine.")
    if action == "apply":
        require(not os.path.lexists(active) and not os.path.lexists(backup),
                "Active host settings or the selected backup already exist.")
        old_manifest = project.plain_path(old_code / "compose.yaml")
        require(old_manifest.is_file(), "The original Compose revision is unavailable.")
        sources = [project.plain_path(path) for path in aliases(project.control)]
        project.require_ignored(parked)
        services = owned_containers(call, name, project.identity, {old_id})
        require(set(services) == {"receiver", "collector"}, "Both legacy containers are required.")
        health = status(previous["port"])
        require(health["ready"] is True
                and (health["project_id"], health["compose_project"], health["config_id"])
                == (project.identity, name, old_id), "The legacy receiver is unavailable.")
        parked.mkdir(parents=True)
        with snapshot.open("xb") as stream:
            stream.write(legacy.read_bytes())
        call("stop", *services.values())
        for source in sources:
            source.rename(parked / source.name)
        write_json(active, updated)
        args = SimpleNamespace(port=None, codex_home=None, collector_image=None)
        service = capture.ComposeService(project, args, deadline)
        service.persist(cwd)
        service.compose(stage, "config", "--quiet")
        owned_containers(call, name, project.identity, {old_id})
        since = datetime.now(UTC).isoformat()
        service.compose(stage, "up", "--detach", "--force-recreate", "receiver", "collector")
        while True:
            items = service.containers()
            if service.healthy(items, status):
                service.check_logs(items, since)
                break
            deadline.pause("New receiver and Collector readiness", 0.25)
        verify_mounts(service, items)
        require(not aliases(project.control), "Legacy aliases remain on the host.")
        print("Ready. Bind mounts and legacy alias removal verified.")
    else:
        moves = [(project.plain_path(source), project.plain_path(project.control / source.name))
                 for source in aliases(project.plain_path(parked))]
        require(all(not os.path.lexists(destination) for _, destination in moves),
                "A restore destination is occupied. Preserve both copies.")
        parked_settings = project.plain_path(backup / "isolated-settings.json")
        if os.path.lexists(active):
            require(read_json(active) == updated and not os.path.lexists(parked_settings),
                    "The host settings changed or the backup name is occupied.")
        services = owned_containers(call, name, project.identity, {old_id, new_id})
        if services:
            call("stop", *services.values())
        if active.exists():
            active.rename(parked_settings)
        for source, destination in moves:
            source.rename(destination)
        require(legacy.read_bytes() == snapshot.read_bytes(), "Original settings differ from backup.")
        require(project.plain_path(old_code / "compose.yaml").is_file(),
                "The original Compose revision is unavailable.")
        compose = ("compose", "--project-name", name, "--file", str(old_code / "compose.yaml"))
        call(*compose, "config", "--quiet")
        owned_containers(call, name, project.identity, {old_id, new_id})
        call(*compose, "up", "--detach", "--force-recreate", "receiver", "collector")
        services = owned_containers(call, name, project.identity, {old_id})
        require(set(services) == {"receiver", "collector"}, "Both restored containers are required.")
        while True:
            try:
                health = status(previous["port"], timeout=2)
            except OSError:
                deadline.pause("Legacy receiver and Collector readiness", 0.25)
                continue
            require((health["project_id"], health["compose_project"], health["config_id"])
                    == (project.identity, name, old_id), "The restored endpoint identity differs.")
            if health["ready"] is True:
                break
            deadline.pause("Legacy receiver and Collector readiness", 0.25)
        print("Original project and service revision restored.")


if len(sys.argv) != 4 or sys.argv[1] not in {"apply", "rollback"} or not re.fullmatch(
    r"[A-Za-z0-9][A-Za-z0-9_-]*", sys.argv[3]
):
    raise SystemExit("Usage: migrate-capture.py {apply|rollback} PRIMARY BACKUP-NAME")
action, primary, backup_name = sys.argv[1:]
project = Project(Path(primary), cwd)
project.prepare()
try:
    with project.launcher_lock():
        require(all(record["end_utc"] is not None for _, record in project.registrations()),
                "Close all capture registrations before maintenance.")
        root = capture.verify_service_root(project)
        backup = project.plain_path(root / "migration-rollback" / backup_name)
        migrate(action, project, backup)
except CaptureError as error:
    raise SystemExit(str(error)) from None
```

Run from the same checkout, replacing `<primary>` with the path to the
primary checkout and `<backup-name>` with a fresh name.

```text
python -B .agent/tmp/migrate-capture.py apply "<primary>" "<backup-name>"
python tools/workflow/capture/register.py status --project "<primary>"
```

Keep the successful mount check and status with the maintenance record. Then use
[Register and read the fresh event](#register-and-read-the-fresh-event) from the
actual Codex client with a new execution ID. Require a fresh archived record
and close the proof registration. The existing Codex settings are already
configured for the preserved port and credential.

If the cutover fails or is interrupted after the snapshot is written, keep
the backup and run rollback with the same checkout, primary and backup name.
The backup records which aliases were moved. Rollback checks the exact project,
service and revision labels, including when one container was already replaced.
It returns only the parked aliases, parks the new host settings, and recreates
the two services from the original snapshot. Resolve any identity or file
collision while preserving both copies before retrying. Close any proof
registration before rolling back.

```text
python -B .agent/tmp/migrate-capture.py rollback "<primary>" "<backup-name>"
```

Use the original checkout's registration and confirmation commands to obtain a
fresh event from the restored deployment.

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

Record the task, work item, run, round and stage alongside the available thread,
turn and call identities. The result envelope accepts this association under
`observations.capture`. Save the actual paired results and separately produced
verification reports as selected artifacts, including the status and complete
output of a failed attempt. Use a scoped Git diff at the stage boundary if
native patch events omit the required content. Include newly created files and
binary inputs separately when the diff does not preserve their bytes. A script
that writes and restores a file inside one command can finish with an empty
stage diff. The accepted history records tool events and observable changes at command and stage boundaries. Internal writes that a script restores without emitting their contents remain unobserved.

## Preserve selected files at task closure

After the final project graph refresh, preserve the known main and agent session
files, task inputs, results and logs in the existing private execution directory.
Use the surviving checkout and publication record. This operation preserves
whole original files. It leaves existing telemetry and retained copies in place.
Use the task and execution IDs selected from the registration at publication.
Preservation and cleanup both compare the existing registration with the
publication's recorded identity.

```text
python tools/workflow/capture/records.py preserve --primary <primary> --task-id <task> --execution-id <execution> --input <selected-files.json> --publication <surviving-record>
```

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
publication control record retains progress and marks preservation complete
when all selected paths are available. The copy does not generate an archive
revision, manifest or closure report.

For a standalone copy to an existing registered execution, omit `--publication`.
This copy cannot complete preservation for a publication. To resume that operation, pass the printed, already archived paths for files copied in
the earlier invocation. An ordinary copy does not select archive analysis or
assessment.

## Collect and assess

For explicitly selected analysis, use `TA-collect-task-records` with
`records.py collect` to create a private manifest revision. Select
`TA-check-task-records` separately to assess it against the observer's expected
evidence. Keep inputs for those commands outside the evidence they evaluate.

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

Leave the shared services running between tasks. For selected analysis, collect
again if eligible records arrived and record the checked revision, manifest
digest, checker report and effects of telemetry gaps. Ordinary task teardown
uses the separately documented whole-file copy.


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
