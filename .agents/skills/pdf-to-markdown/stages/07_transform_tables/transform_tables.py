#!/usr/bin/env python3
"""
stages/07_transform_tables/transform_tables.py:
Worker for table nodes in chapter streams:
1. Skips child continuation nodes (rendered as part of head).
2. For head or standalone tables, processes combined raw text & crops.
3. Converts simple tables to GFM Markdown (4-column format, Unicode arrows).
4. Converts complex spanned tables to HTML table.
5. Saves formatted output in node['rendered_markdown'].
"""

import argparse
import json
import re
import sys
from pathlib import Path
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def format_simple_gfm_table(raw_text: str) -> str:
    """
    Heuristic GFM table formatter from raw text.
    Replaces ASCII arrows with Unicode, escapes pipes, aligns columns.
    """
    lines = [l.strip() for l in raw_text.splitlines() if l.strip()]
    if not lines:
        return ""

    rows = []
    max_cols = 0
    for line in lines:
        # Split by tab or multiple spaces
        cells = [c.strip() for c in re.split(r"\t+|\s{3,}", line) if c.strip()]
        if cells:
            rows.append(cells)
            max_cols = max(max_cols, len(cells))

    if max_cols < 2:
        # Single column fallback
        return "\n".join(f"- {l}" for l in lines)

    # Normalize row lengths
    norm_rows = []
    for r in rows:
        norm = r + [""] * (max_cols - len(r))
        # Format hex and unicode arrows
        formatted_cells = []
        for c in norm:
            c = re.sub(r"->", "→", c)
            c = re.sub(r"<-", "←", c)
            c = re.sub(r"(\$[0-9A-Fa-f]{3,8})", r"`\1`", c)
            formatted_cells.append(c)
        norm_rows.append(formatted_cells)

    # Header
    header = norm_rows[0]
    separator = [":---"] * max_cols
    body = norm_rows[1:] if len(norm_rows) > 1 else []

    out = []
    out.append("| " + " | ".join(header) + " |")
    out.append("| " + " | ".join(separator) + " |")
    for b in body:
        out.append("| " + " | ".join(b) + " |")

    return "\n".join(out)


def process_tables(workspace_dir: Path, config: dict):
    chapters_dir = workspace_dir / "chapters"
    if not chapters_dir.exists():
        raise FileNotFoundError(f"Missing chapters directory: {chapters_dir}")

    prompt_path = Path(__file__).resolve().parent / "prompt_markdown_table.md"
    base_prompt = prompt_path.read_text(encoding="utf-8") if prompt_path.exists() else ""

    gemini = GeminiClient(config) if GeminiClient else None
    if gemini and gemini.is_available():
        print(f"[*] Table Worker LLM active ({gemini.default_model}, thinking: {gemini.thinking_level}).")
    else:
        print(f"[*] Table Worker LLM unavailable (no GEMINI_API_KEY). Using heuristic GFM table formatter.")

    chapter_files = sorted(list(chapters_dir.glob("*.json")))
    print(f"[*] Transforming tables across {len(chapter_files)} chapter files...")

    transformed_count = 0

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        modified = False
        for node in nodes:
            if node.get("type") != "table":
                continue

            # Skip child continuation nodes
            if node.get("continuation_status") == "continuation":
                continue

            # Gather raw text from this node and any continuations
            raw_text = node.get("raw_text", "")
            if node.get("continuation_status") == "head" and "merged_nodes" in node:
                # Accumulate text from child continuation nodes
                child_texts = []
                for other in nodes:
                    if other.get("node_id") in node["merged_nodes"] and other.get("node_id") != node["node_id"]:
                        child_texts.append(other.get("raw_text", ""))
                if child_texts:
                    raw_text = raw_text + "\n" + "\n".join(child_texts)

            rendered = None
            if gemini and gemini.is_available() and base_prompt:
                full_prompt = f"{base_prompt}\n\n## Input Table Raw Text:\n```text\n{raw_text}\n```"
                rendered = gemini.generate_text(full_prompt)

            if not rendered:
                rendered = format_simple_gfm_table(raw_text)

            if rendered:
                node["rendered_markdown"] = rendered
            else:
                asset_ref = node.get("svg_path") or node.get("png_path") or ""
                node["rendered_markdown"] = f"![Table]({asset_ref})\n"

            modified = True
            transformed_count += 1

        if modified:
            with open(c_file, "w", encoding="utf-8") as f:
                json.dump(nodes, f, indent=2)

    print(f"[+] Stage 07 complete. Transformed {transformed_count} table blocks.")


def main():
    parser = argparse.ArgumentParser(description="Stage 07: Transform table nodes into Markdown/HTML tables")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_tables(workspace_dir, config)


if __name__ == "__main__":
    main()
