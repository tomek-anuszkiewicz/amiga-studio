"""Run native Graphify query/path/explain without query stamps or query logs."""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys

# Reading must not create bytecode in the checkout, including on interpreter relaunch.
sys.dont_write_bytecode = True
if __package__:
    from . import refresh
else:
    import refresh


def main():
    for stream in (sys.stdout, sys.stderr):
        if stream is not None and hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("query", "path", "explain"))
    parser.add_argument("terms", nargs="+")
    parser.add_argument("--graph", type=Path, required=True, help="Explicit graph.json path")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2],
                        help="Checkout for relative graph paths; defaults to this script's checkout")
    parser.add_argument("--budget", type=int, help="Native query token budget (query only)")
    args = parser.parse_args()
    if len(args.terms) != (2 if args.operation == "path" else 1):
        parser.error("path requires two symbols; query/explain require one quoted question or symbol")
    if args.budget is not None and args.operation != "query":
        parser.error("--budget applies only to query")
    graph = (args.root / args.graph).resolve()
    if not graph.is_file():
        parser.error("Selected graph is missing; use direct source inspection. Build it explicitly "
                     "with refresh.py --graph project|reference or bootstrap -Graphify.")
    os.environ["GRAPHIFY_OUT"] = "graphify-out"
    os.environ["GRAPHIFY_QUERY_LOG_DISABLE"] = "1"
    os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
    try:
        try:
            cli = refresh.import_module("graphify.cli")
        except ModuleNotFoundError as exc:
            if exc.name != "graphify":
                raise
            return subprocess.run([str(refresh.installed_python()), "-B", str(Path(__file__).resolve()),
                                   *sys.argv[1:]]).returncode
        if refresh.metadata.version("graphifyy") != refresh.GRAPHIFY_VERSION:
            raise RuntimeError(f"Graphify {refresh.GRAPHIFY_VERSION} is required.")
        # 0.9.74 otherwise unconditionally writes cache/last_query_stamp, even
        # with query logging disabled. Suppress only this incidental writer in
        # this process; the installed package and native traversal are unchanged.
        original_stamp = cli._touch_query_stamp
        original_args = sys.argv
        cli._touch_query_stamp = lambda _path: None
        sys.argv = ["graphify", args.operation, *args.terms, "--graph", str(graph)]
        if args.budget is not None:
            sys.argv.extend(["--budget", str(args.budget)])
        try:
            cli.dispatch_command(args.operation)
        finally:
            cli._touch_query_stamp = original_stamp
            sys.argv = original_args
        return 0
    except (OSError, RuntimeError, subprocess.CalledProcessError, refresh.metadata.PackageNotFoundError) as exc:
        print(f"Read-only Graphify query failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
