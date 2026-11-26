"""Query the repository Graphify graph within selected source directories."""

import argparse
import json
import subprocess
import tempfile
import sys
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
TEMP_ROOT = REPOSITORY_ROOT / ".agent" / "tmp"


def filter_graph(graph: dict, scopes: list[str], root: Path) -> dict:
    prefixes = []
    for scope in scopes:
        directory = Path(scope)
        if directory.is_absolute() or ".." in directory.parts or ":" in scope or "\\" in scope:
            raise ValueError(f"Scope must be a relative directory inside the selected graph: {scope}")
        resolved = (root / directory).resolve()
        if not resolved.is_relative_to(root.resolve()) or not resolved.is_dir():
            raise ValueError(f"Scope is not a directory inside the selected graph: {scope}")
        if directory.as_posix() != ".":
            prefixes.append(directory.as_posix().rstrip("/") + "/")

    def included(source_file: str | None) -> bool:
        path = (source_file or "").replace("\\", "/").removeprefix("./")
        if not path or Path(path).is_absolute() or ":" in path or ".." in Path(path).parts:
            return False
        if not (root / path).resolve().is_relative_to(root.resolve()):
            return False
        return not prefixes or any(path.startswith(prefix) for prefix in prefixes)

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
    parser.add_argument("--graph", required=True, choices=("project", "reference"))
    parser.add_argument("--scope", action="append", default=[],
                        help="Directory relative to the selected graph root; repeat to select several")
    parser.add_argument("--budget", type=int, default=2000, help="Graphify output token budget")
    args = parser.parse_args()

    root = REPOSITORY_ROOT if args.graph == "project" else REPOSITORY_ROOT / "ref_src"
    graph_path = root / "graphify-out/graph.json"
    if not graph_path.is_file():
        parser.error(f"{args.graph} graph is missing. Run python tools/graphify/refresh.py "
                     f"--graph {args.graph}, or ./tools/bootstrap/bootstrap.ps1 -Graphify")

    graph = json.loads(graph_path.read_text(encoding="utf-8"))
    try:
        selected = filter_graph(graph, args.scope, root)
    except ValueError as exc:
        parser.error(str(exc))
    if not selected["nodes"]:
        parser.error(f"No graph nodes found for scopes: {', '.join(args.scope)}")

    TEMP_ROOT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="graphify-scope-", dir=TEMP_ROOT) as directory:
        scoped_path = Path(directory) / "graph.json"
        scoped_path.write_text(json.dumps(selected, ensure_ascii=False), encoding="utf-8")
        print(f"Graph: {args.graph}; scopes: {', '.join(args.scope) or 'all'}; "
              f"{len(selected['nodes'])} nodes; source root: {root}", flush=True)
        return subprocess.run(
            [sys.executable, str(REPOSITORY_ROOT / "tools/graphify/read.py"), "query", args.question,
             "--budget", str(args.budget), "--graph", str(scoped_path)],
            cwd=REPOSITORY_ROOT,
            check=False,
        ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
