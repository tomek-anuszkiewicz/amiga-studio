"""Query the repository Graphify graph within selected source directories."""

import argparse
import json
import shutil
import subprocess
import tempfile
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
GRAPH_PATH = REPOSITORY_ROOT / "graphify-out" / "graph.json"
TEMP_ROOT = REPOSITORY_ROOT / ".agent" / "tmp"
SCOPE_PREFIXES = {
    "project": ("crates/", "tools/", "tests/", "benchmarks/", "examples/"),
    "vamiga": ("ref_src/vAmiga/",),
    "minimig": ("ref_src/minimig/",),
    "arp": ("ref_src/amiga_replacement_project/",),
}


def filter_graph(graph: dict, scopes: list[str]) -> dict:
    prefixes = tuple(prefix for scope in scopes for prefix in SCOPE_PREFIXES[scope])

    def included(source_file: str | None) -> bool:
        path = (source_file or "").replace("\\", "/").removeprefix("./")
        return any(path.startswith(prefix) for prefix in prefixes)

    nodes = [node for node in graph["nodes"] if included(node.get("source_file"))]
    node_ids = {node["id"] for node in nodes}
    links = [
        link
        for link in graph["links"]
        if link["source"] in node_ids
        and link["target"] in node_ids
        and included(link.get("source_file"))
    ]
    metadata = dict(graph.get("graph", {}))
    metadata.pop("hyperedges", None)
    return {**graph, "nodes": nodes, "links": links, "graph": metadata, "hyperedges": []}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("question", help="Question or symbol to pass to Graphify")
    parser.add_argument(
        "--scope", action="append", required=True, choices=sorted(SCOPE_PREFIXES),
        help="Source scope; repeat to compare projects",
    )
    parser.add_argument("--budget", type=int, default=2000, help="Graphify output token budget")
    args = parser.parse_args()

    graphify = shutil.which("graphify")
    if graphify is None:
        parser.error("Graphify is not on PATH")
    if not GRAPH_PATH.is_file():
        parser.error(f"Graphify graph is missing: {GRAPH_PATH}")

    graph = json.loads(GRAPH_PATH.read_text(encoding="utf-8"))
    selected = filter_graph(graph, args.scope)
    if not selected["nodes"]:
        parser.error(f"No graph nodes found for scopes: {', '.join(args.scope)}")

    TEMP_ROOT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="graphify-scope-", dir=TEMP_ROOT) as directory:
        scoped_path = Path(directory) / "graph.json"
        scoped_path.write_text(json.dumps(selected, ensure_ascii=False), encoding="utf-8")
        print(f"Scopes: {', '.join(args.scope)}; {len(selected['nodes'])} nodes", flush=True)
        return subprocess.run(
            [graphify, "query", args.question, "--budget", str(args.budget), "--graph", str(scoped_path)],
            cwd=REPOSITORY_ROOT,
            check=False,
        ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
