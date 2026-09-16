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
import shutil
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


def process_tables(workspace_dir: Path, config: dict):
    input_candidates = [
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "07_transform_tables"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in out_dir.glob("*.json"):
        f.unlink()

    # Setup 07_transform_tables/assets and synchronize upstream assets
    out_assets_dir = out_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)
    for f in out_assets_dir.glob("*"):
        try:
            f.unlink()
        except Exception:
            pass

    asset_candidates = [
        workspace_dir / "06_detect_continuations" / "assets",
        workspace_dir / "05_chapter_partition" / "assets",
        workspace_dir / "04_stream_reduction" / "assets",
        workspace_dir / "03_build_raw_stream" / "assets",
        workspace_dir / "assets",
    ]
    src_assets = next((p for p in asset_candidates if p.exists()), None)
    if src_assets:
        for f in src_assets.glob("*"):
            if f.is_file():
                shutil.copy2(f, out_assets_dir / f.name)

    prompt_path = Path(__file__).resolve().parent / "prompt.md"
    if not prompt_path.exists():
        prompt_path = Path(__file__).resolve().parent / "prompt_markdown_table.md"
    base_prompt = prompt_path.read_text(encoding="utf-8") if prompt_path.exists() else ""

    gemini = GeminiClient(config) if GeminiClient else None
    if not gemini or not gemini.is_available():
        raise RuntimeError("GEMINI_API_KEY environment variable is required for Stage 07 table transformation.")

    from concurrent.futures import ThreadPoolExecutor

    concurrency = int(config.get("llm", {}).get("concurrency", 8))
    print(f"[*] Table Worker LLM active ({gemini.default_model}). Transforming tables (concurrency={concurrency})...")

    chapter_files = sorted(list(input_dir.glob("*.json")))
    transformed_count = 0

    def _render_table_task(task):
        idx, raw_text, png_path = task
        full_prompt = f"{base_prompt}\n\n## Input Table Raw Text:\n```text\n{raw_text}\n```"
        if png_path and png_path.exists():
            rendered = gemini.generate_vision(full_prompt, image_path=png_path)
        else:
            rendered = gemini.generate_text(full_prompt)
        return idx, rendered

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        tasks = []
        for idx, node in enumerate(nodes):
            if node.get("type") != "table":
                continue
            if node.get("continuation_status") == "continuation":
                continue

            raw_text = node.get("raw_text", "")
            if node.get("continuation_status") == "head" and "merged_nodes" in node:
                child_texts = []
                for other in nodes:
                    if other.get("node_id") in node["merged_nodes"] and other.get("node_id") != node["node_id"]:
                        child_texts.append(other.get("raw_text", ""))
                if child_texts:
                    raw_text = raw_text + "\n" + "\n".join(child_texts)

            png_rel = node.get("png_path")
            png_path = workspace_dir / png_rel if png_rel else None
            tasks.append((idx, raw_text, png_path))

        if tasks:
            with ThreadPoolExecutor(max_workers=min(len(tasks), concurrency)) as executor:
                results = list(executor.map(_render_table_task, tasks))

            for idx, rendered in results:
                node = nodes[idx]
                if rendered:
                    node["rendered_markdown"] = rendered.strip() + "\n"
                    node_id = node.get("node_id")
                    if node_id:
                        for asset_file in out_assets_dir.glob(f"asset_{node_id}.*"):
                            try:
                                asset_file.unlink()
                            except Exception:
                                pass
                    if "merged_nodes" in node:
                        for m_id in node["merged_nodes"]:
                            for asset_file in out_assets_dir.glob(f"asset_{m_id}.*"):
                                try:
                                    asset_file.unlink()
                                except Exception:
                                    pass
                    node["png_path"] = None
                    node["svg_path"] = None
                    node["raw_text_path"] = None
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
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
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
    into workspace/07_transform_tables/ without mutating prior stages.
    """
    tasks_dir = workspace_dir / "tasks" / "tables"
    if not tasks_dir.exists():
        print(f"[!] No table tasks directory found at {tasks_dir}")
        return 0

    input_candidates = [
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "07_transform_tables"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_assets_dir = out_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)

    # Synchronize upstream assets
    asset_candidates = [
        workspace_dir / "06_detect_continuations" / "assets",
        workspace_dir / "05_chapter_partition" / "assets",
        workspace_dir / "04_stream_reduction" / "assets",
        workspace_dir / "03_build_raw_stream" / "assets",
        workspace_dir / "assets",
    ]
    src_assets = next((p for p in asset_candidates if p.exists()), None)
    if src_assets:
        for f in src_assets.glob("*"):
            if f.is_file():
                shutil.copy2(f, out_assets_dir / f.name)

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
                text_table = rendered_by_node[n_id]
                node["rendered_markdown"] = text_table
                applied_count += 1
                if not re.search(r"!\[.*?\]\(.*?\)", text_table):
                    for asset_file in out_assets_dir.glob(f"asset_{n_id}.*"):
                        try:
                            asset_file.unlink()
                        except Exception:
                            pass
                    if "merged_nodes" in node:
                        for m_id in node["merged_nodes"]:
                            for asset_file in out_assets_dir.glob(f"asset_{m_id}.*"):
                                try:
                                    asset_file.unlink()
                                except Exception:
                                    pass
                    node["png_path"] = None
                    node["svg_path"] = None
                    node["raw_text_path"] = None

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
