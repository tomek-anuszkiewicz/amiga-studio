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
    input_candidates = [
        workspace_dir / "06_chapters_continuations",
        workspace_dir / "05_chapters_raw",
        workspace_dir / "chapters"
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "07_chapters_tables"
    out_dir.mkdir(parents=True, exist_ok=True)

    prompt_path = Path(__file__).resolve().parent / "prompt_markdown_table.md"
    base_prompt = prompt_path.read_text(encoding="utf-8") if prompt_path.exists() else ""

    gemini = GeminiClient(config) if GeminiClient else None
    if gemini and gemini.is_available():
        print(f"[*] Table Worker LLM active ({gemini.default_model}, thinking: {gemini.thinking_level}).")
    else:
        print(f"[*] Table Worker LLM unavailable (no GEMINI_API_KEY). Using heuristic GFM table formatter.")

    chapter_files = sorted(list(input_dir.glob("*.json")))
    print(f"[*] Transforming tables across {len(chapter_files)} chapter files...")

    transformed_count = 0

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

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

            transformed_count += 1

        target_file = out_dir / c_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

    print(f"[+] Stage 07 complete. Transformed {transformed_count} table blocks into {out_dir}.")


def prepare_table_tasks(workspace_dir: Path) -> int:
    """
    Extracts table nodes into workspace/tasks/tables/{node_id}.json and {node_id}.md
    for the Agent to inspect and transform.
    """
    input_candidates = [
        workspace_dir / "06_chapters_continuations",
        workspace_dir / "05_chapters_raw",
        workspace_dir / "chapters"
    ]
    chapters_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not chapters_dir:
        raise FileNotFoundError(f"Missing input chapters directory: {chapters_dir}")

    tasks_dir = workspace_dir / "tasks" / "tables"
    tasks_dir.mkdir(parents=True, exist_ok=True)

    count = 0
    for c_file in sorted(list(chapters_dir.glob("*.json"))):
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        for node in nodes:
            if node.get("type") != "table":
                continue
            if node.get("continuation_status") == "continuation":
                continue

            node_id = node.get("node_id")
            raw_text = node.get("raw_text", "")
            if node.get("continuation_status") == "head" and "merged_nodes" in node:
                child_texts = []
                for other in nodes:
                    if other.get("node_id") in node["merged_nodes"] and other.get("node_id") != node_id:
                        child_texts.append(other.get("raw_text", ""))
                if child_texts:
                    raw_text = raw_text + "\n" + "\n".join(child_texts)

            draft_md = format_simple_gfm_table(raw_text)
            if not draft_md:
                asset_ref = node.get("svg_path") or node.get("png_path") or ""
                draft_md = f"![Table]({asset_ref})\n"

            task_meta = {
                "node_id": node_id,
                "chapter_file": c_file.name,
                "page": node.get("page"),
                "continuation_status": node.get("continuation_status"),
                "merged_nodes": node.get("merged_nodes"),
                "svg_path": node.get("svg_path"),
                "png_path": node.get("png_path"),
                "raw_text_path": node.get("raw_text_path"),
                "raw_text": raw_text,
            }

            meta_file = tasks_dir / f"{node_id}.json"
            md_file = tasks_dir / f"{node_id}.md"

            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump(task_meta, f, indent=2)

            if not md_file.exists():
                with open(md_file, "w", encoding="utf-8") as f:
                    f.write(draft_md)

            count += 1

    print(f"[+] Prepared {count} table tasks in {tasks_dir}")
    return count


def apply_table_tasks(workspace_dir: Path) -> int:
    """
    Reads workspace/tasks/tables/{node_id}.md and injects rendered_markdown
    into workspace/07_chapters_tables/ without mutating prior stages.
    """
    tasks_dir = workspace_dir / "tasks" / "tables"
    if not tasks_dir.exists():
        print(f"[!] No table tasks directory found at {tasks_dir}")
        return 0

    input_candidates = [
        workspace_dir / "06_chapters_continuations",
        workspace_dir / "05_chapters_raw",
        workspace_dir / "chapters"
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "07_chapters_tables"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Collect rendered markdown for all tasks
    rendered_by_node = {}
    for meta_file in sorted(list(tasks_dir.glob("*.json"))):
        with open(meta_file, "r", encoding="utf-8") as f:
            meta = json.load(f)
        node_id = meta.get("node_id")
        md_file = tasks_dir / f"{node_id}.md"
        if md_file.exists():
            with open(md_file, "r", encoding="utf-8") as f:
                rendered_by_node[node_id] = f.read().strip()

    applied_count = 0
    for c_file in sorted(list(input_dir.glob("*.json"))):
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        for node in nodes:
            n_id = node.get("node_id")
            if n_id in rendered_by_node:
                node["rendered_markdown"] = rendered_by_node[n_id]
                applied_count += 1

        target_file = out_dir / c_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

    print(f"[+] Applied {applied_count} table tasks to {out_dir}.")
    return applied_count


def main():
    parser = argparse.ArgumentParser(description="Stage 07: Transform table nodes into Markdown/HTML tables")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--prepare", action="store_true", help="Prepare table tasks for the Agent in workspace/tasks/tables/")
    parser.add_argument("--apply", action="store_true", help="Apply Agent's edited tables from workspace/tasks/tables/ back to chapters")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    if args.prepare:
        prepare_table_tasks(workspace_dir)
        return

    if args.apply:
        apply_table_tasks(workspace_dir)
        return

    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_tables(workspace_dir, config)


if __name__ == "__main__":
    main()
