"""Restore one capture test's starting state into its private directory."""

from functools import cache
import hashlib
import json
import os
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
from zipfile import ZipFile


TESTS = Path(__file__).resolve().parent
CAPTURE = TESTS.parent
REPOSITORY = CAPTURE.parents[2]
VERIFICATION_TESTS = CAPTURE.parents[1] / "verification/tests"
sys.path.insert(0, str(VERIFICATION_TESTS))
from git_fixtures import restore_repository

sys.path.insert(0, str(CAPTURE))
from compose_service import SERVICE_FILES
from registry import path_identity


ARCHIVE = TESTS / "fixtures/capture-repositories.zip"
REBUILD = "python tools/workflow/capture/tests/fixtures/build_capture_repositories.py"
SERVICE = "sessions/.agent/tools/workflow-capture"
MARKER = "@fixture:"
NO_SERVICE_CASES = frozenset({
    "test_removed_handoff_command_is_rejected_before_service_access",
    "test_removed_recover_command_is_rejected_before_service_access",
    "test_retired_cli_controls_are_rejected_before_service_access",
})


def fixture_git_environment():
    environment = {key: value for key, value in os.environ.items() if not key.upper().startswith("GIT_")}
    return {**environment, "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull}


def restore_capture(name, destination):
    restore_repository(name, destination, ARCHIVE)
    if name in ("proof", "service"):
        # The unborn repositories start with empty object and ref directories.
        (destination / ".git/objects").mkdir()
        (destination / ".git/refs").mkdir()
    if name == "proof":
        with (destination / ".gitignore").open("a", encoding="utf-8") as stream:
            stream.write(".agent/tools/\n")


def normalized_bytes(path):
    return path.read_bytes().replace(b"\r\n", b"\n")


def service_sources():
    return {name: normalized_bytes(CAPTURE / name) for name in SERVICE_FILES}


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def check_sources(kind, expected):
    paths = ({name: CAPTURE / name for name in SERVICE_FILES} if kind == "service"
             else {".gitignore": REPOSITORY / ".gitignore"})
    actual = {name: sha256(normalized_bytes(path)) if path.is_file() else None for name, path in paths.items()}
    changed = sorted(actual.keys() ^ expected.keys() |
                     {name for name in actual.keys() & expected.keys() if actual[name] != expected[name]})
    if changed:
        raise ValueError(f"Stale capture {kind} fixture ({', '.join(changed)}). Regenerate with {REBUILD}.")


def revision_digest(settings, contents):
    identity = hashlib.sha256(json.dumps(settings, sort_keys=True).encode())
    for name, data in contents.items():
        identity.update(name.encode() + b"\0" + hashlib.sha256(data).digest())
    return identity.hexdigest()


def json_bytes(value):
    return (json.dumps(value, indent=2) + "\n").encode()


def marker(name):
    return f"{MARKER}{name}@"


def bind_text(data, values):
    for name, value in values.items():
        # The markers occur in JSON string values, including JSON Lines.
        encoded = json.dumps(value).encode()
        token = marker(name).encode()
        if isinstance(value, str):
            data = data.replace(token, encoded[1:-1])
        else:
            data = data.replace(b'"' + token + b'"', encoded)
    return data


def bind_value(value, values):
    if isinstance(value, str) and value.startswith(MARKER):
        return values[value.removeprefix(MARKER).removesuffix("@")]
    return value


def relocate_history(root, history):
    """Update derived fields using the relocated archived bytes and observer inputs."""
    manifest_path = root / history["manifest"]
    observer_path = root / history["observer"]
    manifest = json.loads(manifest_path.read_bytes())
    for entry in manifest["files"]:
        if entry["kind"] in ("rollout", "registration"):
            data = (manifest_path.parent / entry["path"]).read_bytes()
            entry.update(size=len(data), sha256=sha256(data))
        if entry["kind"] == "rollout":
            entry["ranges"][0]["end"] = len(data)
            event = manifest["threads"][0]["last_event"]
            event.update(byte_start=len(data) - len(data.splitlines(keepends=True)[-1]), byte_end=len(data))
    observer = json.loads(observer_path.read_bytes())
    for row in observer["rows"].values():
        for check in row["checks"]:
            if check["kind"] == "source_range":
                check["end"] = Path(check["source"]).stat().st_size
    manifest_path.write_bytes(json_bytes(manifest))
    observer_path.write_bytes(json_bytes(observer))


def restore_case(case_id, destination, archive_path=ARCHIVE):
    with ZipFile(archive_path) as archive:
        inventory = json.loads(archive.read("cases.json"))
        case = json.loads(archive.read(f"cases/{inventory['cases'][case_id]}.json"))
        kind = case.get("sources")
        if kind:
            check_sources(kind, inventory["sources"][kind])

        @cache
        def blob(digest):
            return archive.read(f"blobs/{digest}")

        values = {name: str(destination / relative) for name, relative in case.get("paths", {}).items()}
        revisions = case.get("revisions", {})
        if revisions:
            primary = destination / "sessions"
            codex = destination / "codex"
            identity = path_identity(primary / ".git")
            values.update({
                "project": identity, "compose": f"amiga-capture-{identity[:24]}",
                "rollout": path_identity(codex), "worktree": os.path.normcase(str(primary)),
                "windows": os.name == "nt",
                "user": "10001:10001" if os.name == "nt" else f"{os.getuid()}:{os.getgid()}",
                "foreign": identity[:-1] + ("0" if identity[-1] != "0" else "1"),
            })
            for name, revision in revisions.items():
                settings = {key: bind_value(value, values) for key, value in revision["settings"].items()}
                prefix = f"{SERVICE}/compose/{marker(revision['code'])}/"
                contents = {filename: blob(case["files"][prefix + filename]) for filename in SERVICE_FILES}
                values[name] = revision_digest(settings, contents)

        def relocated(path):
            for name in revisions:
                path = path.replace(marker(name), values[name])
            return destination / path

        for relative in case["directories"]:
            relocated(relative).mkdir(parents=True, exist_ok=True)
        bindings = case.get("bindings", {})
        native_newlines = set(case.get("native_newlines", ()))
        line_separator = os.linesep.encode()
        for relative, digest in case["files"].items():
            path = relocated(relative)
            path.parent.mkdir(parents=True, exist_ok=True)
            data = blob(digest)
            fields = bindings.get(relative)
            if fields:
                data = bind_text(data, {name: values[name] for name in fields})
            if relative in native_newlines:
                data = data.replace(b"\n", line_separator)
            path.write_bytes(data)

    for relative in case.get("backlinks", []):
        admin = destination / f"primary/.git/worktrees/{relative}"
        (admin / "gitdir").write_bytes((destination / relative / ".git").as_posix().encode("utf-8") + b"\n")
    if "history" in case:
        relocate_history(destination, case["history"])


def case_directory(test):
    temporary = TemporaryDirectory()
    test.addCleanup(temporary.cleanup)
    root = Path(temporary.name).resolve()
    restore_case(test.id(), root)
    return root
