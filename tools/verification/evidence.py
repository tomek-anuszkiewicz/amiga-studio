"""Bounded continuation evidence for pending changes against the current HEAD."""
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import uuid

from impact import language_data_input
from repository import git


VERSION = 2


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def file_digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def capture_inputs(root, contents=True):
    """Hash authored inputs and ignored local reference/fixture inputs, not outputs."""
    paths = set(git(root, "ls-files", "--cached", "--others", "--exclude-standard", "-z")
                .decode("utf-8").split("\0")) - {""}
    # Ignored manuals participate in frontmatter; ignored fixtures participate in tests.
    for directory in ("crates", ".cargo"):
        for parent, directories, names in os.walk(root / directory):
            directories[:] = [name for name in directories if name not in {".git", "target", "__pycache__"}]
            paths.update((Path(parent) / name).relative_to(root).as_posix() for name in names)
    paths.update(path.relative_to(root).as_posix()
                 for path in (root / "Obsidian/Amiga").rglob("*.md"))
    paths.update(path.relative_to(root).as_posix()
                 for path in (root / "ref_src/SingleStepTests-680x0/68000/v1").glob("*.json"))
    if (root / ".env").exists():
        paths.add(".env")
    result = {}
    for name in sorted(paths):
        path = root / name
        if path.is_symlink():
            raise ValueError(f"Cannot establish input ownership through a symlink: {name}")
        if path.is_file():
            info = path.stat()
            result[name] = file_digest(path) if contents else (info.st_size, info.st_mtime_ns, info.st_ctime_ns)
        elif path.exists():
            raise ValueError(f"Unsupported input (for example a submodule): {name}")
        else:
            result[name] = None
    language_scope = root / "tools/verification/checks/check_english.json"
    if language_scope.is_file():
        settings = json.loads(language_scope.read_text(encoding="utf-8"))
        settings.pop("file_words", None)
        result["@english-scan-scope"] = digest(settings)
    return result


def execution_conditions(root):
    """Store hashes, never environment values or private configuration contents."""
    versions = {}
    for command in (["cargo", "--version"], ["rustc", "-vV"], ["cargo", "llvm-cov", "--version"]):
        result = subprocess.run(command, cwd=root, capture_output=True, check=True)
        versions[" ".join(command)] = digest([result.stdout.hex(), result.stderr.hex()])
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    configuration = {str(directory / name): file_digest(directory / name) if (directory / name).is_file() else None
                     for directory in [cargo_home, *(parent / ".cargo" for parent in root.parents)]
                     for name in ("config", "config.toml")}
    packages = sorted((item.metadata["Name"], item.version) for item in importlib.metadata.distributions())
    # Tool-call correlation changes without changing build or test configuration.
    environment = {key: value for key, value in os.environ.items() if key != "CODEX_TOOL_CALL_ID"}
    return digest([environment, versions, configuration, packages, platform.platform(),
                   sys.version, sys.executable, file_digest(Path(sys.executable))])


def group_identity(name, commands, inputs, conditions):
    # Vocabulary and exact-file language data are independent of emulator execution.
    # Verification contracts consume these data, and must execute again after edits.
    selected = inputs if name in {"Strict English Policy", "Verification CLI contracts"} else {
        path: value for path, value in inputs.items() if not language_data_input(path)}
    return digest([VERSION, commands, selected, conditions])


class BatchEvidence:
    def __init__(self, root, directory, profile):
        self.root = root.resolve()
        self.directory = directory.resolve()
        if not self.directory.is_relative_to(self.root / ".agent/tmp"):
            raise ValueError("Batch evidence must be stored under .agent/tmp in this checkout")
        self.path = self.directory / f"{profile}.json"
        self.attempt = uuid.uuid4().hex
        self.record = {"version": VERSION, "batch": uuid.uuid4().hex,
                       "head": git(self.root, "rev-parse", "HEAD").decode().strip(),
                       "checkout": digest(str(self.root)), "profile": profile, "groups": {}}

    def load(self):
        """Read the current record only while holding the batch execution lock."""
        if not self.path.exists():
            return
        try:
            previous = json.loads(self.path.read_text(encoding="utf-8"))
        except (ValueError, OSError):
            print("[INFO] Unreadable evidence record; all required groups will execute.")
            return
        if not isinstance(previous, dict) or not isinstance(previous.get("groups"), dict) or not previous.get("batch"):
            print("[INFO] Incomplete evidence record; all required groups will execute.")
            return
        for key in ("version", "head", "checkout", "profile"):
            if previous.get(key) != self.record[key]:
                print(f"[INFO] Batch {key} changed; all required groups will execute.")
                return
        self.record = previous

    def reason(self, name, identity):
        previous = self.record["groups"].get(name)
        if not isinstance(previous, dict) or previous.get("status") != "passed":
            return "missing successful evidence"
        if previous.get("identity") != identity:
            return "inputs, command, selection or execution conditions changed"
        artifacts = previous.get("artifacts")
        if not isinstance(artifacts, dict) or not artifacts:
            return "missing evidence artifacts"
        for relative, expected in artifacts.items():
            path = (self.directory / relative).resolve()
            if not path.is_relative_to(self.directory) or not path.is_file() or file_digest(path) != expected:
                return "missing or corrupt evidence artifact"
        return None

    def save(self):
        self.directory.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_suffix(".tmp")
        temporary.write_text(json.dumps(self.record, indent=2) + "\n", encoding="utf-8")
        temporary.replace(self.path)

    def start(self, name, identity, commands, source):
        directory = self.directory / "verify" / self.attempt / digest(name)[:12]
        self.record["groups"][name] = {"status": "running", "identity": identity,
                                       "commands": commands, "source": source,
                                       "inputs": self.record.get("inputs"),
                                       "conditions": self.record.get("conditions"),
                                       "attempt": self.attempt, "artifacts": {}}
        self.save()
        directory.mkdir(parents=True)
        return directory

    def finish(self, name, directory, success, elapsed):
        if success and name == "Coverage":
            for filename in ("coverage.json", "summary.json", "gaps.md"):
                source = self.root / ".agent/tmp/coverage" / filename
                if not source.is_file():
                    success = False
                    break
                shutil.copy2(source, directory / filename)
        if success and any("tools/verification/run_singlestep.py" in command
                           for _, command, _ in self.record["groups"][name]["commands"]):
            source = self.directory / "singlestep"
            required = ("run.json", "timing.json", "summary.json", "execution.log")
            if not all((source / filename).is_file() for filename in required):
                success = False
            else:
                shutil.copytree(source, directory / "singlestep")
        files = [path for path in directory.rglob("*") if path.is_file()]
        manifest = self.record["groups"][name].get("inputs")
        if manifest:
            files.append(self.directory / manifest)
        artifacts = {path.relative_to(self.directory).as_posix(): file_digest(path) for path in files}
        item = self.record["groups"][name]
        item.update(status="passed" if success and artifacts else "failed",
                    elapsed_seconds=elapsed, artifacts=artifacts)
        self.save()
        return item["status"] == "passed"
