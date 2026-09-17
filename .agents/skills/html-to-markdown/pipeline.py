#!/usr/bin/env python3
"""
pipeline.py: Master CLI Orchestrator for HTML-to-Markdown Reference Conversion.

Handles both single-file technical HTML articles (e.g. Jorge Cwik's 68kPrefetch.html)
and multi-page crawled web publications (e.g. Kuba Winnicki's Achtung! Amiga).
Emits publication-grade Obsidian Markdown directly into the destination reference directory.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import List, Optional

# Reconfigure output for Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="backslashreplace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="backslashreplace")

try:
    from bs4 import BeautifulSoup, NavigableString, Tag
except ImportError:
    BeautifulSoup = None

SKILL_DIR = Path(__file__).resolve().parent
REPO_ROOT = SKILL_DIR.parents[2]

# Try importing GeminiClient from pdf-to-markdown if available
PDF_SKILL_DIR = REPO_ROOT / ".agents" / "skills" / "pdf-to-markdown"
if str(PDF_SKILL_DIR) not in sys.path:
    sys.path.insert(0, str(PDF_SKILL_DIR))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def run_command(cmd: List[str], check: bool = True) -> bool:
    print(f"[*] Executing: {' '.join(cmd)}")
    result = subprocess.run(cmd, check=check)
    return result.returncode == 0


def clean_html_text(text: str) -> str:
    text = re.sub(r"\r\n", "\n", text)
    text = re.sub(r"[ \t]+", " ", text)
    return text.strip()


def extract_assets(input_path: Path, assets_dir: Path):
    """Invokes download_assets.py to copy/download all images and generate sidecars."""
    download_script = SKILL_DIR / "scripts" / "download_assets.py"
    assets_dir.mkdir(parents=True, exist_ok=True)

    if input_path.is_file():
        html_files = [input_path]
    else:
        html_files = sorted(list(input_path.glob("*.html")))

    for hf in html_files:
        cmd = [
            sys.executable,
            str(download_script),
            "--html",
            str(hf),
            "--assets-dir",
            str(assets_dir),
        ]
        try:
            subprocess.run(cmd, check=True)
        except Exception as e:
            print(f"[!] Warning: Asset extraction failed for {hf.name}: {e}", file=sys.stderr)

    if assets_dir.exists() and not any(assets_dir.iterdir()):
        try:
            assets_dir.rmdir()
        except Exception:
            pass


def html_table_to_gfm(table_tag: Tag) -> str:
    """Converts a standard HTML table to GitHub-flavored Markdown."""
    rows = []
    for tr in table_tag.find_all("tr"):
        cells = [clean_html_text(c.get_text()) for c in tr.find_all(["th", "td"])]
        if cells:
            rows.append(cells)

    if not rows:
        return ""

    num_cols = max(len(r) for r in rows)
    # Normalize row lengths
    norm_rows = [r + [""] * (num_cols - len(r)) for r in rows]

    lines = []
    # Header row
    header = norm_rows[0]
    lines.append("| " + " | ".join(c.replace("|", "\\|") for c in header) + " |")
    lines.append("| " + " | ".join(["---"] * num_cols) + " |")

    # Body rows
    for r in norm_rows[1:]:
        lines.append("| " + " | ".join(c.replace("|", "\\|") for c in r) + " |")

    return "\n".join(lines)


def dom_to_markdown(soup: BeautifulSoup, base_heading_level: int = 1) -> str:
    """Fallback deterministic DOM converter when LLM is offline."""
    body = soup.find("body") or soup
    output_parts = []

    for element in body.descendants:
        if not isinstance(element, Tag):
            continue

        if element.name in ["h1", "h2", "h3", "h4", "h5", "h6"]:
            level = int(element.name[1]) + (base_heading_level - 1)
            level = max(1, min(6, level))
            h_text = clean_html_text(element.get_text())
            if h_text:
                output_parts.append(f"\n\n{'#' * level} {h_text}\n\n")

        elif element.name == "p":
            p_text = clean_html_text(element.get_text())
            if p_text:
                output_parts.append(f"{p_text}\n\n")

        elif element.name in ["pre", "code"]:
            if element.parent and element.parent.name in ["pre", "code"]:
                continue
            code_text = element.get_text().strip()
            if code_text:
                lang = "m68k" if any(kw in code_text.lower() for kw in ["move", "lea", "jsr", "rts", "nop", "d0", "a0"]) else "text"
                output_parts.append(f"\n```{lang}\n{code_text}\n```\n\n")

        elif element.name == "table":
            table_md = html_table_to_gfm(element)
            if table_md:
                output_parts.append(f"\n{table_md}\n\n")

        elif element.name == "img":
            src = element.get("src", "")
            alt = element.get("alt", "Figure")
            if src:
                fname = Path(src).name
                output_parts.append(f"![{alt}](assets/{fname})\n\n")

    full_md = "".join(output_parts)
    full_md = re.sub(r"\n{3,}", "\n\n", full_md)
    return full_md.strip()


def convert_with_llm(html_content: str, document_title: str, prompt_file: Path) -> Optional[str]:
    """Transcribes HTML to Markdown using GeminiClient and llm-transcription-prompt.md."""
    if not GeminiClient:
        return None

    try:
        config_path = PDF_SKILL_DIR / "config.yaml"
        config = {}
        if config_path.is_file():
            import yaml
            with open(config_path, "r", encoding="utf-8") as f:
                config = yaml.safe_load(f) or {}
        if "llm" not in config:
            config["llm"] = {
                "model_prose": "gemini-3.8-flash",
                "model_vision": "gemini-3.8-flash",
                "temperature": 0.1,
                "default_thinking_budget": 0,
            }

        gemini = GeminiClient(config)
        base_prompt = ""
        if prompt_file.is_file():
            base_prompt = prompt_file.read_text(encoding="utf-8")

        prompt = (
            f"{base_prompt}\n\n"
            f"You are transcribing the following HTML technical documentation into publication-grade Markdown.\n"
            f"Document Title: {document_title}\n\n"
            f"HTML Content:\n```html\n{html_content[:40000]}\n```\n\n"
            f"Return ONLY the complete, publication-grade Markdown text."
        )

        return gemini.generate_text(prompt, stage="html_to_markdown")
    except Exception as e:
        print(f"[!] Note: LLM conversion unavailable ({e}). Falling back to deterministic DOM extraction.", file=sys.stderr)
        return None


def convert_single_html(input_file: Path, output_dir: Path, doc_title: str) -> Path:
    print(f"[*] Processing single HTML article: {input_file.name}")
    raw_html = input_file.read_text(encoding="utf-8", errors="replace")
    soup = BeautifulSoup(raw_html, "html.parser") if BeautifulSoup else None

    title = doc_title or (soup.title.string.strip() if soup and soup.title and soup.title.string else input_file.stem)
    out_file = output_dir / f"{title}.md"

    # 1. Extract assets
    assets_dir = output_dir / "assets"
    extract_assets(input_file, assets_dir)

    # 2. Transcribe HTML
    prompt_file = SKILL_DIR / "references" / "llm-transcription-prompt.md"
    md_content = convert_with_llm(raw_html, title, prompt_file)

    if not md_content:
        # Fallback to DOM extraction
        frontmatter = (
            "---\n"
            f"title: \"{title}\"\n"
            f"source: \"{input_file.name}\"\n"
            "tags:\n"
            "  - amiga\n"
            "  - reference\n"
            "  - hardware\n"
            "properties:\n"
            f"  title: \"{title}\"\n"
            "---\n\n"
            f"# {title}\n\n"
        )
        body_md = dom_to_markdown(soup) if soup else raw_html
        md_content = frontmatter + body_md

    out_file.write_text(md_content, encoding="utf-8")
    if assets_dir.exists() and not any(assets_dir.iterdir()):
        try:
            assets_dir.rmdir()
        except Exception:
            pass
    print(f"[+] Emitted Markdown: {out_file.name} ({len(md_content)} chars)")
    return out_file


def convert_crawl_directory(input_dir: Path, output_dir: Path, doc_title: str) -> Path:
    print(f"[*] Processing multi-page HTML crawl in: {input_dir.name}")
    title = doc_title or input_dir.name
    out_file = output_dir / f"{title}.md"

    # 1. Extract assets
    assets_dir = output_dir / "assets"
    extract_assets(input_dir, assets_dir)

    # 2. Read all HTML pages in canonical sequence
    index_file = input_dir / "index.html"
    page_order = []
    if index_file.is_file() and BeautifulSoup:
        idx_soup = BeautifulSoup(index_file.read_text(encoding="utf-8", errors="replace"), "html.parser")
        for a in idx_soup.find_all("a", href=True):
            href = a["href"].split("#")[0]
            if href.endswith(".html") and (input_dir / href).is_file() and href not in page_order:
                page_order.append(href)

    # Add remaining pages
    for f in sorted(input_dir.glob("*.html")):
        if f.name not in page_order:
            page_order.append(f.name)

    print(f"    Discovered {len(page_order)} subpages to consolidate.")

    # Frontmatter and unified document header
    frontmatter = (
        "---\n"
        f"title: \"{title}\"\n"
        f"source: \"{input_dir.name}\"\n"
        "tags:\n"
        "  - amiga\n"
        "  - reference\n"
        "  - hardware\n"
        "  - chipset\n"
        "properties:\n"
        f"  title: \"{title}\"\n"
        "---\n\n"
        f"# {title}\n\n"
    )

    toc_entries = []
    chapter_sections = []

    for idx, p_name in enumerate(page_order, 1):
        p_path = input_dir / p_name
        p_html = p_path.read_text(encoding="utf-8", errors="replace")
        p_soup = BeautifulSoup(p_html, "html.parser") if BeautifulSoup else None

        ch_name = p_path.stem.replace("_", " ").title()
        if p_name.lower() == "index.html":
            ch_name = "Overview & Introduction"

        ch_id = re.sub(r"\W+", "-", ch_name).strip("-").lower()
        toc_entries.append(f"- [{ch_name}](#{ch_id})")

        ch_md = dom_to_markdown(p_soup, base_heading_level=2) if p_soup else p_html
        chapter_sections.append(f"\n\n## {ch_name}\n\n{ch_md}")

    full_toc = "## Table of Contents\n\n" + "\n".join(toc_entries) + "\n\n---\n\n"
    unified_content = frontmatter + full_toc + "".join(chapter_sections)

    out_file.write_text(unified_content, encoding="utf-8")
    if assets_dir.exists() and not any(assets_dir.iterdir()):
        try:
            assets_dir.rmdir()
        except Exception:
            pass
    print(f"[+] Emitted consolidated Markdown: {out_file.name} ({len(unified_content)} chars)")
    return out_file


def main():
    parser = argparse.ArgumentParser(description="HTML-to-Markdown Reference Conversion Pipeline")
    parser.add_argument("--input", "-i", type=str, required=True, help="Input HTML file or crawl directory")
    parser.add_argument("--output-dir", "-o", type=str, required=True, help="Destination directory for Markdown and assets")
    parser.add_argument("--document-name", "-n", type=str, default=None, help="Document title for output filename and metadata")
    parser.add_argument("--force", "-f", action="store_true", help="Force re-conversion even if target file exists")

    args = parser.parse_args()
    input_path = Path(args.input).resolve()
    output_dir = Path(args.output_dir).resolve()

    if not input_path.exists():
        print(f"[!] Error: Input path does not exist: {input_path}", file=sys.stderr)
        sys.exit(1)

    output_dir.mkdir(parents=True, exist_ok=True)
    doc_name = args.document_name or (output_dir.name if output_dir.name else input_path.stem)

    if input_path.is_file():
        convert_single_html(input_path, output_dir, doc_name)
    else:
        convert_crawl_directory(input_path, output_dir, doc_name)

    assets_dir = output_dir / "assets"
    if assets_dir.exists() and not any(assets_dir.iterdir()):
        try:
            assets_dir.rmdir()
        except Exception:
            pass

    print("[+] HTML conversion completed successfully.")


if __name__ == "__main__":
    main()
