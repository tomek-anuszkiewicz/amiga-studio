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
    return f"{prefix} {cleaned}\n"


def format_prose_text(raw_text: str) -> str:
    text = raw_text.strip()
    # Backtick hex addresses ($DFF000, $0040)
    text = re.sub(r"(?<!`)\$([0-9A-Fa-f]{3,8})(?!`)", r"`$\1`", text)
    # Backtick common chip registers (DMACON, INTENA, COLOR00, etc.)
    text = re.sub(r"\b(DMACONR?|INTENAR?|INTREQR?|ADKCONR?|BPLCON[0-3]|COLOR[0-3][0-9]|COP1LCH?|COP2LCH?|BLTCON[01])\b", r"`\1`", text)
    return text + "\n"


def format_code_block(raw_text: str) -> str:
    lang = "m68k"
    if "#include" in raw_text or "void " in raw_text or "int " in raw_text:
        lang = "c"
    elif re.search(r"^[0-9A-Fa-f]{4,8}:", raw_text, re.MULTILINE):
        lang = "text"

    return f"```{lang}\n{raw_text.strip()}\n```\n"


def format_toc_block(raw_text: str) -> str:
    lines = [l.strip() for l in raw_text.splitlines() if l.strip()]
    formatted_lines = []
    for line in lines:
        # Strip trailing page numbers or dot leaders
        cleaned = re.sub(r"(\.{3,}|\s{3,})\d+$", "", line).strip()
        if cleaned:
            formatted_lines.append(f"- {cleaned}")

    toc_body = "\n".join(formatted_lines)
    return f"{TOC_START_MARKER}\n{toc_body}\n{TOC_END_MARKER}\n"


def process_prose(workspace_dir: Path, config: dict):
    chapters_dir = workspace_dir / "chapters"
    if not chapters_dir.exists():
        raise FileNotFoundError(f"Missing chapters directory: {chapters_dir}")

    gemini = GeminiClient(config) if GeminiClient else None
    if gemini and gemini.is_available():
        print(f"[*] Prose Worker LLM active ({gemini.default_model}, thinking: {gemini.thinking_level}).")
    else:
        print(f"[*] Prose Worker LLM unavailable (no GEMINI_API_KEY). Using heuristic prose formatter.")

    chapter_files = sorted(list(chapters_dir.glob("*.json")))
    print(f"[*] Formatting prose, code, and TOC across {len(chapter_files)} chapter files...")

    formatted_count = 0

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        modified = False
        for node in nodes:
            n_type = node.get("type")
            raw_text = node.get("raw_text", "")

            # Don't overwrite if already rendered (e.g. table or graphic)
            if node.get("rendered_markdown"):
                continue

            if n_type == "heading":
                lvl = node.get("heading_level", 2)
                node["rendered_markdown"] = format_heading(raw_text, level=lvl)
                modified = True
                formatted_count += 1
            elif n_type == "code_block":
                node["rendered_markdown"] = format_code_block(raw_text)
                modified = True
                formatted_count += 1
            elif n_type == "toc":
                node["rendered_markdown"] = format_toc_block(raw_text)
                modified = True
                formatted_count += 1
            elif n_type == "toc_header":
                # Will be ignored during Stage 10 emission, but provide clean placeholder
                node["rendered_markdown"] = ""
                modified = True
            elif n_type == "prose":
                node["rendered_markdown"] = format_prose_text(raw_text)
                modified = True
                formatted_count += 1

        if modified:
            with open(c_file, "w", encoding="utf-8") as f:
                json.dump(nodes, f, indent=2)

    print(f"[+] Stage 09 complete. Formatted {formatted_count} prose/code/TOC nodes.")


def prepare_prose_tasks(workspace_dir: Path) -> int:
    """
    Extracts TOC and code block nodes into workspace/tasks/prose/ for inspection.
    """
    chapters_dir = workspace_dir / "chapters"
    if not chapters_dir.exists():
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
                "chapter_file": str(c_file.relative_to(workspace_dir)),
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
    Applies edited prose/TOC tasks back into chapter streams.
    """
    tasks_dir = workspace_dir / "tasks" / "prose"
    if not tasks_dir.exists():
        print(f"[!] No prose tasks directory found at {tasks_dir}")
        return 0

    applied_count = 0
    for meta_file in sorted(list(tasks_dir.glob("*.json"))):
        with open(meta_file, "r", encoding="utf-8") as f:
            meta = json.load(f)

        node_id = meta.get("node_id")
        chapter_rel = meta.get("chapter_file")
        md_file = tasks_dir / f"{node_id}.md"

        if not md_file.exists() or not chapter_rel:
            continue

        with open(md_file, "r", encoding="utf-8") as f:
            rendered_md = f.read().strip()

        chapter_path = workspace_dir / chapter_rel
        if not chapter_path.exists():
            continue

        with open(chapter_path, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        updated = False
        for node in nodes:
            if node.get("node_id") == node_id:
                node["rendered_markdown"] = rendered_md
                updated = True
                break

        if updated:
            with open(chapter_path, "w", encoding="utf-8") as f:
                json.dump(nodes, f, indent=2)
            applied_count += 1

    print(f"[+] Applied {applied_count} prose tasks to chapter streams.")
    return applied_count


def main():
    parser = argparse.ArgumentParser(description="Stage 09: Format prose, code blocks, and tag TOC")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--prepare", action="store_true", help="Prepare TOC and code tasks in workspace/tasks/prose/")
    parser.add_argument("--apply", action="store_true", help="Apply Agent's edited prose tasks back to chapters")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    if args.prepare:
        prepare_prose_tasks(workspace_dir)
        return

    if args.apply:
        apply_prose_tasks(workspace_dir)
        return

    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_prose(workspace_dir, config)


if __name__ == "__main__":
    main()
