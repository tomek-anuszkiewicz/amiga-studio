"""Capture selected task conditions and compare them with a saved inventory."""

import argparse
from datetime import datetime, timezone
from hashlib import file_digest, sha256
from importlib.metadata import distributions
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[3]
INVENTORY_FORMAT = 2
CONFIG = ".codex/config.toml"
REQUIREMENTS = (
    "tools/verification/requirements.txt",
    "tools/verification/checks/requirements.txt",
)
INPUTS = (
    "Cargo.lock", "Cargo.toml", *REQUIREMENTS, CONFIG,
    ".agents/references/workflow-contract.md",
    "tools/workflow/result.schema.json",
)
COMMANDS = (
    ("git", "--version"), ("rustc", "-Vv"), ("cargo", "--version"),
    ("rustup", "show", "active-toolchain"), ("rustup", "target", "list", "--installed"),
    ("rust-lld", "--version"), ("clang", "--version"), ("wasm-bindgen", "--version"),
    ("cargo", "llvm-cov", "--version"), ("py", "-0"), ("python", "--version"),
    ("pip", "--version"), ("node", "--version"), ("npm", "--version"),
    ("npm", "ls", "--global", "--depth=0", "--json"),
    ("docker", "--version"), ("docker", "version", "--format", "{{.Client.Version}} / {{.Server.Version}}"),
    ("docker", "compose", "version", "--short"), ("codex", "--version"), ("graphify", "--version"),
    ("powershell", "-NoProfile", "-Command", "$PSVersionTable.PSVersion.ToString()"),
)
VERSION = r"v?\d+(?:\.[a-zA-Z0-9]+)+(?:[-+][a-zA-Z0-9_.+-]+)?"
NAME = r"[@a-zA-Z0-9_][a-zA-Z0-9_.-]*(?:/[a-zA-Z0-9_.-]+)?"
IMAGE_NAME = r"[a-z0-9.-]+(?::\d+)?(?:/[a-z0-9_.-]+)*"
IMAGE_TAG = r":[a-zA-Z0-9_.-]+"
IMAGE_DIGEST = r"@sha256:[0-9a-f]{64}"
UNKNOWN = object()


def now():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    with path.open("rb") as source:
        return file_digest(source, "sha256").hexdigest()


def run(command, cwd=ROOT):
    # Rustup proxies must not fetch a missing toolchain while being probed.
    env = dict(os.environ, RUSTUP_AUTO_INSTALL="0", GIT_NO_LAZY_FETCH="1")
    return subprocess.run(command, cwd=cwd, env=env, capture_output=True, timeout=15)


def git_bytes(*args):
    result = run(("git", *args))
    return result.stdout if result.returncode == 0 else None


def allowed(value, pattern):
    return value if isinstance(value, str) and re.fullmatch(pattern, value) else None


def package_versions(output):
    try:
        dependencies = json.loads(output).get("dependencies", {})
        return {
            name: allowed(details.get("version"), VERSION)
            for name, details in dependencies.items() if allowed(name, NAME)
        }
    except (ValueError, AttributeError):
        return None


def probe_value(command, output):
    tool = command[0]
    if command[1] == "ls":
        return package_versions(output)
    if tool == "py":
        return "\n".join(re.findall(r"(?m)^ *(-V:\d+(?:\.\d+)*(?:[-.\w]+)?)", output))
    if tool == "pip":
        value = re.match(rf"pip ({VERSION}) from ", output)
        return f"pip {value[1]}" if value else None
    if command[1] == "image":
        return allowed(output, r"[a-z0-9_.:-]+(?:/[a-z0-9_.-]+)*@sha256:[0-9a-f]{64}")
    if tool == "docker" and command[1] == "version":
        return allowed(output, rf"{VERSION} / {VERSION}")
    if tool == "clang":
        version = re.search(rf"\bclang version ({VERSION})", output.partition("\n")[0])
        return f"clang version {version[1]}" if version else None
    if tool in {"rustc", "rustup"}:
        return allowed(output, r"[a-zA-Z0-9_.: ()\n-]+")
    first_line = output.partition("\n")[0]
    return allowed(first_line, rf"(?:[-a-zA-Z0-9 ]+ )?{VERSION}(?: \([a-f0-9]+ \d{{4}}-\d{{2}}-\d{{2}}\))?(?:, build [a-f0-9]+)?")


def probe(command, executable_hashes, cwd=ROOT):
    record = {"utc": now(), "command": list(command), "status": "unavailable"}
    executable = shutil.which(command[0])
    if executable is None:
        return record
    path = Path(executable)
    if path not in executable_hashes:
        try:
            executable_hashes[path] = digest(path)
        except OSError:
            executable_hashes[path] = None
    record.update(executable=path.name, executable_sha256=executable_hashes[path])
    try:
        result = run((executable, *command[1:]), cwd)
    except subprocess.TimeoutExpired:
        return dict(record, status="timeout")
    except OSError:
        return dict(record, status="error")
    record["exit_code"] = result.returncode
    if result.returncode:
        return dict(record, status="error")
    output = (result.stdout or result.stderr).decode("utf-8", errors="replace").strip()
    value = probe_value(command, output)
    if value is None:
        return dict(record, status="unrecognized")
    return dict(record, status="observed", result=value)


def requirement_summary(path):
    requirements = {}
    unresolved = []
    try:
        for number, line in enumerate((ROOT / path).read_text(encoding="utf-8-sig").splitlines(), 1):
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            match = re.fullmatch(r"([\w.-]+)\s*((?:(?:[<>=!~]{1,3})[\w.*+-]+,?)+)", line)
            if match:
                name, specifier = match.groups()
                requirements[name] = {
                    "specifier": specifier,
                    "pinned": re.fullmatch(r"==[^=*,]+", specifier) is not None,
                }
            else:
                unresolved.append(number)
    except (OSError, UnicodeError):
        return None
    return {"requirements": requirements, "unresolved_lines": unresolved}


def read_client(session_log, turn_id):
    if session_log is None:
        return {"status": "unavailable"}
    metadata = None
    context = None
    try:
        with session_log.open(encoding="utf-8-sig") as source:
            for line in source:
                row = json.loads(line)
                if row["type"] == "session_meta" and metadata is None:
                    metadata = row["payload"]
                if row["type"] == "turn_context":
                    candidate = row["payload"]
                    if turn_id is None or candidate.get("turn_id") == turn_id:
                        context = candidate
    except (OSError, UnicodeError, ValueError, KeyError, TypeError):
        return {"status": "error"}
    if metadata is None:
        return {"status": "unrecognized"}
    version = allowed(metadata.get("cli_version"), VERSION)
    source = metadata.get("source")
    surfaces = {"exec": "dedicated Codex CLI", "cli": "Codex CLI", "vscode": "VS Code"}
    turn_status = "observed" if context is not None else "unavailable"
    context = context or {}
    return {
        "status": "observed",
        "surface": surfaces.get(source) if isinstance(source, str) else None,
        "version": version if source in ("exec", "cli") else None,
        "backend_version": version if source not in ("exec", "cli") else None,
        "turn_status": turn_status,
        "model": allowed(context.get("model"), r"[\w.-]+"),
        "effort": allowed(context.get("effort"), r"none|minimal|low|medium|high|xhigh|max|ultra"),
        "turn_id": allowed(context.get("turn_id"), r"[a-fA-F0-9-]+"),
        "plugins": None,
    }


def selected_context():
    if Path("/.dockerenv").is_file() or Path("/run/.containerenv").is_file():
        return "container"
    if platform.system() == "Linux" and ("WSL_INTEROP" in os.environ or "microsoft" in platform.release().lower()):
        return "WSL"
    # Local Windows probes do not establish the host context in a virtual machine.
    return "native" if platform.system() == "Windows" else "unknown"


def input_hashes(paths, commit):
    blobs = {}
    checkout = {}
    for path in paths:
        content = git_bytes("cat-file", "blob", f"{commit}:{path}")
        blobs[path] = sha256(content).hexdigest() if content is not None else None
        try:
            checkout[path] = digest(ROOT / path)
        except OSError:
            checkout[path] = None
    return blobs, checkout


def capture(args):
    reference = git_bytes("rev-parse", "--verify", "--end-of-options", f"{args.governing_commit}^{{commit}}")
    if reference is None:
        raise ValueError("The governing commit is unavailable.")
    commit = reference.decode("ascii").strip()
    context = selected_context()
    hashes = {}
    probes = [probe(command, hashes) for command in COMMANDS]
    for image in args.docker_image:
        probes.append(probe(("docker", "image", "inspect", image, "--format", "{{index .RepoDigests 0}}"), hashes))

    paths = set(INPUTS)
    tracked_roles = git_bytes("ls-tree", "-r", "--name-only", "-z", commit, "--", ".codex/agents")
    if tracked_roles is not None:
        paths.update(path for path in tracked_roles.decode("utf-8").split("\0")
                     if re.fullmatch(r"\.codex/agents/TA-[\w-]+\.toml", path))
    paths.update(path.relative_to(ROOT).as_posix() for path in (ROOT / ".codex/agents").glob("TA-*.toml"))
    node_packages = {}
    projects = args.node_project or (["."] if (ROOT / "package.json").is_file() else [])
    for project in projects:
        target = (ROOT / project).resolve()
        if Path(project).is_absolute() or not target.is_relative_to(ROOT) or not (target / "package.json").is_file():
            raise ValueError("Each Node project must be a repository-relative directory with package.json.")
        paths.update((target / name).relative_to(ROOT).as_posix()
                     for name in ("package.json", "package-lock.json", "npm-shrinkwrap.json"))
        record = probe(("npm", "ls", "--depth=0", "--json"), hashes, target)
        node_packages[target.relative_to(ROOT).as_posix()] = record

    blobs, checkout = input_hashes(sorted(paths), commit)
    try:
        config = tomllib.loads((ROOT / CONFIG).read_text(encoding="utf-8-sig"))
        mcp = sorted(name for name in config.get("mcp_servers", {}) if allowed(name, r"[\w-]+"))
    except (OSError, UnicodeError, ValueError):
        mcp = None
    packages = {dist.metadata["Name"]: dist.version for dist in distributions()
                if allowed(dist.metadata["Name"], r"[\w.-]+") and allowed(dist.version, r"[\w.+!-]+")}
    selected = Path(sys.executable)
    try:
        selected_hash = digest(selected)
    except OSError:
        selected_hash = None
    return {
        "format_version": INVENTORY_FORMAT, "captured_at": now(), "governing_commit": commit,
        "host": {"os": platform.platform(), "architecture": platform.machine(),
                 "processors": os.cpu_count(), "native": True if context == "native" else
                 None if context == "unknown" else False, "context": context, "shell": args.shell,
                 "sdk": allowed(os.environ.get("WindowsSDKVersion", "").rstrip("\\/"), VERSION),
                 "native_compiler": allowed(os.environ.get("VCToolsVersion"), VERSION)},
        "probes": probes,
        "python": {"selected": selected.name, "selected_sha256": selected_hash,
                   "version": allowed(sys.version, r"[a-zA-Z0-9_.: ()\[\]+,\n-]+"), "venv": sys.prefix != sys.base_prefix,
                   "pip_version": packages.get("pip"), "packages": packages},
        "node": {"local_packages": node_packages},
        "declared_dependencies": list(REQUIREMENTS),
        "python_requirements": {path: requirement_summary(path) for path in REQUIREMENTS},
        "repository_input_git_blob_sha256": blobs,
        "repository_input_checkout_sha256": checkout,
        "role_settings": None,  # Configured roles are recoverable from the governing commit.
        "declared_mcp_servers": mcp,
        "client": read_client(args.session_log, args.turn_id),
        "limits": [
            "Executable probes select from PATH. Python launcher registrations are probed where available.",
            "Node global packages are installed observations. Tool use and plugin versions require runtime evidence.",
            "Client fields come from the selected session log. With no turn ID, settings describe its latest recorded context.",
            "A null input hash means no bytes were obtained from that source. No configuration or error text is copied.",
        ],
    }


def observed(record):
    return record["result"] if record["status"] == "observed" else UNKNOWN


def inventory_fields(inventory):
    fields = {key: inventory[key] for key in (
        "host", "python", "python_requirements", "repository_input_git_blob_sha256",
        "repository_input_checkout_sha256", "declared_mcp_servers", "role_settings",
    )}
    client = inventory["client"]
    fields["client"] = {key: client.get(key)
                        for key in ("surface", "version", "backend_version", "model", "effort", "plugins")}
    fields["declared_dependencies"] = dict.fromkeys(inventory["declared_dependencies"], True)
    fields["node_local_packages"] = {key: observed(value) for key, value in inventory["node"]["local_packages"].items()}
    fields["probes"] = {" ".join(record["command"]): observed(record) for record in inventory["probes"]}
    fields["executables"] = {
        " ".join(record["command"]): {"name": record.get("executable", "").casefold(),
                                    **({"sha256": record["executable_sha256"]}
                                       if "executable_sha256" in record else {})}
        for record in inventory["probes"] if record.get("executable")
    }
    return fields


def compare(saved, fresh):
    report = {status: [] for status in ("changed", "missing", "unknown")}

    def visit(path, previous, current):
        if previous is UNKNOWN or current is UNKNOWN or previous is None or current is None:
            report["unknown"].append(path)
        elif isinstance(previous, dict) and isinstance(current, dict):
            for key in sorted(previous.keys() | current.keys()):
                child = f"{path}/{key}" if path else key
                if key not in previous:
                    report["unknown"].append(child)
                elif key not in current:
                    report["missing"].append(child)
                else:
                    visit(child, previous[key], current[key])
        elif previous != current:
            report["changed"].append({"item": path, "saved": previous, "fresh": current})

    visit("", inventory_fields(saved), inventory_fields(fresh))
    return report


def load_inventory(path):
    value = json.loads(path.read_text(encoding="utf-8"))
    if (not isinstance(value, dict) or value.get("format_version") != INVENTORY_FORMAT
            or not isinstance(value.get("probes"), list)):
        raise ValueError(f"Expected an environment inventory in format {INVENTORY_FORMAT}.")
    return value


def image_reference(value):
    if not allowed(value, rf"{IMAGE_NAME}(?:{IMAGE_TAG}|(?:{IMAGE_TAG})?{IMAGE_DIGEST})"):
        raise argparse.ArgumentTypeError("Expected an image name with a tag or SHA-256 digest, without a URL.")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="operation", required=True)
    collect = commands.add_parser("capture", help="Write a fresh task environment inventory.")
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--governing-commit", default="HEAD")
    collect.add_argument("--shell", help="Record the selected shell name.")
    collect.add_argument("--rollout", dest="session_log", type=Path)
    collect.add_argument("--turn-id")
    collect.add_argument("--docker-image", type=image_reference, action="append", default=[])
    collect.add_argument("--node-project", action="append", default=[])
    diff = commands.add_parser("compare", help="Report differences without altering either inventory.")
    diff.add_argument("saved", type=Path)
    diff.add_argument("fresh", type=Path)
    diff.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.operation == "capture":
        if args.turn_id and args.session_log is None:
            parser.error("--turn-id requires a session log.")
        if args.shell and not allowed(args.shell, r"[a-zA-Z0-9 _-]{1,40}"):
            parser.error("--shell expects a shell name without a path.")
    try:
        if args.operation == "capture":
            result = capture(args)
        else:
            result = compare(load_inventory(args.saved), load_inventory(args.fresh))
        content = json.dumps(result, indent=2, ensure_ascii=True) + "\n"
        if args.output is None:
            print(content, end="")
        else:
            with args.output.open("x", encoding="utf-8", newline="\n") as output:
                output.write(content)
    except FileExistsError:
        parser.exit(2, "The output already exists. Choose a new path.\n")
    except (OSError, UnicodeError, subprocess.TimeoutExpired):
        parser.exit(2, "Could not read or write the selected inventory inputs.\n")
    except (ValueError, KeyError, TypeError, AttributeError):
        if args.operation == "compare":
            parser.exit(2, f"Expected valid environment inventories in format {INVENTORY_FORMAT}.\n")
        parser.exit(2, "The selected environment inputs are invalid.\n")


if __name__ == "__main__":
    main()
