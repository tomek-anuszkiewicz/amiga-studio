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
    has_toc_links = "[[" in content and ("toc" in lower_content or "contents" in lower_content)
    has_toc_words = "table of contents" in lower_content or "contents" in lower_content

    # If this is the dedicated TOC partition or contains Table of Contents
    if "toc" in Path(current_name).stem.lower() or has_toc_links or has_toc_words:
        return "Table of Contents", "toc"

    # Check if Chapter 1 body is present in this opening file
    if re.search(r"^#+\s+chapter\s+1\b", content, re.MULTILINE | re.IGNORECASE):
        return "Chapter 1: Introduction", "chapter_1"

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


def process_first_chapter_refinement(
    output_dir: Path,
    workspace_dir: Path,
    input_dir: Path = None,
    config_path: Path = None,
    inspect_only: bool = False,
    override_title: str = None,
    override_slug: str = None
):
    import shutil

    if not input_dir:
        candidates = [
            workspace_dir / "11_markdown_linked",
            workspace_dir / "10_markdown_raw",
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
    match_prefix = re.match(r"^(\d+)_", old_stem)
    prefix = match_prefix.group(1) if match_prefix else "01"

    with open(first_file, "r", encoding="utf-8") as f:
        content = f.read()

    if inspect_only:
        print(f"[*] Opening Chapter Inspection: {first_file.name}")
        print("=" * 60)
        lines = content.splitlines()[:50]
        print("\n".join(lines))
        print("=" * 60)
        auto_title, auto_slug = determine_canonical_title_and_slug(content, first_file.name)
        print(f"Suggested Canonical Title: {auto_title}")
        print(f"Suggested Slug           : {auto_slug}")
        return

    new_title = override_title
    new_slug = override_slug

    if not new_title or not new_slug:
        config = {}
        if config_path and config_path.exists():
            try:
                with open(config_path, "r", encoding="utf-8") as f:
                    config = yaml.safe_load(f) or {}
            except Exception:
                config = {}

        gemini = GeminiClient(config) if GeminiClient else None
        if not gemini or not gemini.is_available():
            raise RuntimeError("GEMINI_API_KEY environment variable is required for Stage 12 name refinement.")

        prompt_file = Path(__file__).parent / "prompt.md"
        base_prompt = prompt_file.read_text(encoding="utf-8") if prompt_file.exists() else ""

        full_prompt = (
            f"{base_prompt}\n\n"
            f"Preliminary File Name: {first_file.name}\n\n"
            f"Content Excerpt:\n```markdown\n{content[:4000]}\n```\n"
        )
        parsed = gemini.generate_json(full_prompt)
        if isinstance(parsed, dict):
            new_title = parsed.get("title")
            new_slug = parsed.get("slug")

    if not new_title or not new_slug:
        clean_stem = re.sub(r"^\d+_", "", Path(first_file.name).stem)
        new_title = clean_stem.replace("_", " ").title()
        new_slug = clean_stem

    new_filename = f"{prefix}_{new_slug}.md"

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
        for old_f in output_dir.glob("*.md"):
            old_f.unlink()
        for f in md_files:
            shutil.copy2(f, output_dir / f.name)

    target_first_file = output_dir / first_file.name
    new_path = output_dir / new_filename

    print(f"[*] Analyzing opening chapter: {first_file.name}")
    print(f"    Resolved canonical title: \"{new_title}\"")
    print(f"    Resolved canonical slug : \"{new_slug}\" -> {new_filename}")

    # Update YAML frontmatter
    updated_content = update_frontmatter_title(content, new_title)

    if new_path != target_first_file:
        with open(new_path, "w", encoding="utf-8") as f:
            f.write(updated_content)
        if target_first_file.exists():
            target_first_file.unlink()
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
        with open(target_first_file, "w", encoding="utf-8") as f:
            f.write(updated_content)

    print(f"[+] Stage 12 complete. Output vault finalized in {output_dir}.")


def main():
    parser = argparse.ArgumentParser(description="Stage 12: Refine first chapter name and slug")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--input-dir", type=str, default=None, help="Input directory (defaults to workspace/11_markdown_linked)")
    parser.add_argument("--output-dir", type=str, default="output_markdown", help="Output directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--inspect", action="store_true", help="Inspect opening chapter excerpt and suggested titles")
    parser.add_argument("--title", type=str, default=None, help="Explicit canonical title")
    parser.add_argument("--slug", type=str, default=None, help="Explicit canonical slug")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    output_dir = Path(args.output_dir)
    input_dir = Path(args.input_dir) if args.input_dir else None

    process_first_chapter_refinement(
        output_dir,
        workspace_dir,
        input_dir=input_dir,
        config_path=Path(args.config),
        inspect_only=args.inspect,
        override_title=args.title,
        override_slug=args.slug
    )


if __name__ == "__main__":
    main()
