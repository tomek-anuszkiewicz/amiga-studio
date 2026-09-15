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


def main():
    parser = argparse.ArgumentParser(description="Stage 09: Format prose, code blocks, and tag TOC")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_prose(workspace_dir, config)


if __name__ == "__main__":
    main()
