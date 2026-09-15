#!/usr/bin/env python3
"""
stages/12_refine_first_chapter_name/refine_name.py:
Inspects the content and preliminary name of the first chapter in <output_dir>.
Determines its canonical title and slug (typically Table of Contents / Front Matter).
Renames the file, updates its Line 1 YAML title, and synchronizes any cross-file wikilinks.
"""

import argparse
import json
import os
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


def determine_canonical_title_and_slug(content: str, current_name: str) -> tuple:
    """
    Analyzes opening section content to determine canonical title and slug.
    """
    lower_content = content.lower()

    # Check for Table of Contents indicators
    has_toc_links = bool(re.search(r"-\s*\[\[.+\]\]", content))
    has_toc_words = "table of contents" in lower_content or "contents" in lower_content

    if has_toc_links or has_toc_words:
        if "preface" in lower_content or "foreword" in lower_content:
            return "Preface and Table of Contents", "preface_and_contents"
        return "Table of Contents", "table_of_contents"

    # Check for Preface / Foreword
    if "preface" in lower_content:
        return "Preface", "preface"
    if "foreword" in lower_content:
        return "Foreword", "foreword"
    if "introduction" in lower_content:
        return "Introduction", "introduction"

    # Fallback to current stem
    clean_stem = re.sub(r"^\d+_", "", Path(current_name).stem)
    title = clean_stem.replace("_", " ").title()
    return title, clean_stem


def update_frontmatter_title(content: str, new_title: str) -> str:
    """
    Updates the title field in the Line 1 YAML frontmatter.
    """
    if content.startswith("---"):
        parts = content.split("---", 2)
        if len(parts) >= 3:
            fm_text = parts[1]
            updated_fm = re.sub(r'title:\s*".*?"', f'title: "{new_title}"', fm_text)
            if updated_fm == fm_text:
                updated_fm = re.sub(r"title:\s*.*?\n", f'title: "{new_title}"\n', fm_text)
            return f"---{updated_fm}---{parts[2]}"
    return content


def process_first_chapter_refinement(output_dir: Path, workspace_dir: Path, config_path: Path = None):
    if not output_dir.exists():
        raise FileNotFoundError(f"Output directory does not exist: {output_dir}")

    md_files = sorted(list(output_dir.glob("*.md")))
    if not md_files:
        print("[!] No Markdown files found in output directory.")
        return

    first_file = md_files[0]
    old_stem = first_file.stem
    match_prefix = re.match(r"^(\d+)_", old_stem)
    prefix = match_prefix.group(1) if match_prefix else "01"

    with open(first_file, "r", encoding="utf-8") as f:
        content = f.read()

    config = {}
    if config_path and config_path.exists():
        try:
            with open(config_path, "r", encoding="utf-8") as f:
                config = yaml.safe_load(f) or {}
        except Exception:
            config = {}

    gemini = GeminiClient(config) if GeminiClient else None
    new_title, new_slug = None, None

    if gemini and gemini.is_available():
        prompt_file = Path(__file__).parent / "prompt.md"
        base_prompt = ""
        if prompt_file.exists():
            with open(prompt_file, "r", encoding="utf-8") as pf:
                base_prompt = pf.read()

        full_prompt = (
            f"{base_prompt}\n\n"
            f"Preliminary File Name: {first_file.name}\n\n"
            f"Content Excerpt:\n```markdown\n{content[:4000]}\n```\n"
        )
        print(f"[*] Calling Gemini ({gemini.default_model}, thinking: {gemini.thinking_level}) for canonical title & slug...")
        raw_res = gemini.generate_text(full_prompt)
        if raw_res:
            try:
                json_match = re.search(r"\{.*\}", raw_res, re.DOTALL)
                if json_match:
                    parsed = json.loads(json_match.group(0))
                    new_title = parsed.get("title")
                    new_slug = parsed.get("slug")
            except Exception as e:
                print(f"[!] Warning: Failed to parse LLM response as JSON: {e}")

    if not new_title or not new_slug:
        new_title, new_slug = determine_canonical_title_and_slug(content, first_file.name)

    new_filename = f"{prefix}_{new_slug}.md"
    new_path = output_dir / new_filename

    print(f"[*] Analyzing opening chapter: {first_file.name}")
    print(f"    Resolved canonical title: \"{new_title}\"")
    print(f"    Resolved canonical slug : \"{new_slug}\" -> {new_filename}")

    # Update YAML frontmatter
    updated_content = update_frontmatter_title(content, new_title)

    if new_path != first_file:
        # Write new file and remove old file
        with open(new_path, "w", encoding="utf-8") as f:
            f.write(updated_content)
        first_file.unlink()
        print(f"[+] Renamed {first_file.name} -> {new_filename}")

        # Update any cross-file wikilinks referencing the old stem
        new_stem = new_path.stem
        for other_file in output_dir.glob("*.md"):
            with open(other_file, "r", encoding="utf-8") as f:
                txt = f.read()
            if f"[[{old_stem}" in txt:
                updated_txt = txt.replace(f"[[{old_stem}", f"[[{new_stem}")
                with open(other_file, "w", encoding="utf-8") as f:
                    f.write(updated_txt)
                print(f"    Updated wikilinks in {other_file.name} to point to {new_stem}")
    else:
        with open(first_file, "w", encoding="utf-8") as f:
            f.write(updated_content)

    print(f"[+] Stage 12 complete. First chapter naming refined.")


def main():
    parser = argparse.ArgumentParser(description="Stage 12: Refine first chapter name and slug")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--output-dir", type=str, default="output_markdown", help="Output directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    output_dir = Path(args.output_dir)

    process_first_chapter_refinement(output_dir, workspace_dir, config_path=Path(args.config))


if __name__ == "__main__":
    main()
