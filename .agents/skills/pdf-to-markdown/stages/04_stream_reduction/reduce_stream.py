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
from typing import Optional
import yaml


# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def weld_prose_with_gemini(text1: str, text2: str, gemini: Optional[GeminiClient], prompt_template: str) -> str:
    """
    Uses Gemini LLM to evaluate cross-page paragraph continuation and perform accurate de-hyphenation.
    """
    t1 = text1.rstrip()
    t2 = text2.lstrip()

    if gemini and gemini.is_available() and prompt_template:
        prompt = (
            f"{prompt_template}\n\n"
            f"## Tail Text of Preceding Page:\n```text\n{t1[-300:]}\n```\n\n"
            f"## Head Text of Next Page:\n```text\n{t2[:300]}\n```\n"
        )
        res = gemini.generate_json(prompt)
        if isinstance(res, dict) and res.get("is_continuation"):
            dehyphen = res.get("de_hyphenated_word")
            if dehyphen and "-" in t1[-12:]:
                base1 = re.sub(r"\b\w+-\s*$", "", t1)
                rest2 = re.sub(r"^\w+\s*", "", t2)
                return f"{base1}{dehyphen} {rest2}"
            return f"{t1} {t2}"

    # Default clean separation
    return f"{t1}\n\n{t2}"


def reduce_stream(workspace_dir: Path, config: dict):
    raw_candidates = [
        workspace_dir / "03_raw_stream" / "raw_stream.json",
        workspace_dir / "raw_stream.json"
    ]
    raw_stream_path = next((p for p in raw_candidates if p.exists()), None)
    if not raw_stream_path:
        raise FileNotFoundError(f"Missing raw_stream.json in {workspace_dir}")

    prompt_path = Path(__file__).resolve().parent / "prompt_seam.md"
    prompt_template = prompt_path.read_text(encoding="utf-8") if prompt_path.exists() else ""

    gemini = GeminiClient(config) if GeminiClient else None
    if not gemini or not gemini.is_available():
        raise RuntimeError("GEMINI_API_KEY environment variable is required for Stage 04 stream reduction.")

    with open(raw_stream_path, "r", encoding="utf-8") as f:
        raw_nodes = json.load(f)

    print(f"[*] Reducing stream of {len(raw_nodes)} nodes from {raw_stream_path.name} using Gemini seam analyzer...")

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
            prev["raw_text"] = weld_prose_with_gemini(prev["raw_text"], node["raw_text"], gemini, prompt_template)
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
