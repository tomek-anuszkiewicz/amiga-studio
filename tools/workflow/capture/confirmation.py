"""Read fresh, attributable Collector evidence from the primary host archive."""

from hashlib import sha256
import json
import re
import time

from otlp import DIAGNOSTIC_KEY, ROUTE_KEY, SIGNALS, attributed_records, attributes, items, record_interval
from record_sources import safe_file
from registry import CaptureError, epoch_ns, read_json


STARTUP_SECONDS = 180
POLL_SECONDS = 0.25
DEFAULT_WAIT_SECONDS = 5
STAGE = "Collector archive proof"
SIGNAL_FILES = {"/v1/logs": "logs.jsonl", "/v1/traces": "traces.jsonl"}
CHUNK = re.compile(r"(logs|traces)-([0-9a-f]{32})-([0-9a-f]{64})\.jsonl")
MAX_RECORD = 32 * 1024 * 1024


def archive_path(project, registration_path, name):
    path = registration_path.parent / "otel" / name
    project.require_ignored(path)
    return path


def chunk_paths(directory):
    if directory.exists():
        for path in sorted(directory.iterdir()):
            match = CHUNK.fullmatch(path.name)
            if match:
                yield path, f"/v1/{match[1]}"


def baseline(project, registration_path):
    positions = {"chunks": [path.name for path, _ in chunk_paths(
        archive_path(project, registration_path, "logs.jsonl").parent
    )]}
    for name in SIGNAL_FILES.values():
        path = archive_path(project, registration_path, name)
        try:
            with safe_file(path, single_link=True).open("rb") as stream:
                end = stream.seek(0, 2)
                if end:
                    stream.seek(-1, 2)
                positions[name] = {"offset": end, "partial": end > 0 and stream.read(1) != b"\n"}
        except ValueError:
            if path.exists():
                raise
        except FileNotFoundError:
            pass
    return positions


def qualifying_record(raw, endpoint, route, thread, registered_ns, observed_ns):
    resource_key, scope_key, leaf = SIGNALS[endpoint]
    try:
        rows = json.loads(raw)
        for resource_index, group in enumerate(items(rows, resource_key)):
            resource = attributes(group.get("resource", {}))
            if resource.get(ROUTE_KEY) != route or DIAGNOSTIC_KEY in resource:
                continue
            for scope_index, scope in enumerate(items(group, scope_key)):
                for record_index, (record, attrs) in enumerate(attributed_records(leaf, group, scope)):
                    if attrs is None or attrs["conversation.id"] != thread:
                        continue
                    try:
                        start, end = record_interval(leaf, record, attrs)
                    except (ValueError, OverflowError):
                        continue
                    if registered_ns < start <= end <= observed_ns:
                        return {
                            "signal": resource_key, "resource_index": resource_index,
                            "scope_index": scope_index, "record_index": record_index, "event_ns": start,
                        }
    except (ValueError, UnicodeError):
        return None
    return None


def scan(project, path, endpoint, position, data, deadline):
    if not path.exists():
        if position["offset"]:
            raise CaptureError(f"{STAGE}: a previous file disappeared.")
        return None
    with safe_file(path, single_link=True).open("rb") as stream:
        offset = position["offset"]
        if stream.seek(0, 2) < offset:
            raise CaptureError(f"{STAGE}: a Collector file was truncated.")
        stream.seek(offset)
        while True:
            deadline.remaining(STAGE)
            raw = stream.readline(MAX_RECORD + 1)
            if len(raw) > MAX_RECORD:
                raise CaptureError(f"{STAGE}: a Collector record exceeds the bounded read size.")
            if not raw.endswith(b"\n"):
                break
            end = stream.tell()
            observed_ns = time.time_ns()
            deadline.remaining(STAGE)
            if not position["partial"]:
                route = f"{data['task_id']}/{data['execution_id']}"
                record = qualifying_record(
                    raw, endpoint, route, data["root_thread_id"], epoch_ns(data["start_utc"]), observed_ns,
                )
                deadline.remaining(STAGE)
                if record:
                    return {
                        **record, "path": path.relative_to(project.primary).as_posix(),
                        "byte_start": offset, "byte_end": end, "line_sha256": sha256(raw).hexdigest(),
                        "observed_ns": observed_ns,
                    }
            position.update(offset=end, partial=False)
            offset = end
    return None


def confirm(project, registration_path, thread, wait_seconds, deadline_type):
    try:
        data = read_json(registration_path)
        if not project.matches_common(data["git_common_dir"]):
            raise CaptureError("The selected registration belongs to a different Git repository.")
        if data["schema_version"] != 1 or registration_path != project.registration_path(
            data["task_id"], data["execution_id"]
        ):
            raise CaptureError("The selected registration does not match the execution path.")
        if data["root_thread_id"] != thread or data["end_utc"] is not None or not data["otel"]:
            raise CaptureError("An open OTLP registration for the initiating thread is required.")
        startup = data.get("startup")
        if not startup:
            raise CaptureError("The selected execution has no live startup deadline.")
        deadline = deadline_type(startup["deadline_ns"])
        positions = {name: dict(startup["baseline"].get(name, {"offset": 0, "partial": False}))
                     for name in SIGNAL_FILES.values()}
        paths = {name: archive_path(project, registration_path, name) for name in SIGNAL_FILES.values()}
        directory = next(iter(paths.values())).parent
        seen = set(startup["baseline"].get("chunks", []))
        pause_at = time.monotonic() + wait_seconds
        while True:
            deadline.remaining(STAGE)
            legacy_files = [(paths[name], endpoint, positions[name]) for endpoint, name in SIGNAL_FILES.items()]
            chunks = [(path, endpoint, {"offset": 0, "partial": False})
                      for path, endpoint in chunk_paths(directory) if path.name not in seen]
            for path, endpoint, position in legacy_files + chunks:
                if path.name not in positions:
                    project.require_ignored(path)
                evidence = scan(project, path, endpoint, position, data, deadline)
                if path.name not in positions and position["offset"]:
                    seen.add(path.name)
                if evidence:
                    return {
                        "status": "confirmed", "task_id": data["task_id"], "execution_id": data["execution_id"],
                        "thread_id": thread, "registered_utc": data["start_utc"],
                        "deadline_ns": deadline.expires_ns, "record": evidence,
                    }
            pause = pause_at - time.monotonic()
            if pause <= 0:
                deadline.remaining(STAGE)
                return {
                    "status": "pending", "task_id": data["task_id"], "execution_id": data["execution_id"],
                    "thread_id": thread, "registered_utc": data["start_utc"],
                    "deadline_ns": deadline.expires_ns,
                }
            deadline.pause(STAGE, min(POLL_SECONDS, pause))
    except (OSError, KeyError, ValueError) as error:
        if str(error).startswith(f"{STAGE}:"):
            raise
        raise CaptureError(f"{STAGE}: {error}") from error
