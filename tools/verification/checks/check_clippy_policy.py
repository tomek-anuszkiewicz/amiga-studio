"""Enforce approved source-file/symbol permissions over Clippy diagnostics."""
import json
from pathlib import Path
import re
import subprocess
import sys


REPO_ROOT = Path(__file__).resolve().parents[3]
POLICY_FILE = "tools/verification/checks/clippy_permissions.json"
POLICY_LINTS = frozenset({"clippy::disallowed_types", "clippy::disallowed_methods"})
SYMBOL_MESSAGE = re.compile(r"use of a disallowed (?:type|method) `([^`]+)`")


def load_permissions(root):
    entries = json.loads((root / POLICY_FILE).read_text(encoding="utf-8"))
    if not isinstance(entries, list):
        raise ValueError("Clippy permissions must be a list")
    permissions = {}
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {"file", "lint", "symbol", "reason"}:
            raise ValueError("Each Clippy permission needs file, lint, symbol and reason")
        if not all(isinstance(value, str) and value.strip() for value in entry.values()):
            raise ValueError("Clippy permission fields must be nonempty strings")
        path = Path(entry["file"])
        if (path.is_absolute() or ".." in path.parts or "\\" in entry["file"]
                or not entry["file"].startswith("crates/") or path.suffix != ".rs"
                or any(mark in entry["file"] for mark in ("*", "?", "[", ":"))):
            raise ValueError("Clippy permissions require exact repository-relative Rust paths")
        if entry["lint"] not in POLICY_LINTS:
            raise ValueError(f"Unknown permission lint: {entry['lint']}")
        key = (entry["file"], entry["lint"], entry["symbol"])
        if key in permissions:
            raise ValueError(f"Duplicate Clippy permission: {key}")
        permissions[key] = entry["reason"]
    return permissions


def diagnostic_path(root, manifest, span):
    raw = Path(span["file_name"].replace("\\", "/"))
    bases = (root, Path(manifest).parent)
    candidates = {candidate.resolve() for base in bases
                  if (candidate := base / raw).is_file()}
    if len(candidates) != 1:
        raise ValueError(f"Unknown or ambiguous diagnostic source: {span['file_name']}")
    try:
        return candidates.pop().relative_to(root.resolve()).as_posix()
    except ValueError as error:
        raise ValueError("Policy diagnostic source is outside the repository") from error


def inspect_output(output, root, permissions):
    used = set()
    failures = []
    finished = False
    for line in output.splitlines():
        record = json.loads(line)
        if not isinstance(record, dict):
            raise ValueError("Cargo emitted a non-object JSON record")
        if record.get("reason") == "build-finished":
            if finished or not isinstance(record.get("success"), bool):
                raise ValueError("Invalid Cargo build completion record")
            finished = True
            if not record["success"]:
                failures.append("Cargo reported an unsuccessful build")
            continue
        if record.get("reason") != "compiler-message":
            continue
        message = record["message"]
        code = (message.get("code") or {}).get("code")
        if code not in POLICY_LINTS:
            if message.get("rendered"):
                print(message["rendered"], file=sys.stderr, end="")
            if message["level"] == "error":
                failures.append(message["message"])
            continue
        symbol = SYMBOL_MESSAGE.fullmatch(message["message"])
        if symbol is None:
            raise ValueError("Unsupported Clippy policy diagnostic wording")
        spans = [span for span in message["spans"] if span.get("is_primary")]
        if len(spans) != 1:
            raise ValueError("A policy diagnostic must have one primary source location")
        span = spans[0]
        path = diagnostic_path(root, record["manifest_path"], span)
        key = (path, code, symbol.group(1))
        if key in permissions:
            used.add(key)
        else:
            failures.append(f"{path}:{span['line_start']}: Unapproved {symbol.group(1)} ({code})")
    if not finished:
        failures.append("Missing Cargo build completion record")
    return used, failures


def cargo_commands():
    flags = ["--message-format=json", "--", "--force-warn", "clippy::disallowed_types",
             "--force-warn", "clippy::disallowed_methods"]
    return [
        ["cargo", "clippy", "--workspace", "--all-targets", *flags],
        ["cargo", "clippy", "--workspace", "--lib", "--target", "wasm32-unknown-unknown", *flags],
    ]


def main():
    try:
        permissions = load_permissions(REPO_ROOT)
        used = set()
        failures = []
        for command in cargo_commands():
            print("[RUN] " + subprocess.list2cmdline(command), flush=True)
            result = subprocess.run(command, cwd=REPO_ROOT, capture_output=True,
                                    text=True, encoding="utf-8", errors="strict")
            if result.stderr:
                print(result.stderr, file=sys.stderr, end="")
            found, errors = inspect_output(result.stdout, REPO_ROOT, permissions)
            used.update(found)
            failures.extend(errors)
            if result.returncode:
                failures.append(f"Cargo Clippy exited with code {result.returncode}")
        if not failures:
            failures.extend(f"Unused Clippy permission: {path}: {symbol} ({lint})"
                            for path, lint, symbol in sorted(permissions.keys() - used))
        for failure in failures:
            print(f"[FAIL] {failure}", file=sys.stderr)
        if failures:
            return 1
        print(f"[PASS] Clippy policy: native and Wasm; {len(used)} approved permissions in use")
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"[FAIL] Clippy policy: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
