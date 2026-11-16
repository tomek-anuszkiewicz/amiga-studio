"""Build or refresh one independent Graphify corpus (project by default)."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from functools import cache
from importlib import import_module, metadata
import os
import json
from pathlib import Path
import shutil
import subprocess
import sys


GRAPHIFY_VERSION = "0.9.74"


def refresh(watch, root=Path("."), *, acquire_lock=True) -> bool:
    """Retain Graphify's rebuild; cache its immutable per-instance path results."""
    original = watch._StoredSourcePaths

    class MemoizedSourcePaths(original):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            for name in ("normalize", "absolute_identity", "identity", "in_watch_root"):
                setattr(self, name, cache(getattr(self, name)))

    watch._StoredSourcePaths = MemoizedSourcePaths
    try:
        force = os.environ.get("GRAPHIFY_FORCE", "").lower() in ("1", "true", "yes")
        return watch._rebuild_code(root, force=force, block_on_lock=True,
                                   **({} if acquire_lock else {"acquire_lock": False}))
    finally:
        watch._StoredSourcePaths = original


@contextmanager
def reference_code_only(detector):
    """Keep upstream extraction/reconciliation, limiting membership to code."""
    original_detect = detector.detect
    original_ignored = detector.ignored_predicate

    def detect(*args, **kwargs):
        result = original_detect(*args, **kwargs)
        result["files"] = {kind: files if kind == "code" else []
                           for kind, files in result["files"].items()}
        return result

    def ignored(*args, **kwargs):
        predicate = original_ignored(*args, **kwargs)
        return lambda path: predicate(path) or detector.classify_file(Path(path)) != "code"

    detector.detect = detect
    detector.ignored_predicate = ignored
    try:
        yield
    finally:
        detector.detect = original_detect
        detector.ignored_predicate = original_ignored


def build(watch, root: Path, graph: str) -> bool:
    """Bind all generated state and copied provenance to the selected checkout."""
    scan_root = root if graph == "project" else root / "ref_src"
    if not scan_root.is_dir():
        raise RuntimeError("Reference sources are missing; provision them with bootstrap -Sources.")
    detector = import_module("graphify.detect")
    if graph == "reference":
        with reference_code_only(detector):
            if not detector.detect(scan_root, gitignore=False)["files"]["code"]:
                raise RuntimeError("No supported reference code found; provision sources with bootstrap -Sources.")
            return rebuild_selected(watch, scan_root, gitignore=False)
    return rebuild_selected(watch, scan_root, gitignore=True)


def rebuild_selected(watch, root: Path, *, gitignore: bool) -> bool:
    out = root / "graphify-out"
    out.mkdir(parents=True, exist_ok=True)
    # Configure under the same lock as the rebuild. Never trust copied absolute
    # root sidecars or a previous extract's saved --no-gitignore setting.
    with watch._rebuild_lock(out, blocking=True) as acquired:
        if not acquired:
            return False
        (out / ".graphify_build.json").write_text(
            json.dumps({"gitignore": gitignore}), encoding="utf-8")
        (out / ".graphify_root").write_text(".", encoding="utf-8")
        previous = Path.cwd()
        os.chdir(root)
        try:
            return refresh(watch, root, acquire_lock=False)
        finally:
            os.chdir(previous)


def installed_python() -> Path:
    """Find the existing uv-managed Graphify interpreter; never provision tools."""
    uv = shutil.which("uv")
    if uv is None:
        raise RuntimeError("Run this command with the Python interpreter containing graphifyy.")
    result = subprocess.run([uv, "tool", "dir"], capture_output=True, text=True, check=True)
    environment = Path(result.stdout.strip()) / "graphifyy"
    interpreter = environment / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
    if not interpreter.is_file() or interpreter.absolute() == Path(sys.executable).absolute():
        raise RuntimeError("No existing Graphify interpreter found; run with its Python interpreter.")
    return interpreter


def run_python(interpreter: Path) -> int:
    """Wait for the refresh with Graphify's default deterministic startup seed."""
    environment = dict(os.environ)
    environment.setdefault("PYTHONHASHSEED", "0")
    return subprocess.run(
        [str(interpreter), str(Path(__file__).resolve()), *sys.argv[1:]], env=environment
    ).returncode


def main() -> int:
    for stream in (sys.stdout, sys.stderr):
        if stream is not None and hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--graph", choices=("project", "reference"), default="project")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2],
                        help="Target checkout (default: the checkout containing this script)")
    args = parser.parse_args()
    root = args.root.resolve()
    if not (root / ".graphifyignore").is_file():
        parser.error("Target checkout must contain .graphifyignore.")
    # Set before importing Graphify: several modules capture this at import time.
    os.environ["GRAPHIFY_OUT"] = "graphify-out"
    try:
        try:
            watch = import_module("graphify.watch")
        except ModuleNotFoundError as exc:
            if exc.name != "graphify":
                raise
            interpreter = installed_python()
            return run_python(interpreter)
        version = metadata.version("graphifyy")
        if version != GRAPHIFY_VERSION:
            raise RuntimeError(
                f"This refresh supports graphifyy {GRAPHIFY_VERSION}; installed version is {version}."
            )
        if "PYTHONHASHSEED" not in os.environ:
            return run_python(Path(sys.executable))
        if not build(watch, root, args.graph):
            return 1
        print(f"{args.graph}: code graph updated at "
              f"{root / ('ref_src/' if args.graph == 'reference' else '') / 'graphify-out'}. "
              "Semantic document extraction remains a separate project operation.")
        return 0
    except (OSError, RuntimeError, subprocess.CalledProcessError, metadata.PackageNotFoundError) as exc:
        print(f"Graphify refresh failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
