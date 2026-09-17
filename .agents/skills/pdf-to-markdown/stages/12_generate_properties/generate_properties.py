#!/usr/bin/env python3
"""
stages/12_generate_properties/generate_properties.py:
Generates publication-grade Obsidian YAML frontmatter properties using Gemini LLM:
1. Discovers emitted Markdown chapter files from Stage 11.
2. Identifies the opening section (front matter / Table of Contents / title page) to infer canonical book title.
3. For each chapter, prompts LLM with chapter markdown and first chapter context to generate:
   - title: Canonical chapter or section title.
   - book: Canonical book or reference manual title.
   - chapter: Normalized chapter designator (e.g. "Chapter 1", "Section 3", "Table of Contents").
   - tags: 4-8 relevant domain and topic tags (lowercase kebab-case).
4. Injects Line 1 YAML properties into each file and writes to <output_dir>.
5. Synchronizes visual assets from input to output directory.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
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

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def sanitize_tag(raw_tag: str) -> str:
    """Normalizes a tag into clean lowercase kebab-case."""
    cleaned = raw_tag.strip().lower()
    cleaned = re.sub(r"^#+", "", cleaned)
    cleaned = re.sub(r"[^a-z0-9]+", "-", cleaned).strip("-")
    return cleaned


def fallback_infer_properties(
    file_path: Path,
    content: str,
    first_file_content: str,
    fallback_book: str = "Amiga Reference Manual"
) -> dict:
    """Heuristic fallback when LLM is unavailable."""
    stem = file_path.stem
    clean_stem = re.sub(r"^\d+[_\s-]+", "", stem).replace("_", " ").strip()

    # Detect title from first markdown header
    title_match = re.search(r"^#+\s+(.+)$", content, re.MULTILINE)
    if title_match:
        raw_title = title_match.group(1).strip()
        title = re.sub(r"[*_`]", "", raw_title)
    else:
        title = clean_stem.title() if clean_stem else "Document"

    # Detect chapter designator
    chapter_match = re.search(
        r"\b(Section\s+\d+|Chapter\s+\d+|Appendix\s+[A-Z]|Table of Contents|Front Matter|Preface|Index|Glossary)\b",
        title,
        re.IGNORECASE
    )
    if chapter_match:
        chapter = chapter_match.group(1).title()
    elif "toc" in stem.lower() or "contents" in stem.lower():
        chapter = "Table of Contents"
    else:
        num_m = re.match(r"^(\d+)", stem)
        chapter = f"Section {int(num_m.group(1))}" if num_m else "Section"

    tags = ["amiga", "reference", "hardware"]
    lower_content = content.lower()
    if "68000" in lower_content or "m68k" in lower_content:
        tags.extend(["m68000", "motorola"])
    if "blitter" in lower_content:
        tags.append("blitter")
    if "copper" in lower_content:
        tags.append("copper")
    if "denise" in lower_content or "playfield" in lower_content or "sprite" in lower_content:
        tags.append("denise")
    if "paula" in lower_content or "audio" in lower_content:
        tags.append("paula")

    # Format tags
    seen = set()
    dedup_tags = []
    for t in tags:
        st = sanitize_tag(t)
        if st and st not in seen:
            seen.add(st)
            dedup_tags.append(st)

    return {
        "title": title,
        "book": fallback_book,
        "chapter": chapter,
        "tags": dedup_tags[:8]
    }


def generate_chapter_properties(
    file_path: Path,
    content: str,
    first_file_excerpt: str,
    gemini: GeminiClient,
    base_prompt: str,
    fallback_book: str = "Amiga Reference Manual"
) -> dict:
    """Generates Obsidian properties for a single chapter using Gemini."""
    if not gemini or not gemini.is_available():
        return fallback_infer_properties(file_path, content, first_file_excerpt, fallback_book)

    # Extract substantial excerpt of chapter content (up to 4000 characters)
    chapter_excerpt = content[:4000]

    full_prompt = (
        f"{base_prompt}\n\n"
        f"## Opening Book / Front Matter Context (for inferring 'book' title):\n"
        f"```markdown\n{first_file_excerpt[:3500]}\n```\n\n"
        f"## Current Chapter File Name: `{file_path.name}`\n\n"
        f"## Current Chapter Content Excerpt:\n"
        f"```markdown\n{chapter_excerpt}\n```\n"
    )

    try:
        data = gemini.generate_json(full_prompt, stage="12_generate_properties")
        if isinstance(data, dict):
            title = str(data.get("title", "")).strip().strip('"')
            book = str(data.get("book", "")).strip().strip('"')
            chapter = str(data.get("chapter", "")).strip().strip('"')
            raw_tags = data.get("tags", [])

            clean_tags = []
            if isinstance(raw_tags, list):
                for t in raw_tags:
                    st = sanitize_tag(str(t))
                    if st and st not in clean_tags:
                        clean_tags.append(st)

            if not clean_tags:
                clean_tags = ["amiga", "reference", "hardware"]

            if title and book and chapter:
                return {
                    "title": title,
                    "book": book,
                    "chapter": chapter,
                    "tags": clean_tags[:8]
                }
    except Exception as e:
        print(f"[!] Warning: LLM properties generation failed for {file_path.name}: {e}")

    return fallback_infer_properties(file_path, content, first_file_excerpt, fallback_book)


def serialize_obsidian_frontmatter(props: dict) -> str:
    """Serializes properties dict into canonical Obsidian YAML frontmatter."""
    lines = ["---"]
    title_escaped = props.get("title", "").replace('"', '\\"')
    book_escaped = props.get("book", "").replace('"', '\\"')
    chapter_escaped = props.get("chapter", "").replace('"', '\\"')

    lines.append(f'title: "{title_escaped}"')
    lines.append(f'book: "{book_escaped}"')
    lines.append(f'chapter: "{chapter_escaped}"')
    lines.append("tags:")
    for tag in props.get("tags", []):
        lines.append(f"  - {tag}")
    lines.append("---")
    lines.append("")
    return "\n".join(lines)


def process_generate_properties(
    workspace_dir: Path,
    input_dir: Path,
    output_dir: Path,
    config: dict,
    skip_llm: bool = False
):
    output_dir.mkdir(parents=True, exist_ok=True)
    out_assets = output_dir / "assets"
    out_assets.mkdir(parents=True, exist_ok=True)

    # Clean old markdown files in output_dir
    for old_md in output_dir.glob("*.md"):
        try:
            old_md.unlink()
        except Exception:
            pass

    # Synchronize assets from input directory
    in_assets = input_dir / "assets"
    if in_assets.exists():
        for asset in in_assets.glob("*"):
            if asset.is_file():
                try:
                    shutil.copy2(asset, out_assets / asset.name)
                except Exception:
                    pass
    if out_assets.exists() and not any(out_assets.iterdir()):
        try:
            out_assets.rmdir()
        except Exception:
            pass

    md_files = sorted(list(input_dir.glob("*.md")))
    if not md_files:
        raise FileNotFoundError(f"No Markdown files found in {input_dir}")

    # Read first chapter for global book inference
    first_file = md_files[0]
    with open(first_file, "r", encoding="utf-8") as f:
        first_content = f.read()

    # Determine fallback book name from directory structure or first file
    fallback_book = workspace_dir.parent.name if workspace_dir.parent else "Amiga Reference Manual"
    if fallback_book in [".", "workspace", ""]:
        fallback_book = "Amiga Technical Reference"

    # Initialize Gemini client
    gemini = None
    if not skip_llm and GeminiClient:
        try:
            gemini = GeminiClient(config)
        except Exception as e:
            print(f"[!] Warning: Unable to initialize GeminiClient ({e}). Using heuristic fallback.")

    prompt_path = Path(__file__).parent / "prompt.md"
    base_prompt = prompt_path.read_text(encoding="utf-8") if prompt_path.exists() else ""

    print(f"[*] Generating Obsidian properties for {len(md_files)} file(s) from {input_dir}...")
    print(f"    First chapter reference: {first_file.name}")

    def process_file(md_path: Path):
        with open(md_path, "r", encoding="utf-8") as f:
            content = f.read()

        # Strip any existing frontmatter before regenerating
        cleaned_content = content
        if cleaned_content.startswith("---"):
            parts = cleaned_content.split("---", 2)
            if len(parts) >= 3:
                cleaned_content = parts[2].lstrip("\r\n")

        props = generate_chapter_properties(
            file_path=md_path,
            content=cleaned_content,
            first_file_excerpt=first_content,
            gemini=gemini,
            base_prompt=base_prompt,
            fallback_book=fallback_book
        )

        frontmatter_block = serialize_obsidian_frontmatter(props)
        final_text = f"{frontmatter_block}\n{cleaned_content}".rstrip() + "\n"

        target_path = output_dir / md_path.name
        with open(target_path, "w", encoding="utf-8") as f:
            f.write(final_text)

        print(f"    [+] {md_path.name} -> title: \"{props['title']}\", chapter: \"{props['chapter']}\"")
        return props

    concurrency = config.get("llm", {}).get("concurrency", 4) if gemini and gemini.is_available() else 1
    if concurrency > 1 and len(md_files) > 1:
        with ThreadPoolExecutor(max_workers=concurrency) as executor:
            list(executor.map(process_file, md_files))
    else:
        for md_path in md_files:
            process_file(md_path)

    print(f"[+] Stage 12 complete. Processed {len(md_files)} file(s) with Obsidian properties into {output_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 12: Generate Obsidian Properties")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--input-dir", type=str, default=None, help="Input directory (defaults to workspace/11_emit_markdown)")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory (defaults to workspace/12_generate_properties)")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")
    parser.add_argument("--skip-llm", action="store_true", help="Skip LLM invocation and use heuristic fallback")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 12: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 12: Config file is empty or invalid: {config_path}")

    input_candidates = [
        Path(args.input_dir) if args.input_dir else None,
        workspace_dir / "11_emit_markdown",
        workspace_dir / "10_proofread_stream",
        Path("output_markdown")
    ]
    input_dir = next((p for p in input_candidates if p and p.exists() and list(p.glob("*.md"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Stage 12: No input markdown files found in candidates under {workspace_dir}")

    output_dir = Path(args.output_dir) if args.output_dir else (workspace_dir / "12_generate_properties")

    process_generate_properties(
        workspace_dir=workspace_dir,
        input_dir=input_dir,
        output_dir=output_dir,
        config=config,
        skip_llm=args.skip_llm
    )


if __name__ == "__main__":
    main()
