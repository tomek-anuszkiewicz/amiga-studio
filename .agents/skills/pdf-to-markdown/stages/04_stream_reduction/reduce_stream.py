#!/usr/bin/env python3
"""
stages/04_stream_reduction/reduce_stream.py:
Normalizes the sequential node stream:
1. Suppresses all header and footer nodes.
2. Welds consecutive prose nodes across page breaks and performs de-hyphenation.
3. Emits workspace/reduced_stream.json.
"""

import argparse
import json
import re
import sys
from pathlib import Path
import yaml


def de_hyphenate_and_join(text1: str, text2: str) -> str:
    """
    Deterministically joins text1 and text2, repairing split words across seams.
    """
    t1 = text1.rstrip()
    t2 = text2.lstrip()

    # Check if t1 ends with a hyphenated word: e.g. "instruc-"
    match = re.search(r"(\b\w+)-$ ", t1 + " ")
    if match and t2:
        prefix = match.group(1)
        # Extract first word of t2
        match_t2 = re.match(r"^(\w+)(.*)", t2, re.DOTALL)
        if match_t2:
            suffix = match_t2.group(1)
            rest_t2 = match_t2.group(2)
            joined_word = prefix + suffix
            base_t1 = t1[:match.start(1)]
            return f"{base_t1}{joined_word}{rest_t2}"

    # Check if sentence continues without terminal punctuation (. ! ? : ;)
    if t1 and not re.search(r"[.!?:]\s*$", t1) and t2 and not t2[0].isupper():
        return f"{t1} {t2}"

    return f"{t1}\n\n{t2}"


def reduce_stream(workspace_dir: Path, config: dict):
    raw_candidates = [
        workspace_dir / "03_raw_stream" / "raw_stream.json",
        workspace_dir / "raw_stream.json"
    ]
    raw_stream_path = next((p for p in raw_candidates if p.exists()), None)
    if not raw_stream_path:
        raise FileNotFoundError(f"Missing raw_stream.json in {workspace_dir}")

    with open(raw_stream_path, "r", encoding="utf-8") as f:
        raw_nodes = json.load(f)

    print(f"[*] Reducing stream of {len(raw_nodes)} nodes from {raw_stream_path.name}...")

    reduced_nodes = []
    skipped_count = 0

    for node in raw_nodes:
        n_type = node.get("type")

        # 1. Suppress headers and footers
        if n_type in ("header", "footer"):
            skipped_count += 1
            continue

        # 2. Check if we can weld with the preceding node (prose + prose)
        if reduced_nodes and reduced_nodes[-1]["type"] == "prose" and n_type == "prose":
            prev = reduced_nodes[-1]
            # Weld text
            prev["raw_text"] = de_hyphenate_and_join(prev["raw_text"], node["raw_text"])
            # Update end bbox / page metadata
            prev["page_end"] = node["page"]
            if "welded_nodes" not in prev:
                prev["welded_nodes"] = [prev["node_id"]]
            prev["welded_nodes"].append(node["node_id"])
            continue

        # Add node
        node_copy = dict(node)
        node_copy["page_start"] = node["page"]
        node_copy["page_end"] = node["page"]
        reduced_nodes.append(node_copy)

    out_dir = workspace_dir / "04_reduced_stream"
    out_dir.mkdir(parents=True, exist_ok=True)
    reduced_stream_path = out_dir / "reduced_stream.json"
    with open(reduced_stream_path, "w", encoding="utf-8") as f:
        json.dump(reduced_nodes, f, indent=2)

    # Legacy copy for flat access
    with open(workspace_dir / "reduced_stream.json", "w", encoding="utf-8") as f:
        json.dump(reduced_nodes, f, indent=2)

    print(f"[+] Stage 04 complete. Suppressed {skipped_count} headers/footers.")
    print(f"    Reduced {len(raw_nodes)} -> {len(reduced_nodes)} nodes in {reduced_stream_path}")


def main():
    parser = argparse.ArgumentParser(description="Stage 04: Stream reduction and prose welding")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    reduce_stream(workspace_dir, config)


if __name__ == "__main__":
    main()
