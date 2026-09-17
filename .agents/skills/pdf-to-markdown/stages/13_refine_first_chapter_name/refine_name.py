#!/usr/bin/env python3
"""
stages/13_refine_first_chapter_name/refine_name.py:
Inspects the content and preliminary name of the first chapter in <output_dir>.
Queries Gemini to determine its canonical title and slug (typically Table of Contents / Front Matter).
Renames the file, updates its Line 1 YAML title, and synchronizes any cross-file wikilinks.
"""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import sys
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

from llm_client import GeminiClient


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


def process_first_chapter_refinement(
    output_dir: Path,
    workspace_dir: Path,
    config_path: Path,
    input_dir: Path = None,
):
    if not input_dir:
        candidates = [
            workspace_dir / "12_generate_properties",
            workspace_dir / "11_emit_markdown",
            output_dir
        ]
        input_dir = next((p for p in candidates if p.exists() and list(p.glob("*.md"))), output_dir)

    if not input_dir.exists():
        raise FileNotFoundError(f"Input directory does not exist: {input_dir}")

    md_files = sorted(list(input_dir.glob("*.md")))
    if not md_files:
        print("[!] No Markdown files found in input directory.")
        return

    first_file = md_files[0]
    old_stem = first_file.stem
    match_prefix = re.match(r"^(\d+)", old_stem)
    prefix = match_prefix.group(1) if match_prefix else "00"

    with open(first_file, "r", encoding="utf-8") as f:
        content = f.read()

    if not config_path or not config_path.is_file():
        raise FileNotFoundError(f"Stage 13: Config file not found: {config_path}")
    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 13: Config file is empty or invalid: {config_path}")

    gemini = GeminiClient(config)
    prompt_file = Path(__file__).parent / "prompt.md"
    base_prompt = prompt_file.read_text(encoding="utf-8") if prompt_file.exists() else ""

    full_prompt = (
        f"{base_prompt}\n\n"
        f"Preliminary File Name: {first_file.name}\n\n"
        f"Content Excerpt:\n```markdown\n{content[:4000]}\n```\n"
    )

    parsed = gemini.generate_json(full_prompt, stage="13_refine_chapter")
    if not isinstance(parsed, dict) or "title" not in parsed or "slug" not in parsed:
        raise ValueError(f"Gemini did not return valid title and slug JSON for {first_file.name}: {parsed}")

    new_title = str(parsed["title"]).strip().strip('"')
    new_slug = str(parsed["slug"]).strip().strip('"')

    clean_title_name = re.sub(r'[:/\\|]', ' - ', new_title)
    clean_title_name = re.sub(r'[*?"<>]', '', clean_title_name).strip(' -.')
    new_filename = f"{prefix} - {clean_title_name}.md" if clean_title_name else f"{prefix}_{new_slug}.md"

    output_dir.mkdir(parents=True, exist_ok=True)
    out_assets_dir = output_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)

    # If input and output differ, copy files to output without mutating input
    if input_dir.resolve() != output_dir.resolve():
        src_assets = input_dir / "assets"
        if src_assets.exists():
            for f in src_assets.glob("*"):
                if f.is_file():
                    shutil.copy2(f, out_assets_dir / f.name)
        if out_assets_dir.exists() and not any(out_assets_dir.iterdir()):
            try:
                out_assets_dir.rmdir()
            except Exception:
                pass
        for old_f in output_dir.glob("*.md"):
            old_f.unlink()
        for f in md_files:
            shutil.copy2(f, output_dir / f.name)

    # Target opening files (e.g. prefix 00: Front Matter, Table of Contents)
    all_out_md = sorted(list(output_dir.glob("*.md")))
    opening_targets = [f for f in all_out_md if re.match(r"^00\b", f.stem)]
    if not opening_targets and all_out_md:
        opening_targets = [all_out_md[0]]

    for target_file in opening_targets:
        old_stem = target_file.stem
        match_prefix = re.match(r"^(\d+)", old_stem)
        prefix = match_prefix.group(1) if match_prefix else "00"

        with open(target_file, "r", encoding="utf-8") as f:
            content = f.read()

        full_prompt = (
            f"{base_prompt}\n\n"
            f"Preliminary File Name: {target_file.name}\n\n"
            f"Content Excerpt:\n```markdown\n{content[:4000]}\n```\n"
        )

        parsed = gemini.generate_json(full_prompt, stage="13_refine_chapter")
        if not isinstance(parsed, dict) or "title" not in parsed or "slug" not in parsed:
            print(f"[!] Warning: Gemini did not return valid title and slug JSON for {target_file.name}: {parsed}")
            continue

        new_title = str(parsed["title"]).strip().strip('"')
        new_slug = str(parsed["slug"]).strip().strip('"')

        clean_title_name = re.sub(r'[:/\\|]', ' - ', new_title)
        clean_title_name = re.sub(r'[*?"<>]', '', clean_title_name)
        clean_title_name = re.sub(r'\s+', ' ', clean_title_name).strip(' -.')
        new_filename = f"{prefix} - {clean_title_name}.md" if clean_title_name else f"{prefix}_{new_slug}.md"
        new_path = output_dir / new_filename

        print(f"[*] Analyzing opening chapter: {target_file.name}")
        print(f"    Resolved canonical title: \"{new_title}\"")
        print(f"    Resolved canonical slug : \"{new_slug}\" -> {new_filename}")

        # Update YAML frontmatter
        updated_content = update_frontmatter_title(content, new_title)

        if new_path != target_file:
            with open(new_path, "w", encoding="utf-8") as f:
                f.write(updated_content)
            if target_file.exists():
                target_file.unlink()
            print(f"[+] Saved canonical file: {new_filename}")

            # Update cross-file wikilinks in output_dir
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
            with open(target_file, "w", encoding="utf-8") as f:
                f.write(updated_content)

    print(f"[+] Stage 13 complete. Output vault finalized in {output_dir}.")


def main():
    parser = argparse.ArgumentParser(description="Stage 13: Refine first chapter name and slug")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--input-dir", type=str, default=None, help="Input directory (defaults to workspace/12_generate_properties)")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory (defaults to workspace/13_refine_first_chapter_name)")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    output_dir = Path(args.output_dir) if args.output_dir else (workspace_dir / "13_refine_first_chapter_name")
    input_dir = Path(args.input_dir) if args.input_dir else None
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 13: Config file not found: {config_path}")

    process_first_chapter_refinement(
        output_dir=output_dir,
        workspace_dir=workspace_dir,
        config_path=config_path,
        input_dir=input_dir,
    )


if __name__ == "__main__":
    main()
