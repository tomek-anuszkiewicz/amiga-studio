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
import shutil
import sys
from pathlib import Path
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

# Import CodexClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

from conversion import CodexClient


def process_tables(workspace_dir: Path, config: dict):
    input_dir = workspace_dir / "06_detect_continuations"

    out_dir = workspace_dir / "07_transform_tables"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in (path for path in out_dir.iterdir() if path.suffix == ".json"):
        f.unlink()

    # Setup 07_transform_tables/assets and synchronize upstream assets
    out_assets_dir = out_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)
    for f in out_assets_dir.glob("*"):
        f.unlink()

    src_assets = workspace_dir / "06_detect_continuations" / "assets"
    if src_assets.exists():
        for f in src_assets.glob("*"):
            if f.is_file():
                shutil.copy2(f, out_assets_dir / f.name)

    prompt_path = Path(__file__).resolve().parent / "prompt.md"
    base_prompt = prompt_path.read_text(encoding="utf-8")

    with CodexClient(config, stage="07_transform_tables", images=True) as codex:

        from concurrent.futures import ThreadPoolExecutor

        concurrency = int(config.get("llm", {}).get("concurrency", 1))
        print(f"[*] Table Worker LLM active ({codex.selected.model}). Transforming tables (concurrency={concurrency})...")

        chapter_files = sorted(list((path for path in input_dir.iterdir() if path.suffix == ".json")))
        transformed_count = 0

        chapters = {}
        metadata = {}
        all_tasks = []

        for c_file in chapter_files:
            with open(c_file, "r", encoding="utf-8") as f:
                chapter = json.load(f)
                nodes = chapter["nodes"]
            chapters[c_file] = nodes
            metadata[c_file] = chapter

            for idx, node in enumerate(nodes):
                if node.get("type") != "table":
                    continue
                if node.get("continuation_status") == "continuation":
                    node["rendered_markdown"] = ""
                    continue

                raw_text = node.get("raw_text", "")
                image_paths = []
                if node.get("continuation_status") == "head" and "merged_nodes" in node:
                    child_texts = []
                    if node.get("png_path"):
                        p = workspace_dir / node["png_path"]
                        image_paths.append(p)
                    for other in nodes:
                        if other.get("node_id") in node["merged_nodes"] and other.get("node_id") != node["node_id"]:
                            if other.get("type") == "table":
                                child_texts.append(other.get("raw_text", ""))
                                if other.get("png_path"):
                                    p = workspace_dir / other["png_path"]
                                    image_paths.append(p)
                    if child_texts:
                        raw_text = raw_text + "\n" + "\n".join(child_texts)
                else:
                    png_rel = node.get("png_path")
                    if png_rel:
                        p = workspace_dir / png_rel
                        image_paths.append(p)

                png_arg = image_paths if len(image_paths) > 1 else (image_paths[0] if image_paths else None)
                all_tasks.append((c_file, idx, raw_text, png_arg))

        def _render_table_task(task):
            c_file, idx, raw_text, png_arg = task
            extra_hint = ""
            if isinstance(png_arg, (list, tuple)) and len(png_arg) > 1:
                extra_hint = f"\n\nNote: This table spans {len(png_arg)} consecutive pages. Merge all rows from all pages into a SINGLE unified continuous table. Drop redundant repeated header rows between pages."
            full_prompt = f"{base_prompt}{extra_hint}\n\n## Input Table Raw Text:\n```text\n{raw_text}\n```"
            if png_arg:
                rendered = codex.generate_vision(full_prompt, image_path=png_arg)
            else:
                rendered = codex.generate_text(full_prompt)
            return c_file, idx, rendered

        if all_tasks:
            print(f"[*] Transforming {len(all_tasks)} tables across {len(chapters)} chapters (concurrency={concurrency})...")
            with ThreadPoolExecutor(max_workers=min(len(all_tasks), concurrency)) as executor:
                results = list(executor.map(_render_table_task, all_tasks))

            for c_file, idx, rendered in results:
                node = chapters[c_file][idx]
                if rendered:
                    node["rendered_markdown"] = rendered.strip() + "\n"
                    node_id = node.get("node_id")
                    if node_id:
                        for asset_file in out_assets_dir.glob(f"asset_{node_id}.*"):
                            asset_file.unlink()
                    if "merged_nodes" in node:
                        for m_id in node["merged_nodes"]:
                            for asset_file in out_assets_dir.glob(f"asset_{m_id}.*"):
                                asset_file.unlink()
                    node["png_path"] = None
                    node["svg_path"] = None
                else:
                    asset_ref = node.get("svg_path") or node.get("png_path") or ""
                    node["rendered_markdown"] = f"![Table]({asset_ref})\n"
                transformed_count += 1

        for c_file, nodes in chapters.items():
            purged_nodes = [
                n for n in nodes
                if n.get("continuation_status") != "absorbed_caption"
                and n.get("type") != "caption_continuation"
            ]
            target_file = out_dir / c_file.name
            with open(target_file, "w", encoding="utf-8") as f:
                json.dump({**metadata[c_file], "nodes": purged_nodes}, f, indent=2)

        print(f"[+] Stage 07 complete. Transformed {transformed_count} table blocks into {out_dir}.")


def main():
    parser = argparse.ArgumentParser(description="Stage 07: Transform table nodes into Markdown/HTML tables")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)


    process_tables(workspace_dir, config)


if __name__ == "__main__":
    main()
