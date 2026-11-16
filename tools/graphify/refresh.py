"""Refresh the unified graph with source-path memoization scoped to one rebuild."""

from __future__ import annotations

import argparse
from functools import cache
from importlib import import_module, metadata
import os
from pathlib import Path
import shutil
import subprocess
import sys


GRAPHIFY_VERSION = "0.9.74"


def refresh(watch) -> bool:
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
        return watch._rebuild_code(Path("."), force=force, block_on_lock=True)
    finally:
        watch._StoredSourcePaths = original


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
        [str(interpreter), str(Path(__file__).resolve())], env=environment
    ).returncode


def main() -> int:
    for stream in (sys.stdout, sys.stderr):
        if stream is not None and hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    if not Path(".graphifyignore").is_file():
        parser.error("Run from the repository root containing .graphifyignore.")
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
        if not refresh(watch):
            return 1
        print("Code graph updated. Semantic document extraction remains a separate operation.")
        return 0
    except (OSError, RuntimeError, subprocess.CalledProcessError, metadata.PackageNotFoundError) as exc:
        print(f"Graphify refresh failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
