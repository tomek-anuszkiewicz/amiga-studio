"""Register a Codex thread, confirm its Collector record and close its range."""

import argparse
from datetime import UTC, datetime
import json
import os
from pathlib import Path
import sys

from compose_service import CAPTURE_SERVICES, ComposeService, Deadline, MOUNTED_HOME, saved_settings, write_authority
from confirmation import DEFAULT_WAIT_SECONDS, STARTUP_SECONDS, baseline, confirm
from control import IDENTITY, control_key, registration_request, send_registration, status
from record_sources import safe_file
from registry import (
    CaptureError, Project, add_thread, after_turn_boundary, close_registration, read_json,
    require_available_range, rollout_header, rollout_paths, thread_id, timestamp, utc, write_json,
)


def default_codex_home():
    return Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")


def boundary(value: str | None, *, now=None) -> str:
    now = now or datetime.now(UTC)
    chosen = timestamp(value) if value else now
    if chosen > now:
        raise CaptureError("The selected capture boundary is in the future.")
    return utc(chosen)


def verify_mounted_rollout(project, codex_home, thread, resolve_common):
    candidates = rollout_paths(codex_home, thread)
    if len(candidates) != 1:
        raise CaptureError("Session inputs: the receiver needs exactly one rollout for the selected thread.")
    with safe_file(candidates.pop()).open("rb") as stream:
        rollout_header(stream, thread, project.common, resolve_common=resolve_common)


def register(project, args, thread, *, deadline=None, startup=None, resolve_common=None, host_now=None):
    path = project.registration_path(args.task, args.execution)
    stage = "Execution registration"
    timeout = min(30, deadline.remaining(stage)) if deadline else 30
    with project.lock(timeout):
        project.require_ignored(path)
        at = boundary(getattr(args, "at", None), now=host_now)
        turn = getattr(args, "turn_id", None)
        if args.action == "start":
            if path.exists():
                raise CaptureError("This execution ID already has a registration.")
            data = {
                "schema_version": 1,
                "task_id": args.task,
                "execution_id": args.execution,
                "root_thread_id": thread,
                "git_common_dir": str(project.common),
                "start_utc": at,
                "end_utc": None,
                "otel": args.otel,
                "threads": [],
            }
            if turn is not None:
                data["start_turn_id"] = turn
            if args.after_turn is not None:
                data["pending_after_turn_id"] = args.after_turn
            if startup is not None:
                data["startup"] = startup
            add_thread(project, data, thread, at, turn)
        else:
            if not path.exists():
                raise CaptureError("No registration exists for the requested execution.")
            data = read_json(path)
            if data["schema_version"] != 1 or data["task_id"] != args.task or data["execution_id"] != args.execution:
                raise CaptureError("The requested IDs differ from the stored registration.")
            if not project.matches_common(data["git_common_dir"]):
                raise CaptureError("Capture registration belongs to a different Git repository.")
            if data["end_utc"] is not None:
                raise CaptureError("The requested execution has already ended.")
            if args.action == "arm":
                thread = thread_id(data["root_thread_id"])
                turn = data.get("pending_after_turn_id")
                if turn is None:
                    raise CaptureError("This execution has no pending control turn.")
                completed = after_turn_boundary(
                    args.codex_home, project.common, thread, turn, resolve_common=resolve_common,
                )
                at = boundary(completed, now=host_now)
                require_available_range(project, thread, at, except_path=path)
                data["start_utc"] = at
                data["threads"][0]["start_utc"] = at
                data["after_turn_id"] = data.pop("pending_after_turn_id")
            elif args.action == "include":
                if "pending_after_turn_id" in data:
                    raise CaptureError("Arm capture before including a continuation.")
                if timestamp(at) < timestamp(data["start_utc"]):
                    raise CaptureError("The continuation precedes the start of the execution.")
                add_thread(project, data, thread, at, turn)
            else:
                close_registration(data, thread, at, turn)
        if deadline:
            deadline.remaining(stage)
        path.parent.mkdir(parents=True, exist_ok=True)
        write_json(path, data)
    report = {
        "task_id": data["task_id"], "execution_id": data["execution_id"],
        "thread_id": thread, "boundary_utc": at, "turn_id": turn,
        "pending": "pending_after_turn_id" in data,
        "action": args.action, "registration": path.relative_to(project.primary).as_posix(),
    }
    if startup is not None:
        report["deadline_ns"] = startup["deadline_ns"]
    return report


def controlled_registration(project, request):
    request = registration_request(request)
    args = argparse.Namespace(
        action=request["action"], task=request["task"], execution=request["execution"], otel=request["otel"],
        at=request["at"], turn_id=request["turn_id"], after_turn=request["after_turn"], codex_home=MOUNTED_HOME,
    )
    thread = request["thread"]
    deadline = Deadline(request["deadline_ns"])
    deadline.remaining("Execution registration")
    path = project.registration_path(args.task, args.execution)
    resolve_common = project.checkouts()
    if (args.action == "start" and args.otel) or (args.action == "include" and read_json(path)["otel"]):
        verify_mounted_rollout(project, MOUNTED_HOME, thread, resolve_common)
    startup = None
    if args.action == "start" and args.otel and not (args.at or args.after_turn):
        startup = {"deadline_ns": deadline.expires_ns, "baseline": baseline(project, path)}
    return register(project, args, thread, deadline=deadline, startup=startup, resolve_common=resolve_common)


def port(value):
    number = int(value)
    if not 1024 <= number <= 65535:
        raise argparse.ArgumentTypeError("Select a port between 1024 and 65535.")
    return number


def wait_seconds(value):
    number = int(value)
    if not 0 <= number <= STARTUP_SECONDS:
        raise argparse.ArgumentTypeError(f"Select a wait from 0 to {STARTUP_SECONDS} seconds.")
    return number


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    for action in ("provision", "start", "confirm", "arm", "include", "stop", "status", "handoff", "recover"):
        command = commands.add_parser(action)
        command.add_argument("--project", required=True, type=Path, help="Primary checkout root.")
        if action not in ("provision", "status", "handoff", "recover"):
            command.add_argument("--task", required=True)
            command.add_argument("--execution", required=True)
        if action in ("include", "stop"):
            command.add_argument("--at", help="Selected time in ISO 8601 with offset. Defaults to current UTC.")
        if action == "stop":
            command.add_argument("--legacy-installed", action="store_true",
                                 help="Close an interval before the first REST-control cutover.")
        if action in ("start", "include", "stop"):
            command.add_argument("--turn-id", type=thread_id, help="Observed turn at the selected boundary.")
        if action in ("provision", "start", "status"):
            command.add_argument("--port", type=port, help="Receiver's host port. Defaults to 4318 at provision.")
        if action == "start":
            start = command.add_mutually_exclusive_group()
            start.add_argument("--at", help="Historical boundary for a selected evidence archive.")
            start.add_argument("--after-turn", type=thread_id, help="Wait to arm after this control turn.")
            command.add_argument("--otel", action="store_true", help="Ensure Compose OTLP services and register.")
        if action in ("provision", "start"):
            command.add_argument("--collector-image", help="Override the pinned Collector at initial provision.")
        if action in ("provision", "start", "arm"):
            command.add_argument("--codex-home", type=Path, help="Defaults to the service's saved Codex home if any.")
        if action == "confirm":
            command.add_argument("--wait-seconds", type=wait_seconds, default=DEFAULT_WAIT_SECONDS)
    return parser.parse_args()


def ready(service):
    service.wait_engine()
    health = service.healthy(service.containers(), status)
    if not health:
        raise CaptureError("Receiver and Collector readiness: both installed services are required. "
                           "Use recover during the maintenance window if either service is stopped.")
    return health


def handoff(service, args):
    published = ComposeService(service.project, args, service.deadline, revision="upgrade")
    service.wait_engine()
    containers = service.containers()
    return {
        "project_id": service.project.identity, "compose_project": service.name, "port": service.port,
        "saved_config_id": service.config_id, "published_config_id": published.config_id,
        "control": "http" if service.has_control else "legacy",
        "services": {name: ("missing" if name not in containers
                            else "running" if containers[name]["State"]["Running"] else "stopped")
                     for name in CAPTURE_SERVICES},
        "open_registrations": [
            {"task_id": record["task_id"], "execution_id": record["execution_id"],
             "root_thread_id": record["root_thread_id"]}
            for _, record in service.project.registrations() if record["end_utc"] is None
        ],
    }


def host_request(args):
    deadline = Deadline.start(STARTUP_SECONDS)
    cwd = Path.cwd()
    project = Project(args.project, cwd)
    action = args.action
    if action in ("status", "handoff"):
        installed = ComposeService(project, args, deadline, revision="saved")
        return ready(installed) if action == "status" else handoff(installed, args)
    project.prepare()
    if action in ("provision", "recover"):
        with project.launcher_lock(timeout=min(30, deadline.remaining("Compose preparation"))):
            if action == "provision" and args.codex_home is None and saved_settings(project) is None:
                args.codex_home = default_codex_home()
            revision = "saved" if action == "recover" else None
            return ComposeService(project, args, deadline, revision=revision).ensure(cwd, status)
    thread = None if action == "arm" else thread_id(os.environ.get("CODEX_THREAD_ID"))
    path = project.registration_path(args.task, args.execution)
    if action == "confirm":
        return confirm(project, path, thread, args.wait_seconds, Deadline)
    use_otel = action == "start" and args.otel
    legacy = getattr(args, "legacy_installed", False)
    at, turn, after = (getattr(args, name, None) for name in ("at", "turn_id", "after_turn"))
    if action == "start" and after and turn:
        raise CaptureError("An after-turn start cannot also specify a start turn ID.")
    with project.launcher_lock(timeout=min(30, deadline.remaining("Execution registration"))):
        saved = saved_settings(project)
        if not use_otel and saved is None:
            if legacy:
                raise CaptureError("There is no verified pre-REST installation to close.")
            if action in ("start", "arm") and args.codex_home is None:
                args.codex_home = default_codex_home()
            return register(project, args, thread, deadline=deadline)
        if saved is None and args.codex_home is None:
            args.codex_home = default_codex_home()
        service = ComposeService(project, args, deadline, revision="saved" if saved else None)
        if legacy:
            if service.has_control:
                raise CaptureError("This installed revision supports HTTP control. Use stop without --legacy-installed.")
            health = ready(service)
            return service.legacy_stop(health["containers"]["receiver"], {
                "arguments": {"task": args.task, "execution": args.execution, "action": action,
                              "at": at, "turn_id": turn},
                "thread": thread, "verify_rollout": False, "startup": None,
                "deadline_ns": deadline.expires_ns, "host_utc": utc(datetime.now(UTC)),
            })
        if not service.has_control:
            raise CaptureError("The verified installed revision predates REST control. See the first-cutover procedure.")
        if use_otel:
            health = service.ensure(cwd, status)
        else:
            health = ready(service)
            if action == "include":
                write_authority(project, service.authority, service.codex_home, cwd)
        if health.get("control_version") != 1 or not isinstance(health.get("instance_id"), str):
            raise CaptureError("The verified installed receiver did not advertise registration control.")
        key = control_key(service.settings["token"], service.settings["client_token"])
    # Keep the host launcher unlocked during HTTP. The installed receiver must
    # claim the shared mutation boundary itself, including for direct requests.
    report = send_registration(service.port, key, {
        **{field: health[field] for field in IDENTITY},
        "action": action, "task": args.task, "execution": args.execution, "thread": thread,
        "at": at, "turn_id": turn, "after_turn": after, "otel": use_otel,
        "deadline_ns": deadline.expires_ns,
    }, deadline)
    if use_otel:
        report["receiver"] = health
        report["endpoint"] = f"http://127.0.0.1:{service.port}"
    return report


def main():
    try:
        report = host_request(arguments())
    except (OSError, KeyError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2))
    return 3 if report.get("status") == "pending" else 0


if __name__ == "__main__":
    sys.exit(main())
