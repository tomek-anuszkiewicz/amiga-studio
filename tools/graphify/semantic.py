"""Prepare/apply host-agent semantic extraction for the project graph only."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

if __package__:
    from . import refresh
else:
    import refresh


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(root):
    from graphify.cache import check_semantic_cache
    from graphify.detect import detect

    out = root / "graphify-out"
    out.mkdir(exist_ok=True)
    prompt = root / ".agents/skills/TA-Graphify-extract-semantics/references/extraction-spec.md"
    detected = detect(root, gitignore=True)
    files = [f for kind in ("document", "paper", "image") for f in detected["files"][kind]]
    nodes, edges, hyperedges, uncached = check_semantic_cache(files, root=root, prompt_file=prompt)
    run = Path(tempfile.mkdtemp(prefix="semantic-", dir=out))
    state = {"files": {str(Path(f).relative_to(root)): digest(Path(f)) for f in files},
             "prompt_hash": digest(prompt), "uncached": uncached,
             "cached": {"nodes": nodes, "edges": edges, "hyperedges": hyperedges}}
    (run / "state.json").write_text(json.dumps(state, ensure_ascii=False), encoding="utf-8")
    (run / "pending.txt").write_text("\n".join(uncached), encoding="utf-8")
    print(f"{len(files)} project content files; {len(uncached)} need host extraction. Run: {run}")
    return run


def apply(root, run):
    from graphify.build import build_merge
    from graphify.cache import save_semantic_cache
    from graphify.detect import detect, save_manifest
    from graphify.export import to_json
    from graphify.watch import _rebuild_lock

    out = root / "graphify-out"
    run = run.resolve()
    if not run.is_relative_to(out) or run.parent != out:
        raise ValueError("Select a semantic run directory directly inside project graphify-out.")
    state = json.loads((run / "state.json").read_text(encoding="utf-8"))
    prompt = root / ".agents/skills/TA-Graphify-extract-semantics/references/extraction-spec.md"
    detected = detect(root, gitignore=True)
    files = [f for kind in ("document", "paper", "image") for f in detected["files"][kind]]
    current = {str(Path(f).relative_to(root)): digest(Path(f)) for f in files}
    if current != state["files"] or digest(prompt) != state["prompt_hash"]:
        raise ValueError("Project content or extraction prompt changed; prepare a new semantic run.")
    new = {"nodes": [], "edges": [], "hyperedges": []}
    allowed = {str(Path(f).resolve()) for f in state["uncached"]}
    for chunk in sorted(run.glob("chunk-*.json")):
        data = json.loads(chunk.read_text(encoding="utf-8"))
        for kind in new:
            for item in data.get(kind, []):
                source = item.get("source_file")
                if not source or str(Path(source).resolve()) not in allowed:
                    raise ValueError(f"Invalid source provenance in {chunk.name}: {source}")
                new[kind].append(item)
    represented = {str(Path(n["source_file"]).resolve()) for n in new["nodes"]}
    if represented != allowed:
        raise ValueError("Semantic chunks are incomplete; every pending source needs a document/entity node.")
    combined = {kind: state["cached"][kind] + new[kind] for kind in new}
    for items in combined.values():
        for item in items:
            item["_origin"] = "semantic"
    # Reconcile removed/excluded sources and establish structural outputs first.
    if not refresh.build(refresh.import_module("graphify.watch"), root, "project"):
        raise RuntimeError("Project structural refresh failed before semantic publication.")
    with _rebuild_lock(out, blocking=True) as acquired:
        if not acquired:
            raise RuntimeError("Project graph is locked.")
        graph = build_merge([combined], graph_path=out / "graph.json", root=root)
        if not to_json(graph, {}, str(out / "graph.json"), force=True):
            raise RuntimeError("Semantic graph publication failed.")
        # AST refresh short-circuits unchanged topology; explicitly regenerate
        # communities and all derived outputs for the merged semantic graph.
        subprocess.run([sys.executable, "-m", "graphify", "cluster-only", "--graph",
                        str(out / "graph.json"), "--no-label"], cwd=root, check=True)
        save_semantic_cache(new["nodes"], new["edges"], new["hyperedges"], root=root,
                            allowed_source_files=state["uncached"], prompt_file=prompt)
        save_manifest({kind: detected["files"][kind] for kind in ("document", "paper", "image")},
                      manifest_path=str(out / "manifest.json"), root=root,
                      scan_corpus=[f for values in detected["files"].values() for f in values])
    print("Project semantics applied. Token usage is unavailable unless separately measured by the host.")


def main():
    for stream in (sys.stdout, sys.stderr):
        if stream is not None and hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8", errors="replace")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "apply"))
    parser.add_argument("--graph", choices=("project",), required=True)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--run", type=Path, help="Run directory printed by prepare; required for apply")
    args = parser.parse_args()
    root = args.root.resolve()
    if not (root / ".graphifyignore").is_file():
        parser.error("Target checkout must contain .graphifyignore.")
    if args.action == "apply" and args.run is None:
        parser.error("apply requires --run")
    os.environ["GRAPHIFY_OUT"] = "graphify-out"
    try:
        try:
            refresh.import_module("graphify")
            interpreter = Path(sys.executable)
        except ModuleNotFoundError:
            interpreter = refresh.installed_python()
        if interpreter != Path(sys.executable) or "PYTHONHASHSEED" not in os.environ:
            environment = dict(os.environ)
            environment.setdefault("PYTHONHASHSEED", "0")
            return subprocess.run([str(interpreter), str(Path(__file__).resolve()), *sys.argv[1:]],
                                  env=environment).returncode
        if refresh.metadata.version("graphifyy") != refresh.GRAPHIFY_VERSION:
            raise RuntimeError(f"Graphify {refresh.GRAPHIFY_VERSION} is required.")
        os.chdir(root)
        if args.action == "prepare":
            prepare(root)
        else:
            apply(root, args.run)
        return 0
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as exc:
        print(f"Project semantic extraction failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
