#!/usr/bin/env python3
"""
stages/09_transform_prose/format_prose.py:
Worker for prose, code_block, heading, and toc nodes:
1. Formats prose with backticked hex addresses ($DFF000) and KaTeX.
2. Formats code_block with explicit language tags (m68k, c, text).
3. Wraps toc nodes in explicit delimiters:
   <!-- TOC34534 -->
   ...
   <!-- /TOC34534 -->
4. Preserves explicit node types (prose, code_block, toc) in the JSON stream.
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

TOC_START_MARKER = "<!-- TOC34534 -->"
TOC_END_MARKER = "<!-- /TOC34534 -->"


def format_heading(raw_text: str, level: int = 2) -> str:
    cleaned = re.sub(r"\s+", " ", raw_text).strip()
    prefix = "#" * max(1, min(6, level or 2))
    return f"{prefix} {cleaned}\n\n"


def process_prose(workspace_dir: Path, config: dict):
    input_candidates = [
        workspace_dir / "08_transform_graphics",
        workspace_dir / "07_transform_tables",
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "09_transform_prose"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in out_dir.glob("*.json"):
        f.unlink()

    prompt_path = Path(__file__).resolve().parent / "prompt.md"
    base_prompt = prompt_path.read_text(encoding="utf-8") if prompt_path.exists() else ""

    gemini = GeminiClient(config)

    from concurrent.futures import ThreadPoolExecutor

    concurrency = int(config.get("llm", {}).get("concurrency", 8))
    print(f"[*] Prose Worker LLM active ({gemini.default_model}). Formatting prose/code (concurrency={concurrency})...")

    chapter_files = sorted(list(input_dir.glob("*.json")))
    formatted_count = 0

    def _format_prose_task(task):
        c_file, group_indices, n_type, raw_text, png_path = task
        full_prompt = (
            f"{base_prompt}\n\n"
            f"## Node Type: {n_type}\n"
            f"## Raw Text:\n```text\n{raw_text}\n```\n"
        )
        if n_type == "code_block" and png_path and png_path.exists():
            rendered = gemini.generate_vision(full_prompt, image_path=png_path, stage="09_transform_prose")
        else:
            rendered = gemini.generate_text(full_prompt, stage="09_transform_prose")

        if rendered:
            rendered_text = rendered.strip()
            if n_type == "toc" and TOC_START_MARKER not in rendered_text:
                rendered_text = f"{TOC_START_MARKER}\n{rendered_text}\n{TOC_END_MARKER}"
            return c_file, group_indices, rendered_text + "\n\n"
        else:
            return c_file, group_indices, raw_text + "\n\n"

    chapters = {}
    all_llm_tasks = []

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)
        chapters[c_file] = nodes

        i = 0
        while i < len(nodes):
            node = nodes[i]
            # Don't overwrite if already rendered (e.g. table or graphic)
            if node.get("rendered_markdown"):
                i += 1
                continue

            n_type = node.get("type")
            raw_text = node.get("raw_text", "")

            if n_type in ("heading", "chapter"):
                lvl = 1 if n_type == "chapter" else (node.get("heading_level") or 2)
                node["rendered_markdown"] = format_heading(raw_text, level=lvl)
                formatted_count += 1
                i += 1
            elif n_type == "toc_header":
                node["rendered_markdown"] = ""
                i += 1
            elif n_type == "caption":
                clean_cap = " ".join(raw_text.strip().split())
                node["rendered_markdown"] = f"*{clean_cap}*\n\n"
                formatted_count += 1
                i += 1
            elif n_type == "toc":
                # Batch consecutive TOC nodes on the same page (up to 6000 chars)
                group_indices = [i]
                group_text_parts = [raw_text]
                group_len = len(raw_text)
                cur_page = node.get("page")
                j = i + 1
                while j < len(nodes):
                    next_node = nodes[j]
                    if (
                        next_node.get("type") == "toc"
                        and not next_node.get("rendered_markdown")
                        and next_node.get("page") == cur_page
                        and (group_len + len(next_node.get("raw_text", ""))) < 6000
                    ):
                        group_indices.append(j)
                        t = next_node.get("raw_text", "")
                        group_text_parts.append(t)
                        group_len += len(t)
                        j += 1
                    else:
                        break
                combined_raw = "\n\n".join(group_text_parts)
                all_llm_tasks.append((c_file, group_indices, "toc", combined_raw, None))
                i = j
            elif n_type == "code_block":
                png_rel = node.get("png_path")
                png_path = workspace_dir / png_rel if png_rel else None
                all_llm_tasks.append((c_file, [i], "code_block", raw_text, png_path))
                i += 1
            elif n_type == "prose":
                # Batch consecutive prose nodes on the same page up to 4000 chars if no other types intervene
                group_indices = [i]
                group_text_parts = [raw_text]
                group_len = len(raw_text)
                cur_page = node.get("page")
                j = i + 1
                while j < len(nodes):
                    next_node = nodes[j]
                    if (
                        next_node.get("type") == "prose"
                        and not next_node.get("rendered_markdown")
                        and next_node.get("page") == cur_page
                        and (group_len + len(next_node.get("raw_text", ""))) < 4000
                    ):
                        group_indices.append(j)
                        t = next_node.get("raw_text", "")
                        group_text_parts.append(t)
                        group_len += len(t)
                        j += 1
                    else:
                        break
                combined_raw = "\n\n".join(group_text_parts)
                all_llm_tasks.append((c_file, group_indices, "prose", combined_raw, None))
                i = j
            else:
                i += 1

    if all_llm_tasks:
        print(f"[*] Formatting {len(all_llm_tasks)} prose/code/toc batches across {len(chapters)} chapters (concurrency={concurrency})...")
        with ThreadPoolExecutor(max_workers=min(len(all_llm_tasks), concurrency)) as executor:
            results = list(executor.map(_format_prose_task, all_llm_tasks))

        for c_file, group_indices, rendered_md in results:
            nodes = chapters[c_file]
            head_idx = group_indices[0]
            nodes[head_idx]["rendered_markdown"] = rendered_md
            formatted_count += 1
            for sub_idx in group_indices[1:]:
                nodes[sub_idx]["rendered_markdown"] = ""
                nodes[sub_idx]["continuation_status"] = "continuation"
                formatted_count += 1

    for c_file, nodes in chapters.items():
        target_file = out_dir / c_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

    print(f"[+] Stage 09 complete. Formatted {formatted_count} nodes into {out_dir}.")


def prepare_prose_tasks(workspace_dir: Path) -> int:
    """
    Extracts TOC and code block nodes into workspace/tasks/prose/ for inspection.
    """
    input_candidates = [
        workspace_dir / "08_transform_graphics",
        workspace_dir / "07_transform_tables",
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    chapters_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not chapters_dir:
        raise FileNotFoundError(f"Missing chapters directory: {chapters_dir}")

    tasks_dir = workspace_dir / "tasks" / "prose"
    tasks_dir.mkdir(parents=True, exist_ok=True)

    count = 0
    for c_file in sorted(list(chapters_dir.glob("*.json"))):
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        for node in nodes:
            n_type = node.get("type")
            if n_type not in ("toc", "code_block"):
                continue

            node_id = node.get("node_id")
            raw_text = node.get("raw_text", "")
            draft_md = format_toc_block(raw_text) if n_type == "toc" else format_code_block(raw_text)

            task_meta = {
                "node_id": node_id,
                "chapter_file": c_file.name,
                "type": n_type,
                "page": node.get("page"),
                "raw_text": raw_text,
            }

            with open(tasks_dir / f"{node_id}.json", "w", encoding="utf-8") as f:
                json.dump(task_meta, f, indent=2)

            md_file = tasks_dir / f"{node_id}.md"
            if not md_file.exists():
                with open(md_file, "w", encoding="utf-8") as f:
                    f.write(draft_md)

            count += 1

    print(f"[+] Prepared {count} special prose/TOC tasks in {tasks_dir}")
    return count


def apply_prose_tasks(workspace_dir: Path) -> int:
    """
    Applies edited prose/TOC tasks into workspace/09_transform_prose/.
    """
    tasks_dir = workspace_dir / "tasks" / "prose"
    if not tasks_dir.exists():
        print(f"[!] No prose tasks directory found at {tasks_dir}")
        return 0

    input_candidates = [
        workspace_dir / "08_transform_graphics",
        workspace_dir / "07_transform_tables",
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "09_transform_prose"
    out_dir.mkdir(parents=True, exist_ok=True)

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

    print(f"[+] Applied {applied_count} prose tasks to {out_dir}.")
    return applied_count


def main():
    parser = argparse.ArgumentParser(description="Stage 09: Format prose, code blocks, and tag TOC")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")
    parser.add_argument("--prepare", action="store_true", help="Prepare TOC and code tasks in workspace/tasks/prose/")
    parser.add_argument("--apply", action="store_true", help="Apply Agent's edited prose tasks back to chapters")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 09: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 09: Config file is empty or invalid: {config_path}")

    if args.prepare:
        prepare_prose_tasks(workspace_dir)
        return

    if args.apply:
        apply_prose_tasks(workspace_dir)
        return

    process_prose(workspace_dir, config)


if __name__ == "__main__":
    main()
