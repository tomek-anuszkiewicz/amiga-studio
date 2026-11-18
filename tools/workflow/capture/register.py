"""Register the current Codex thread and close a selected execution range."""

import argparse
from datetime import UTC, datetime
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from urllib.error import HTTPError, URLError

from receiver import (
    COLLECTOR_IMAGE, PORT, PROTOCOL, UPSTREAM_PORT, Collector, control_token, handler_fingerprint, port, query,
)
from registry import (
    CaptureError, Project, add_thread, after_turn_boundary, close_registration, read_json,
    path_identity, require_available_range, thread_id, timestamp, utc, write_json,
)


def require_service(project: Project, number: int) -> dict | None:
    try:
        health = query(number, "/status")
    except HTTPError as error:
        raise CaptureError(f"Capture port {number} returned HTTP {error.code}, without readiness.") from error
    except URLError as error:
        if isinstance(error.reason, ConnectionRefusedError):
            return None
        raise CaptureError(f"Capture port {number} failed the readiness request. {error.reason}") from error
    except (TimeoutError, json.JSONDecodeError) as error:
        raise CaptureError(f"Capture port {number} did not return project receiver status.") from error
    if (not isinstance(health, dict) or health.get("protocol") != PROTOCOL
            or health.get("project_id") != project.identity or health.get("ready") is not True):
        raise CaptureError(f"Capture port {number} is occupied by a different service.")
    return health


def match_receiver(project: Project, args, health: dict) -> dict:
    if health.get("rollout_source") != path_identity(args.codex_home):
        raise CaptureError("The active receiver uses a different Codex home or has no rollout source identity.")
    if health.get("handler_config") != handler_fingerprint():
        raise CaptureError("The active receiver uses different admission code.")
    with project.lock():
        token = control_token(project, create=False)
    expected = Collector(project, args.upstream_port, args.collector_image, token)
    if (health.get("collector") != expected.name or health.get("collector_image") != expected.image
            or health.get("collector_port") != expected.number
            or health.get("collector_config") != expected.fingerprint):
        raise CaptureError("The active receiver uses different Collector settings.")
    return health


def start_receiver(project: Project, args) -> dict:
    if args.port == args.upstream_port:
        raise CaptureError("Use different ports for the project receiver and Collector.")
    previous = require_service(project, args.port)
    if previous is not None:
        return match_receiver(project, args, previous)

    log = project.control / "receiver.log"
    project.require_ignored(log)
    command = [
        sys.executable, str(Path(__file__).with_name("receiver.py").resolve()),
        "--project", str(project.primary), "--codex-home", str(args.codex_home.resolve()),
        "--port", str(args.port), "--upstream-port", str(args.upstream_port),
        "--collector-image", args.collector_image,
    ]
    flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    with log.open("w", encoding="utf-8") as stream:
        process = subprocess.Popen(
            command, cwd=project.primary, stdin=subprocess.DEVNULL, stdout=stream,
            stderr=subprocess.STDOUT, creationflags=flags, start_new_session=os.name != "nt",
        )
    deadline = time.monotonic() + 35
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise CaptureError(log.read_text(encoding="utf-8").strip() or "Receiver exited before readiness.")
        try:
            health = require_service(project, args.port)
        except CaptureError:
            # The new process reserves its socket before starting the Collector.
            health = None
        if health is not None:
            return match_receiver(project, args, health)
        time.sleep(0.15)
    raise CaptureError(f"Receiver on port {args.port} did not acknowledge readiness.")


def boundary(value: str | None) -> str:
    now = datetime.now(UTC)
    chosen = timestamp(value) if value else now
    if chosen > now:
        raise CaptureError("The selected capture boundary is in the future.")
    return utc(chosen)


def register(project: Project, args, thread: str | None) -> dict:
    path = project.registration_path(args.task, args.execution)
    with project.lock():
        project.require_ignored(path)
        at = boundary(getattr(args, "at", None))
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
            add_thread(project, data, thread, at, turn)
        else:
            if not path.exists():
                raise CaptureError("No registration exists for the requested execution.")
            data = read_json(path)
            if data["task_id"] != args.task or data["execution_id"] != args.execution:
                raise CaptureError("The requested IDs differ from the stored registration.")
            if Path(data["git_common_dir"]) != project.common:
                raise CaptureError("Capture registration belongs to a different Git repository.")
            if data["end_utc"] is not None:
                raise CaptureError("The requested execution has already ended.")
            if args.action == "arm":
                thread = thread_id(data["root_thread_id"])
                turn = data.get("pending_after_turn_id")
                if turn is None:
                    raise CaptureError("This execution has no pending control turn.")
                at = boundary(after_turn_boundary(args.codex_home, project.common, thread, turn))
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
        path.parent.mkdir(parents=True, exist_ok=True)
        write_json(path, data)
    return {
        "task_id": data["task_id"], "execution_id": data["execution_id"],
        "thread_id": thread, "boundary_utc": at, "turn_id": turn,
        "pending": "pending_after_turn_id" in data,
        "action": args.action, "registration": path.relative_to(project.primary).as_posix(),
    }


def service_action(project: Project, args) -> dict:
    with project.launcher_lock():
        health = require_service(project, args.port)
        if health is None:
            raise CaptureError(f"No receiver is listening on capture port {args.port}.")
        if args.action == "status":
            return health
        with project.lock():
            token = control_token(project)
        try:
            return query(args.port, "/shutdown", {}, token, timeout=30)
        except HTTPError as error:
            result = json.load(error)
            raise CaptureError(result["error"]) from error


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    for action in ("start", "arm", "include", "stop", "status", "shutdown"):
        command = commands.add_parser(action)
        command.add_argument("--project", required=True, type=Path, help="Primary checkout root.")
        if action in ("start", "arm", "include", "stop"):
            command.add_argument("--task", required=True)
            command.add_argument("--execution", required=True)
        if action in ("include", "stop"):
            command.add_argument("--at", help="Selected time in ISO 8601 with offset. Defaults to current UTC.")
        if action in ("start", "include", "stop"):
            command.add_argument("--turn-id", type=thread_id, help="Observed turn at the selected boundary.")
        if action in ("start", "status", "shutdown"):
            command.add_argument("--port", type=port, default=PORT)
        if action == "start":
            start = command.add_mutually_exclusive_group()
            start.add_argument("--at", help="Selected ISO 8601 time with offset. Defaults to current UTC.")
            start.add_argument("--after-turn", type=thread_id, help="Wait to arm after this control turn.")
            command.add_argument("--otel", action="store_true", help="Start or reuse the local OTLP receiver.")
            command.add_argument("--upstream-port", type=port, default=UPSTREAM_PORT)
            command.add_argument("--collector-image", default=COLLECTOR_IMAGE)
        if action in ("start", "arm"):
            command.add_argument(
                "--codex-home", type=Path, default=Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")
            )
    return parser.parse_args()


def main():
    args = arguments()
    try:
        project = Project(args.project, Path.cwd())
        project.prepare()
        if args.action in ("status", "shutdown"):
            report = service_action(project, args)
        elif args.action == "arm":
            report = register(project, args, None)
        else:
            thread = thread_id(os.environ.get("CODEX_THREAD_ID"))
            if args.action == "start" and args.after_turn and args.turn_id:
                raise CaptureError("An after-turn start cannot also specify a start turn ID.")
            if args.action == "start" and args.otel:
                with project.launcher_lock():
                    health = start_receiver(project, args)
                    report = register(project, args, thread)
                    report["receiver"] = health
                    report["endpoint"] = f"http://127.0.0.1:{args.port}"
            else:
                report = register(project, args, thread)
    except (OSError, CaptureError, KeyError, ValueError) as error:
        if getattr(args, "at", None) is not None:
            print(f"Requested boundary {args.at}.", file=sys.stderr)
        print(str(error), file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
