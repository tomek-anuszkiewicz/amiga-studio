"""Project identity, registration intervals and private archive storage."""

from __future__ import annotations

from contextlib import contextmanager
from datetime import UTC, datetime, timedelta
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from tempfile import NamedTemporaryFile
import time
import uuid


ARCHIVE = Path(".agent/task-archives")
ROLLOUT_HEADER_LIMIT = 512 * 1024
EPOCH = datetime(1970, 1, 1, tzinfo=UTC)
SEGMENT = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,63}")
TIMESTAMP = re.compile(
    r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?(?:Z|[+-]\d{2}:\d{2})"
)


class CaptureError(ValueError):
    """A capture prerequisite or requested boundary is invalid."""


def git(path: Path, *args: str) -> subprocess.CompletedProcess[bytes]:
    # Preserve the sandbox's exact checkout trust without its wildcard or other overrides.
    trust = git_checkout_trust(path)
    env = {key: value for key, value in os.environ.items() if not key.upper().startswith("GIT_")}
    return subprocess.run(
        ["git", *trust, "-C", str(path), *args], capture_output=True, env=env
    )


def git_checkout_trust(path: Path) -> list[str]:
    try:
        count = int(os.environ.get("GIT_CONFIG_COUNT", "0"))
    except ValueError:
        return []
    if not 0 < count <= len(os.environ) // 2:
        return []
    checkout = path.resolve()
    args = []
    for index in range(count):
        key = os.environ.get(f"GIT_CONFIG_KEY_{index}")
        value = os.environ.get(f"GIT_CONFIG_VALUE_{index}")
        if key is None or value is None:
            return []
        if key.casefold() != "safe.directory":
            continue
        if not value:
            args.clear()
            continue
        directory = Path(value)
        if "*" not in value and directory.is_absolute() and checkout.is_relative_to(directory.resolve()):
            args.extend(("-c", f"safe.directory={value}"))
    return args


def git_path(path: Path, flag: str) -> Path:
    result = git(path, "rev-parse", "--path-format=absolute", flag)
    if result.returncode:
        message = f"Git cannot resolve {flag} for the selected checkout."
        for line in result.stderr.decode("utf-8", errors="replace").splitlines():
            if line.startswith(("fatal: ", "error: ")) and line.isprintable():
                message = f"{message} {line}"
                break
        raise CaptureError(message)
    return Path(os.fsdecode(result.stdout.rstrip(b"\r\n"))).resolve(strict=True)


def path_identity(path: Path) -> str:
    return hashlib.sha256(os.fsencode(os.path.normcase(str(path.resolve())))).hexdigest()


def segment(value: str) -> str:
    if not isinstance(value, str) or not SEGMENT.fullmatch(value):
        raise CaptureError("Task and execution IDs must use 1 to 64 portable filename characters.")
    if value.endswith(".") or Path(value).is_reserved():
        raise CaptureError("Task and execution IDs must be portable filenames.")
    return value


def thread_id(value: str) -> str:
    try:
        result = str(uuid.UUID(value))
    except (ValueError, TypeError, AttributeError) as error:
        raise CaptureError("A runtime thread ID must be a canonical UUID.") from error
    if value != result:
        raise CaptureError("A runtime thread ID must be a canonical UUID.")
    return result


def timestamp(value: str) -> datetime:
    if not isinstance(value, str) or not TIMESTAMP.fullmatch(value):
        raise CaptureError("An execution boundary must be ISO 8601 with an explicit UTC offset.")
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(UTC)


def utc(value: datetime) -> str:
    return value.isoformat(timespec="microseconds").replace("+00:00", "Z")


def epoch_ns(value: str) -> int:
    delta = timestamp(value) - EPOCH
    return (delta.days * 86400 + delta.seconds) * 1_000_000_000 + delta.microseconds * 1000


def rollout_paths(codex_home: Path, thread: str) -> set[Path]:
    return {
        path
        for source in ("sessions", "archived_sessions")
        for path in (codex_home / source).rglob(f"rollout-*-{thread}.jsonl")
        if path.is_file()
    }


def rollout_header(stream, thread: str, common: Path) -> dict:
    line = stream.readline(ROLLOUT_HEADER_LIMIT)
    if not line.endswith(b"\n"):
        raise CaptureError("The selected rollout header is incomplete.")
    header = json.loads(line)
    info = header["payload"]
    if header["type"] != "session_meta" or info["id"] != thread:
        raise CaptureError("The rollout header does not match the selected thread.")
    if git_path(Path(info["cwd"]), "--git-common-dir") != common:
        raise CaptureError("The selected rollout belongs to a different Git repository.")
    return info


def after_turn_boundary(codex_home: Path, common: Path, thread: str, turn: str) -> str:
    candidates = rollout_paths(codex_home, thread)
    if len(candidates) != 1:
        raise CaptureError("Exactly one rollout is required for the selected control turn.")
    with candidates.pop().open("rb") as stream:
        rollout_header(stream, thread, common)
        for line in stream:
            if not line.endswith(b"\n"):
                break
            event = json.loads(line)
            if event.get("type") != "event_msg":
                continue
            payload = event["payload"]
            if payload.get("type") == "task_complete" and payload.get("turn_id") == turn:
                return utc(timestamp(event["timestamp"]) + timedelta(microseconds=1))
    raise CaptureError("The selected control turn has no completed event in the rollout.")


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: dict) -> None:
    with NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name)
        try:
            json.dump(value, stream, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        except BaseException:
            stream.close()
            temporary.unlink(missing_ok=True)
            raise
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


@contextmanager
def file_lock(path: Path, timeout: float = 30):
    if os.name == "nt":
        import msvcrt
    else:
        import fcntl

    with path.open("a+b") as stream:
        deadline = time.monotonic() + timeout
        while True:
            try:
                stream.seek(0)
                if os.name == "nt":
                    msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)
                else:
                    fcntl.flock(stream.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except OSError as error:
                if time.monotonic() >= deadline:
                    raise CaptureError("Timed out waiting for the project capture lock.") from error
                time.sleep(0.05)
        yield


class Project:
    def __init__(self, primary: Path, working: Path):
        self.primary = primary.resolve(strict=True)
        if self.primary != git_path(self.primary, "--show-toplevel"):
            raise CaptureError("The project path must be the root of the primary checkout.")
        self.common = git_path(self.primary, "--git-common-dir")
        if self.common != git_path(self.primary, "--git-dir"):
            raise CaptureError("Select the primary checkout as the project.")
        if self.common != git_path(working, "--git-common-dir"):
            raise CaptureError("The working directory does not belong to the selected Git repository.")
        self.identity = path_identity(self.common)
        self.archive = self.primary / ARCHIVE
        self.control = self.archive / "_receiver"

    def plain_path(self, path: Path) -> Path:
        if path.resolve() != path:
            raise CaptureError("Capture paths must stay in the primary archive without links.")
        return path

    def require_ignored(self, path: Path) -> None:
        self.plain_path(path)
        relative = path.relative_to(self.primary).as_posix()
        if git(self.primary, "check-ignore", "-q", "--", relative).returncode:
            raise CaptureError("Git must ignore the selected capture path before data is written.")

    def prepare(self) -> None:
        self.require_ignored(self.control / "registry.lock")
        tracked = git(self.primary, "ls-files", "--", ARCHIVE.as_posix())
        if tracked.returncode:
            raise CaptureError("Git cannot verify the selected capture paths are untracked.")
        if tracked.stdout:
            raise CaptureError("The capture archive contains tracked files.")
        self.control.mkdir(parents=True, exist_ok=True)

    def registration_path(self, task: str, execution: str) -> Path:
        return self.plain_path(
            self.archive / segment(task) / segment(execution) / "registration.json"
        )

    def registrations(self):
        for path in self.archive.glob("*/*/registration.json"):
            self.plain_path(path)
            registration = read_json(path)
            task = registration["task_id"]
            execution = registration["execution_id"]
            if registration["schema_version"] != 1 or path != self.registration_path(task, execution):
                raise CaptureError("Invalid capture registration identity.")
            if Path(registration["git_common_dir"]) != self.common:
                raise CaptureError("Capture registration belongs to a different Git repository.")
            yield path, registration

    def lock(self):
        return file_lock(self.plain_path(self.control / "registry.lock"))

    def launcher_lock(self):
        path = self.control / "launcher.lock"
        self.require_ignored(path)
        return file_lock(path)

    def service_lock(self):
        path = self.control / "service.lock"
        self.require_ignored(path)
        return file_lock(path, timeout=0)


def require_available_range(project: Project, thread: str, boundary: str, except_path: Path | None = None):
    starts = epoch_ns(boundary)
    for path, previous in project.registrations():
        if path == except_path:
            continue
        for selection in previous["threads"]:
            if selection["id"] != thread:
                continue
            end = selection["end_utc"]
            if end is None or starts <= epoch_ns(end):
                raise CaptureError("The selected thread overlaps an existing registered range.")


def add_thread(project: Project, registration: dict, thread: str, boundary: str, turn: str | None):
    require_available_range(project, thread, boundary)
    selection = {"id": thread, "start_utc": boundary, "end_utc": None}
    if turn is not None:
        selection["start_turn_id"] = turn
    registration["threads"].append(selection)


def close_registration(registration: dict, thread: str, boundary: str, turn: str | None):
    selected = [entry for entry in registration["threads"] if entry["id"] == thread]
    if not selected:
        raise CaptureError("Stop must run in a selected root or continuation thread.")
    end = epoch_ns(boundary)
    if end < max(epoch_ns(entry["start_utc"]) for entry in registration["threads"]):
        raise CaptureError("The stop boundary precedes a selected thread's start.")
    if end < registration.get("latest_admitted_ns", 0):
        raise CaptureError("Telemetry accepted after the requested stop. The range remains open.")
    registration["end_utc"] = boundary
    for entry in registration["threads"]:
        entry["end_utc"] = boundary
    if turn is not None:
        registration["end_turn_id"] = turn
        selected[0]["end_turn_id"] = turn
