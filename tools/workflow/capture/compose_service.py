"""Own the durable Compose project and translate its host-side Git identity."""

from dataclasses import dataclass
from datetime import UTC, datetime
import hashlib
import json
import ntpath
import os
from pathlib import Path
import posixpath
import re
import secrets
import socket
import subprocess
import time
from urllib.error import HTTPError, URLError

from registry import ARCHIVE, CaptureError, Project, git, git_path, path_identity, read_json, write_json


MOUNTED_PROJECT = Path("/capture/project")
MOUNTED_HOME = Path("/capture/codex")
MOUNTED_AUTHORITY = Path("/capture/authority")
SERVICE_ROOT = Path(".agent/tools/workflow-capture")
UPSTREAM = "http://collector:4318"
COLLECTOR_OUTPUT = "collector-output"
COLLECTOR_IMAGE = (
    "otel/opentelemetry-collector-contrib@"
    "sha256:e40d61f10c9a617cca1ffd1be4d7e2b3a4b0f0001798ff07e58cabe82e618cea"
)
SOURCE = Path(__file__).resolve().parent
LEGACY_SERVICE_FILES = (
    "compose.yaml", "collector.yaml", "receiver.py", "register.py", "compose_service.py",
    "confirmation.py", "registry.py", "admission.py", "otlp.py", "record_sources.py",
)
SERVICE_FILES = (*LEGACY_SERVICE_FILES, "control.py")
SHARED_COMPOSE_PROJECT = "amiga"
CAPTURE_SERVICES = ("receiver", "collector")
LABEL = "com.docker.compose"
PROJECT_LABEL = "amiga.capture.project"
CONFIG_LABEL = "amiga.capture.config"
CLIENT_HEADER = "x-amiga-codex-capture"


@dataclass(frozen=True)
class Deadline:
    expires_ns: int

    @classmethod
    def start(cls, seconds):
        return cls(time.time_ns() + seconds * 1_000_000_000)

    def remaining(self, stage):
        seconds = (self.expires_ns - time.time_ns()) / 1_000_000_000
        if seconds <= 0:
            raise CaptureError(f"{stage}: the original startup deadline expired.")
        return seconds

    def pause(self, stage, delay):
        time.sleep(min(delay, self.remaining(stage)))


@dataclass(frozen=True)
class HostCheckouts:
    common: str
    windows: bool
    paths: frozenset

    def __call__(self, cwd):
        rules = ntpath if self.windows else posixpath
        text = str(cwd)
        if not rules.isabs(text) or ".." in rules.normcase(text).replace("\\", "/").split("/"):
            return None
        return self.common if rules.normcase(rules.normpath(text)) in self.paths else None


class MountedProject(Project):
    """Use only the host-validated archive root and same-repository checkout paths."""

    def __init__(self):
        self.primary = MOUNTED_PROJECT
        self.archive = self.primary / ARCHIVE
        self.control = self.archive / "_receiver"
        self.identity = os.environ["CAPTURE_PROJECT_ID"]
        self.common = self.authority()["git_common_dir"]

    def authority(self):
        state = read_json(self.plain_path(MOUNTED_AUTHORITY / "compose-authority.json"))
        if state["project_id"] != self.identity or state["archive_ignored"] is not True:
            raise CaptureError("The mounted archive has no matching host Git validation.")
        return state

    def checkouts(self):
        state = self.authority()
        return HostCheckouts(self.common, state["windows"], frozenset(state["worktree_paths"]))

    def matches_common(self, value):
        rules = ntpath if self.authority()["windows"] else posixpath
        return rules.normcase(rules.normpath(value)) == rules.normcase(rules.normpath(self.common))

    def require_ignored(self, path):
        self.authority()
        if not self.plain_path(path).is_relative_to(self.archive) or path == self.archive:
            raise CaptureError("The selected path is outside the ignored host archive.")


def verify_archive(project):
    project.plain_path(project.archive)
    if git(project.primary, "check-ignore", "-q", "--", ARCHIVE.as_posix() + "/").returncode:
        raise CaptureError("Git must ignore the entire selected capture archive before Compose writes to it.")


def verify_service_root(project):
    root = project.plain_path(project.primary / SERVICE_ROOT)
    tracked = git(project.primary, "ls-files", "--", SERVICE_ROOT.as_posix())
    if tracked.returncode or tracked.stdout:
        raise CaptureError("Compose settings: the service directory must be untracked.")
    if git(project.primary, "check-ignore", "-q", "--", SERVICE_ROOT.as_posix() + "/").returncode:
        raise CaptureError("Compose settings: Git must ignore the entire service directory.")
    return root


def write_authority(project, directory, codex_home, cwd):
    verify_archive(project)
    result = git(project.primary, "worktree", "list", "--porcelain", "-z")
    if result.returncode:
        raise CaptureError("Git could not enumerate the selected checkout's linked worktrees.")
    candidates = [cwd.resolve()]
    candidates.extend(Path(os.fsdecode(field.removeprefix(b"worktree ")))
                      for field in result.stdout.split(b"\0") if field.startswith(b"worktree "))
    checked = set()
    for candidate in dict.fromkeys(candidates):
        try:
            if git_path(candidate, "--git-common-dir") == project.common:
                checked.add(os.path.normcase(str(candidate.resolve())))
        except (OSError, CaptureError):
            if candidate == candidates[0]:
                raise
    path = directory / "compose-authority.json"
    project.require_ignored(path)
    directory.mkdir(parents=True, exist_ok=True)
    write_json(path, {
        "project_id": project.identity,
        "git_common_dir": str(project.common),
        "archive_ignored": True,
        "windows": os.name == "nt",
        "worktree_paths": sorted(checked),
        "rollout_source": path_identity(codex_home),
    })


def saved_settings(project):
    legacy_paths = [
        project.control / "compose", project.control / "compose-authority.json",
        *project.control.glob("compose-settings*.json"),
    ]
    if any(path.exists() or path.is_symlink() for path in legacy_paths):
        raise CaptureError("Compose settings: migrate the legacy service files out of the writable archive.")
    path = project.plain_path(project.primary / SERVICE_ROOT / "compose-settings.json")
    if path.exists():
        verify_service_root(project)
        project.require_ignored(path)
        return read_json(path)
    return None


def docker(deadline, stage, args, *, env=None, attempt_seconds=None, input=None):
    budget = deadline.remaining(stage)
    try:
        result = subprocess.run(
            ["docker", *args], capture_output=True, text=True, encoding="utf-8",
            errors="replace", timeout=min(budget, attempt_seconds or budget), env=env, input=input,
        )
    except FileNotFoundError as error:
        raise CaptureError(f"{stage}: Docker is unavailable on PATH.") from error
    except subprocess.TimeoutExpired as error:
        deadline.remaining(stage)
        raise CaptureError(f"{stage}: Docker did not answer in time.") from error
    deadline.remaining(stage)
    return result


class ComposeService:
    def __init__(self, project, args, deadline, *, revision=None):
        self.project = project
        self.deadline = deadline
        self.settings = saved_settings(project)
        self.new = self.settings is None
        if self.new and revision is not None:
            raise CaptureError("Compose identity: no service is provisioned for the primary checkout.")
        if self.new:
            self.settings = {
                "project_id": project.identity, "compose_project": f"amiga-capture-{project.identity[:24]}",
                "port": args.port or 4318, "codex_home": str(args.codex_home.resolve()),
                "collector_image": args.collector_image or COLLECTOR_IMAGE, "token": secrets.token_hex(32),
                "client_token": secrets.token_hex(32),
                "user": "10001:10001" if os.name == "nt" else f"{os.getuid()}:{os.getgid()}",
            }
        elif (self.settings["project_id"] != project.identity
              or any(getattr(args, key, None) is not None and str(getattr(args, key)) != self.settings[key]
                     for key in ("collector_image",))
              or (getattr(args, "port", None) is not None and args.port != self.settings["port"])
              or (getattr(args, "codex_home", None) is not None
                  and Path(self.settings["codex_home"]) != args.codex_home.resolve())):
            raise CaptureError("Compose identity: the requested service settings differ from the saved project.")
        self.name = self.settings.get("compose_project")
        if not isinstance(self.name, str) or not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", self.name):
            raise CaptureError("Compose identity: the saved Compose project name is invalid.")
        self.port = self.settings["port"]
        self.codex_home = Path(self.settings["codex_home"])
        self.root = verify_service_root(project)
        for directory in (self.codex_home / "sessions", self.codex_home / "archived_sessions"):
            if not directory.is_dir() or directory.resolve() != directory:
                raise CaptureError(f"Session inputs: the selected Codex home needs a plain {directory.name} directory.")
            if any(directory.is_relative_to(root) or root.is_relative_to(directory)
                   for root in (project.archive, self.root)):
                raise CaptureError("Session inputs: session directories must be separate from the capture paths.")
        if revision == "saved":
            saved_id = self.settings["config_id"]
            if not isinstance(saved_id, str) or not re.fullmatch(r"[0-9a-f]{64}", saved_id):
                raise CaptureError("Compose identity: the saved service revision is invalid.")
            snapshot = project.plain_path(self.root / "compose" / saved_id)
            names = {path.name for path in snapshot.iterdir()} if snapshot.is_dir() else set()
            files = SERVICE_FILES if "control.py" in names else LEGACY_SERVICE_FILES
            if names != set(files):
                raise CaptureError("Compose configuration: the saved service snapshot is incomplete or unexpected.")
            self.contents = {name: project.plain_path(snapshot / name).read_bytes() for name in files}
        else:
            self.contents = {name: (SOURCE / name).read_bytes().replace(b"\r\n", b"\n")
                             for name in SERVICE_FILES}
        identity = hashlib.sha256()
        service_settings = {key: value for key, value in self.settings.items() if key != "config_id"}
        identity.update(json.dumps(service_settings, sort_keys=True).encode())
        for name, data in self.contents.items():
            identity.update(name.encode() + b"\0" + hashlib.sha256(data).digest())
        self.config_id = identity.hexdigest()
        if not self.new and revision != "upgrade" and self.settings["config_id"] != self.config_id:
            if revision == "saved":
                raise CaptureError("Compose configuration: the saved service snapshot differs from its revision.")
            raise CaptureError("Compose identity: the saved project uses a different capture service revision.")
        self.code = self.root / "compose" / self.config_id
        self.authority = self.root / "authority"
        self.env = {
            **os.environ,
            "COMPOSE_REMOVE_ORPHANS": "false",
            "CAPTURE_PROJECT_ID": project.identity,
            "CAPTURE_CONFIG_ID": self.config_id,
            "CAPTURE_COMPOSE_PROJECT": self.name,
            "CAPTURE_ARCHIVE": str(project.archive),
            "CAPTURE_COLLECTOR_OUTPUT": str(project.control / COLLECTOR_OUTPUT),
            "CAPTURE_CODE": str(self.code),
            "CAPTURE_AUTHORITY": str(self.authority),
            "CAPTURE_SESSIONS": str(self.codex_home / "sessions"),
            "CAPTURE_ARCHIVED_SESSIONS": str(self.codex_home / "archived_sessions"),
            "CAPTURE_PORT": str(self.port),
            "CAPTURE_COLLECTOR_IMAGE": self.settings["collector_image"],
            "CAPTURE_COLLECTOR_TOKEN": self.settings["token"],
            "CAPTURE_CLIENT_TOKEN": self.settings["client_token"],
            "CAPTURE_USER": self.settings["user"],
        }

    def call(self, stage, *args, input=None):
        result = docker(self.deadline, stage, args, env=self.env, input=input)
        if result.returncode:
            detail = result.stderr.strip() or result.stdout.strip()
            # A failed Compose parse can echo an expanded secret.
            for name in ("token", "client_token"):
                detail = detail.replace(self.settings[name], "[redacted]")
            raise CaptureError(f"{stage}: {detail or f'Docker exited with {result.returncode}.'}")
        return result.stdout

    def compose(self, stage, *args):
        return self.call(stage, "compose", "--project-name", self.name,
                         "--file", str(self.code / "compose.yaml"), *args)

    def wait_engine(self):
        stage = "Docker engine"
        while True:
            try:
                probe = docker(self.deadline, stage, ("info", "--format", "{{.OSType}}"), attempt_seconds=5)
                if probe.returncode == 0:
                    if probe.stdout.strip() != "linux":
                        raise CaptureError(f"{stage}: select the Linux container engine.")
                    return
            except CaptureError as error:
                if not str(error).endswith("did not answer in time."):
                    raise
            self.deadline.pause(f"{stage} unavailable", 0.5)

    def containers(self, *, transition_revision=None):
        stage = "Compose identity"
        ids = self.call(stage, "ps", "-aq", "--filter", f"label={LABEL}.project={self.name}").split()
        revisions = {self.config_id}
        if transition_revision is not None:
            revisions.add(transition_revision)
        containers = {}
        for item in json.loads(self.call(stage, "inspect", *ids)) if ids else []:
            labels = item["Config"]["Labels"]
            service = labels.get(f"{LABEL}.service")
            is_capture_service = service in CAPTURE_SERVICES
            if (self.name == SHARED_COMPOSE_PROJECT and not is_capture_service
                    and PROJECT_LABEL not in labels and CONFIG_LABEL not in labels):
                continue
            if (not is_capture_service or service in containers
                    or labels.get(PROJECT_LABEL) != self.project.identity
                    or labels.get(CONFIG_LABEL) not in revisions
                    or item["HostConfig"]["RestartPolicy"]["Name"] != "unless-stopped"):
                raise CaptureError(f"{stage}: an existing container has different capture ownership or settings.")
            containers[service] = item
        return containers

    def persist_code(self):
        for name, data in self.contents.items():
            path = self.code / name
            self.project.require_ignored(path)
            if path.exists():
                if path.read_bytes() != data:
                    raise CaptureError("Compose configuration: a saved service file differs from the selected revision.")
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                with path.open("xb") as stream:
                    stream.write(data)

    def persist(self, cwd):
        verify_archive(self.project)
        verify_service_root(self.project)
        self.persist_code()
        spool = self.project.control / COLLECTOR_OUTPUT
        self.project.require_ignored(spool)
        spool.mkdir(exist_ok=True)
        write_authority(self.project, self.authority, self.codex_home, cwd)
        if self.new:
            path = self.root / "compose-settings.json"
            self.project.require_ignored(path)
            write_json(path, {**self.settings, "config_id": self.config_id})

    def healthy(self, containers, status):
        if set(containers) != set(CAPTURE_SERVICES) or not all(
            item["State"]["Running"] for item in containers.values()
        ):
            return False
        try:
            report = status(self.port, timeout=min(2, self.deadline.remaining("Receiver and Collector readiness")))
        except (HTTPError, URLError, TimeoutError, ConnectionError):
            return False
        except ValueError as error:
            raise CaptureError("Receiver identity: the loopback endpoint did not return receiver status.") from error
        if (not isinstance(report, dict) or report.get("project_id") != self.project.identity
                or report.get("compose_project") != self.name or report.get("config_id") != self.config_id
                or report.get("rollout_source") != path_identity(self.codex_home)
                or report.get("upstream") != UPSTREAM or report.get("ready") is not True):
            raise CaptureError("Receiver identity: the loopback endpoint belongs to a different service.")
        return {**report, "containers": {key: item["Id"] for key, item in containers.items()}}

    def ensure(self, cwd, status):
        self.wait_engine()
        containers = self.containers()
        if not containers.get("receiver", {}).get("State", {}).get("Running"):
            with socket.socket() as probe:
                if os.name == "nt":
                    probe.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
                try:
                    probe.bind(("127.0.0.1", self.port))
                except OSError as error:
                    raise CaptureError(
                        "Receiver endpoint: the loopback port is occupied or unavailable."
                    ) from error
        self.persist(cwd)
        since = datetime.now(UTC).isoformat()
        if len(containers) != len(CAPTURE_SERVICES) or not all(
            item["State"]["Running"] for item in containers.values()
        ):
            self.compose("Compose recovery", "up", "--detach", "--no-recreate", *CAPTURE_SERVICES)
            containers = self.containers()
        return self.wait_ready(containers, status, since)

    def wait_ready(self, containers, status, since):
        while True:
            health = self.healthy(containers, status)
            if health:
                break
            self.check_logs(containers, since)
            self.deadline.pause("Receiver and Collector readiness", 0.25)
            containers = self.containers()
        self.check_logs(containers, since)
        return health

    def check_logs(self, containers, since):
        for service, item in containers.items():
            stage = f"{service.capitalize()} logs"
            result = docker(self.deadline, stage, ("logs", "--since", since, "--tail", "40", item["Id"]))
            if result.returncode:
                raise CaptureError(f"{stage}: Docker could not inspect current service logs.")
            if re.search(r"(?im)Capture processing failed|Cannot run the capture receiver|"
                         r"\t(error|fatal)\t|^\s*Error: ", result.stdout + result.stderr):
                raise CaptureError(f"{stage}: a new error was recorded. Inspect the selected container logs.")

    @property
    def has_control(self):
        return "control.py" in self.contents

    def legacy_stop(self, receiver_id, request):
        if self.has_control or request["arguments"]["action"] != "stop":
            raise CaptureError("Legacy Docker control is only for closing a verified pre-REST installation.")
        report = self.call(
            "Execution registration", "exec", "-i", receiver_id, "python", "-B",
            "/capture/code/register.py", "--mounted-request", input=json.dumps(request),
        )
        return json.loads(report)
