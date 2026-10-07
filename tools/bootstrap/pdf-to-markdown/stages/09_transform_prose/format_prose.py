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

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

# Import CodexClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

from conversion import CodexClient

TOC_START_MARKER = "<!-- TOC34534 -->"
TOC_END_MARKER = "<!-- /TOC34534 -->"
ADVISORY_WORDS = {"NOTE", "WARNING", "CAUTION", "IMPORTANT", "TIP", "INFO"}


def format_heading(raw_text: str, level: int = 2) -> str:
    cleaned = re.sub(r"\s+", " ", raw_text).strip()
    prefix = "#" * max(1, min(6, level or 2))
    return f"{prefix} {cleaned}\n\n"


def assemble_callouts(nodes: list) -> int:
    """
    Assembles callout headers and callout_text blocks into native Obsidian callouts:
    > [!KIND]
    > Body text line 1
    > Body text line 2
    Also suppresses standalone advisory headings if followed by callout_text.
    """
    assembled_count = 0
    k = 0
    while k < len(nodes):
        node = nodes[k]
        n_type = node.get("type")
        raw_title = node.get("raw_text", "").strip()
        clean_tag = re.sub(r"[^\w]", "", raw_title).upper()

        is_callout_header = (n_type == "callout") or (n_type == "heading" and clean_tag in ADVISORY_WORDS)

        if is_callout_header:
            tag = clean_tag if clean_tag in ADVISORY_WORDS else "NOTE"

            body_parts = []
            consumed_indices = []
            m = k + 1
            while m < len(nodes):
                next_n = nodes[m]
                next_type = next_n.get("type")
                if next_type == "callout_text":
                    txt = next_n.get("rendered_markdown", "").strip() or next_n.get("raw_text", "").strip()
                    if txt:
                        body_parts.append(txt)
                    consumed_indices.append(m)
                    m += 1
                elif next_n.get("continuation_status") == "continuation" and consumed_indices:
                    m += 1
                else:
                    break

            if body_parts:
                combined_body = "\n\n".join(body_parts).strip()
                # Strip redundant leading "NOTE:" / "WARNING:" from the body if present
                combined_body = re.sub(rf"^(?:{tag})\s*[:.-]?\s*", "", combined_body, flags=re.IGNORECASE).strip()

                callout_lines = [f"> [!{tag}]"]
                for line in combined_body.splitlines():
                    if line.strip():
                        callout_lines.append(f"> {line}")
                    else:
                        callout_lines.append(">")

                node["type"] = "callout"
                node["rendered_markdown"] = "\n".join(callout_lines) + "\n\n"
                for c_idx in consumed_indices:
                    nodes[c_idx]["rendered_markdown"] = ""
                    nodes[c_idx]["continuation_status"] = "continuation"
                assembled_count += 1
                k = m
                continue
            else:
                if n_type == "callout":
                    node["rendered_markdown"] = ""
                k += 1
                continue

        elif n_type == "callout_text" and not (node.get("rendered_markdown") or "").startswith("> [!"):
            # Standalone callout_text not preceded by a callout header
            txt = node.get("rendered_markdown", "").strip() or node.get("raw_text", "").strip()
            if txt:
                clean_tag = "NOTE"
                for word in ADVISORY_WORDS:
                    if re.match(rf"^{word}\s*[:.-]", txt, flags=re.IGNORECASE):
                        clean_tag = word
                        txt = re.sub(rf"^{word}\s*[:.-]?\s*", "", txt, flags=re.IGNORECASE).strip()
                        break
                callout_lines = [f"> [!{clean_tag}]"]
                for line in txt.splitlines():
                    if line.strip():
                        callout_lines.append(f"> {line}")
                    else:
                        callout_lines.append(">")
                node["rendered_markdown"] = "\n".join(callout_lines) + "\n\n"
                assembled_count += 1
            k += 1
        else:
            k += 1

    return assembled_count


def process_prose(workspace_dir: Path, config: dict):
    input_dir = workspace_dir / "08_transform_graphics"

    out_dir = workspace_dir / "09_transform_prose"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in (path for path in out_dir.iterdir() if path.suffix == ".json"):
        f.unlink()

    prompt_path = Path(__file__).resolve().parent / "prompt.md"
    base_prompt = prompt_path.read_text(encoding="utf-8")

    with CodexClient(config, stage="09_transform_prose", images=True) as codex:

        from concurrent.futures import ThreadPoolExecutor

        concurrency = int(config.get("llm", {}).get("concurrency", 1))
        print(f"[*] Prose Worker LLM active ({codex.selected.model}). Formatting prose/code (concurrency={concurrency})...")

        chapter_files = sorted(list((path for path in input_dir.iterdir() if path.suffix == ".json")))
        formatted_count = 0

        def _format_prose_task(task):
            c_file, group_indices, n_type, raw_text, png_path = task
            full_prompt = (
                f"{base_prompt}\n\n"
                f"## Node Type: {n_type}\n"
                f"## Raw Text:\n```text\n{raw_text}\n```\n"
            )
            if n_type == "code_block" and png_path:
                rendered = codex.generate_vision(full_prompt, image_path=png_path)
            else:
                rendered = codex.generate_text(full_prompt)

            if rendered:
                rendered_text = rendered.strip()
                if n_type == "toc" and TOC_START_MARKER not in rendered_text:
                    rendered_text = f"{TOC_START_MARKER}\n{rendered_text}\n{TOC_END_MARKER}"
                return c_file, group_indices, rendered_text + "\n\n"
            else:
                return c_file, group_indices, raw_text + "\n\n"

        chapters = {}
        metadata = {}
        all_llm_tasks = []

        for c_file in chapter_files:
            with open(c_file, "r", encoding="utf-8") as f:
                chapter = json.load(f)
                nodes = chapter["nodes"]
            chapters[c_file] = nodes
            metadata[c_file] = chapter

            i = 0
            while i < len(nodes):
                node = nodes[i]
                # Don't overwrite if already rendered (e.g. table or graphic)
                if node.get("rendered_markdown"):
                    i += 1
                    continue

                n_type = node.get("type")
                raw_text = node.get("raw_text", "")

                if n_type == "callout":
                    # Will be assembled with subsequent callout_text blocks in assemble_callouts
                    i += 1
                elif n_type in ("heading", "chapter"):
                    lvl = 1 if n_type == "chapter" else (node.get("heading_level") or 2)
                    node["rendered_markdown"] = format_heading(raw_text, level=lvl)
                    formatted_count += 1
                    i += 1
                elif n_type == "toc_heading":
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
                elif n_type in ("prose", "callout_text"):
                    # Batch consecutive nodes of identical type on the same page up to 4000 chars
                    cur_type = n_type
                    group_indices = [i]
                    group_text_parts = [raw_text]
                    group_len = len(raw_text)
                    cur_page = node.get("page")
                    j = i + 1
                    while j < len(nodes):
                        next_node = nodes[j]
                        if (
                            next_node.get("type") == cur_type
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
            assembled = assemble_callouts(nodes)
            if assembled > 0:
                formatted_count += assembled
            target_file = out_dir / c_file.name
            with open(target_file, "w", encoding="utf-8") as f:
                json.dump({**metadata[c_file], "nodes": nodes}, f, indent=2)

        print(f"[+] Stage 09 complete. Formatted {formatted_count} nodes into {out_dir}.")


def main():
    parser = argparse.ArgumentParser(description="Stage 09: Format prose, code blocks, and tag TOC")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)


    process_prose(workspace_dir, config)


if __name__ == "__main__":
    main()
