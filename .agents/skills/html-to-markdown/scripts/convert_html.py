#!/usr/bin/env python3
"""HTML to Markdown Converter for Technical Documentation.

Converts legacy HTML exports (Microsoft Word HTML), vintage technical papers,
and multi-page documentation into publication-quality Obsidian Markdown.
"""

from __future__ import annotations

import argparse
import html
import os
import re
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, List, Optional, Set, Tuple
from bs4 import BeautifulSoup, Comment, Tag

# Windows-1252 to Unicode translation mapping
CP1252_REPLACEMENTS = {
    0x85: "...",      # Horizontal ellipsis
    0x91: "'",        # Left single quote
    0x92: "'",        # Right single quote / apostrophe
    0x93: '"',        # Left double quote
    0x94: '"',        # Right double quote
    0x95: "*",        # Bullet
    0x96: "--",       # En dash
    0x97: "---",      # Em dash
    0x99: "(TM)",     # Trademark
    0xA0: " ",        # Non-breaking space
}

M68K_MNEMONICS = {
    "ABCD", "ADD", "ADDA", "ADDI", "ADDQ", "ADDX", "AND", "ANDI", "ASL", "ASR",
    "BCC", "BCS", "BEQ", "BGE", "BGT", "BHI", "BLE", "BLS", "BLT", "BMI", "BNE",
    "BPL", "BVC", "BVS", "BRA", "BSR", "BCHG", "BCLR", "BSET", "BTST", "CHK",
    "CLR", "CMP", "CMPA", "CMPI", "CMPM", "DBCC", "DBCS", "DBEQ", "DBF", "DBGE",
    "DBGT", "DBHI", "DBLE", "DBLS", "DBLT", "DBMI", "DBNE", "DBPL", "DBT",
    "DBVC", "DBVS", "DBRA", "DIVS", "DIVU", "EOR", "EORI", "EXG", "EXT",
    "ILLEGAL", "JMP", "JSR", "LEA", "LINK", "LSL", "LSR", "MOVE", "MOVEA",
    "MOVEM", "MOVEP", "MOVEQ", "MULS", "MULU", "NBCD", "NEG", "NEGX", "NOP",
    "NOT", "OR", "ORI", "PEA", "RESET", "ROL", "ROR", "ROXL", "ROXR", "RTE",
    "RTR", "RTS", "SBCD", "SCC", "SCS", "SEQ", "SF", "SGE", "SGT", "SHI",
    "SLE", "SLS", "SLT", "SMI", "SNE", "SPL", "ST", "SVC", "SVS", "STOP",
    "SUB", "SUBA", "SUBI", "SUBQ", "SUBX", "SWAP", "TAS", "TRAP", "TRAPV",
    "TST", "UNLK",
}


@dataclass
class HeadingItem:
    level: int
    title: str
    slug: str


def make_anchor_slug(heading_text: str) -> str:
    """Generate a GitHub and Obsidian compatible anchor slug."""
    clean = re.sub(r"[^\w\s-]", "", heading_text.lower())
    return re.sub(r"[\s_]+", "-", clean).strip("-")


def detect_encoding(file_path: Path) -> str:
    """Detect character encoding from HTML meta tags or byte signatures."""
    with open(file_path, "rb") as stream:
        raw_header = stream.read(4096).lower()

    if b"charset=windows-1252" in raw_header or b"windows-1252" in raw_header:
        return "windows-1252"
    if b"charset=iso-8859" in raw_header:
        return "iso-8859-1"
    if b"charset=utf-8" in raw_header:
        return "utf-8"

    cp1252_markers = bytes([0x91, 0x92, 0x93, 0x94, 0x96, 0x97, 0x85])
    if any(marker in raw_header for marker in cp1252_markers):
        return "windows-1252"

    return "utf-8"


def read_html(file_path: Path) -> Tuple[str, str]:
    """Read HTML file with character encoding normalization."""
    encoding = detect_encoding(file_path)
    try:
        with open(file_path, "r", encoding=encoding, errors="replace") as stream:
            content = stream.read()
    except Exception:
        with open(file_path, "r", encoding="windows-1252", errors="replace") as stream:
            content = stream.read()
        encoding = "windows-1252"

    normalized_chars = []
    for char in content:
        code = ord(char)
        if code in CP1252_REPLACEMENTS:
            normalized_chars.append(CP1252_REPLACEMENTS[code])
        else:
            normalized_chars.append(char)
    return "".join(normalized_chars), encoding


def clean_web_cruft_and_unwrap_tables(soup: BeautifulSoup, strip_nav: bool = True) -> None:
    """Remove scripts, styles, metadata, hidden elements, and unwrap layout tables."""
    for element in soup(["script", "style", "xml", "meta", "link"]):
        element.decompose()

    for comment in soup.find_all(string=lambda text: isinstance(text, Comment)):
        comment.extract()

    # Remove hidden elements and optional navigation bars
    to_decompose = []
    for element in list(soup.find_all(True)):
        if element.attrs is None:
            continue
        style = element.get("style", "")
        if isinstance(style, str) and ("display: none" in style.lower() or "visibility: hidden" in style.lower()):
            to_decompose.append(element)
            continue

        if strip_nav:
            elem_id = str(element.get("id", "")).lower()
            cls_val = element.get("class", "")
            elem_cls = " ".join(cls_val).lower() if isinstance(cls_val, list) else str(cls_val).lower()
            if elem_id in ("menu", "navigator", "footer", "nav", "sidebar") or elem_cls in ("menu", "navigator", "footer", "nav", "sidebar"):
                to_decompose.append(element)
                continue

    for elem in to_decompose:
        elem.decompose()

    # Unwrap layout tables that wrap block-level contents
    changed = True
    while changed:
        changed = False
        for t in soup.find_all("table"):
            if t.find(["p", "h1", "h2", "h3", "h4", "table", "div", "pre", "ul", "ol"]):
                for tr in t.find_all("tr", recursive=False):
                    for td in tr.find_all(["td", "th"], recursive=False):
                        td.unwrap()
                    tr.unwrap()
                t.unwrap()
                changed = True
                break

    # Strip leftover styling and layout attributes
    for tag in soup.find_all(True):
        removals = [
            key for key in tag.attrs
            if key.startswith(("mso-", "v:", "o:", "w:"))
            or key in ("class", "style", "bgcolor", "align", "valign", "border", "cellpadding", "cellspacing")
        ]
        for key in removals:
            del tag[key]


def is_register_trace_line(text: str) -> bool:
    """Check if a line is a CPU register trace like IRC: $1234."""
    clean = text.strip().upper()
    trace_prefixes = ("IRC:", "IR:", "IRD:", "PC:", "SR:", "CCR:", "D0:", "D1:", "A0:", "A1:", "A7:", "SP:")
    return clean.startswith(trace_prefixes) or bool(re.match(r"^(IRC|IRD|IR|PC|SP)\s*:\s*\$[0-9A-F]{4}", clean))


def is_assembly_snippet(text: str) -> bool:
    """Check if a text block represents M68000 assembly instructions."""
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    if not lines:
        return False

    if any(is_register_trace_line(line) for line in lines):
        return True

    mnemonic_hits = 0
    address_prefix_hits = 0

    for line in lines:
        if re.match(r"^(\$[0-9a-fA-F]{4,8}|[0-9a-fA-F]{6}):", line):
            address_prefix_hits += 1
            continue

        tokens = re.split(r"[\s,;]+", line.upper())
        for token in tokens:
            cleaned = token.split(".")[0]
            if cleaned in M68K_MNEMONICS:
                mnemonic_hits += 1
                break

    if address_prefix_hits >= 1 and mnemonic_hits >= 1:
        return True
    if mnemonic_hits >= 2:
        return True
    if len(lines) == 1 and mnemonic_hits >= 1 and any(reg in lines[0].upper() for reg in ("D0", "D1", "A0", "A1", "SP", "PC")):
        return True

    return False


def convert_table(table: Tag) -> Optional[str]:
    """Convert an HTML table to a clean GitHub-Flavored Markdown table."""
    rows = table.find_all("tr")
    if not rows:
        return None

    if len(rows) == 1:
        cells = rows[0].find_all(["td", "th"])
        if len(cells) == 1:
            return None

    parsed_rows: List[List[str]] = []
    max_cols = 0

    for row in rows:
        cells = row.find_all(["td", "th"])
        row_content = []
        for cell in cells:
            text = " ".join(cell.get_text().split())
            text = text.replace("|", "\\|")
            row_content.append(text)
        if row_content:
            max_cols = max(max_cols, len(row_content))
            parsed_rows.append(row_content)

    if not parsed_rows or max_cols == 0:
        return None

    for r in parsed_rows:
        while len(r) < max_cols:
            r.append("")

    header_row = parsed_rows[0]
    separator = [":---"] * max_cols

    md_lines = [
        "| " + " | ".join(header_row) + " |",
        "| " + " | ".join(separator) + " |",
    ]

    for data_row in parsed_rows[1:]:
        md_lines.append("| " + " | ".join(data_row) + " |")

    return "\n".join(md_lines)


class HtmlToMarkdownConverter:
    """Comprehensive converter from HTML to Obsidian Markdown."""

    def __init__(
        self,
        title: Optional[str] = None,
        author: Optional[str] = None,
        source_url: Optional[str] = None,
        generate_toc: bool = True,
        input_dir: Optional[Path] = None,
        output_dir: Optional[Path] = None,
        copy_assets: bool = False,
    ):
        self.override_title = title
        self.override_author = author
        self.source_url = source_url
        self.generate_toc = generate_toc
        self.input_dir = input_dir
        self.output_dir = output_dir
        self.copy_assets = copy_assets
        self.headings: List[HeadingItem] = []

    def extract_metadata(self, soup: BeautifulSoup) -> Tuple[str, str]:
        title = self.override_title
        author = self.override_author or "Unknown"

        if not title:
            if soup.title and soup.title.string:
                title = soup.title.string.strip()
            elif soup.find("h1"):
                title = soup.find("h1").get_text(strip=True)
            elif soup.find("h2"):
                title = soup.find("h2").get_text(strip=True)
            else:
                title = "Technical Reference Document"

        meta_author = soup.find("meta", attrs={"name": re.compile(r"author", re.I)})
        if meta_author and meta_author.get("content"):
            author = meta_author["content"].strip()

        return title, author

    def build_frontmatter(self, title: str, author: str) -> str:
        lines = [
            "---",
            f'title: "{title}"',
            f'author: "{author}"',
            'version: "1.0 (Converted Edition)"',
        ]
        if self.source_url:
            lines.append(f'source: "{self.source_url}"')
        lines.extend([
            "tags:",
            "  - amiga",
            "  - m68k",
            "  - m68000",
            "  - cpu",
            "  - architecture",
            "  - reference",
            "properties:",
            '  processor: "Motorola MC68000"',
            '  architecture: "16/32-bit CISC"',
            f'  author: "{author}"',
            "---",
            "",
        ])
        return "\n".join(lines)

    def handle_asset_copy(self, rel_src: str) -> None:
        """Copy referenced asset and sidecar to output directory."""
        if not self.copy_assets or not self.input_dir or not self.output_dir:
            return

        clean_rel = rel_src.lstrip("./").replace("/", os.sep)
        src_path = self.input_dir / clean_rel
        dest_path = self.output_dir / clean_rel

        if src_path.exists():
            dest_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src_path, dest_path)

            sidecar_src = Path(str(src_path) + ".txt")
            if sidecar_src.exists():
                sidecar_dest = Path(str(dest_path) + ".txt")
                shutil.copy2(sidecar_src, sidecar_dest)

    def convert_document(self, html_content: str) -> str:
        soup = BeautifulSoup(html_content, "html.parser")
        doc_title, doc_author = self.extract_metadata(soup)
        clean_web_cruft_and_unwrap_tables(soup)

        body = soup.body if soup.body else soup

        # Find top-level content elements, ignoring those nested in tables
        raw_elements = list(body.find_all(["h1", "h2", "h3", "h4", "p", "table", "pre", "ul", "ol", "img"]))
        top_elements: List[Tag] = []
        for elem in raw_elements:
            if elem.find_parent("table") is None:
                top_elements.append(elem)

        content_lines: List[str] = []
        seen_title = False

        for elem in top_elements:
            if elem.name == "img":
                src = elem.get("src", "")
                alt = elem.get("alt", "Diagram")
                if src:
                    self.handle_asset_copy(src)
                    content_lines.append(f"\n![{alt}]({src})\n")
                continue

            if elem.name == "table":
                img = elem.find("img")
                if img:
                    src = img.get("src", "")
                    alt = img.get("alt", "Diagram")
                    if src:
                        self.handle_asset_copy(src)
                        content_lines.append(f"\n![{alt}]({src})\n")
                    continue

                table_md = convert_table(elem)
                if table_md:
                    content_lines.append("\n" + table_md + "\n")
                continue

            text = elem.get_text().strip()
            if not text:
                continue

            # Heading tags
            if elem.name in ("h1", "h2", "h3", "h4"):
                clean_text = " ".join(text.split())
                if not seen_title and elem.name in ("h1", "h2"):
                    seen_title = True
                    content_lines.append(f"# {clean_text}\n")
                    continue

                if elem.name in ("h1", "h2"):
                    level = 2
                elif elem.name == "h3":
                    level = 3
                else:
                    level = 4

                slug = make_anchor_slug(clean_text)
                self.headings.append(HeadingItem(level=level, title=clean_text, slug=slug))
                hashes = "#" * level
                content_lines.append(f"\n{hashes} {clean_text}\n")
                continue

            # Code / Assembly detection
            if elem.name == "pre" or is_assembly_snippet(text):
                clean_lines = [line.rstrip() for line in text.splitlines()]
                lang = "assembly" if is_assembly_snippet(text) else "text"
                content_lines.append(f"\n```{lang}\n" + "\n".join(clean_lines) + "\n```\n")
                continue

            # Lists
            if elem.name in ("ul", "ol"):
                items = elem.find_all("li")
                for i, item in enumerate(items, 1):
                    item_text = " ".join(item.get_text().split())
                    prefix = f"{i}. " if elem.name == "ol" else "- "
                    content_lines.append(f"{prefix}{item_text}")
                content_lines.append("")
                continue

            clean_para = " ".join(text.split())

            # Check if uppercase paragraph is a true section heading (excluding register traces)
            if (
                len(clean_para) < 60
                and clean_para.isupper()
                and len(clean_para.split()) <= 4
                and not is_register_trace_line(clean_para)
                and "$" not in clean_para
            ):
                level = 3
                slug = make_anchor_slug(clean_para)
                self.headings.append(HeadingItem(level=level, title=clean_para, slug=slug))
                content_lines.append(f"\n### {clean_para}\n")
                continue

            content_lines.append(f"{clean_para}\n")

        toc_lines = []
        if self.generate_toc and self.headings:
            toc_lines.append("## Table of Contents\n")
            for h in self.headings:
                indent = "  " * (h.level - 2)
                toc_lines.append(f"{indent}- [{h.title}](#{h.slug})")
            toc_lines.append("\n---\n")

        frontmatter = self.build_frontmatter(doc_title, doc_author)
        assembled = frontmatter + "\n".join(toc_lines) + "\n".join(content_lines)
        assembled = re.sub(r"\n{3,}", "\n\n", assembled).strip() + "\n"
        return assembled


def main() -> int:
    parser = argparse.ArgumentParser(description="Convert technical HTML to Obsidian Markdown.")
    parser.add_argument("--input", "-i", required=True, help="Input HTML file path")
    parser.add_argument("--output", "-o", help="Output Markdown file path")
    parser.add_argument("--inspect", action="store_true", help="Inspect and print document summary only")
    parser.add_argument("--title", help="Override document title")
    parser.add_argument("--author", help="Override document author")
    parser.add_argument("--source-url", help="Source URL for frontmatter")
    parser.add_argument("--no-toc", action="store_true", help="Disable Table of Contents generation")
    parser.add_argument("--copy-assets", action="store_true", help="Copy referenced assets and sidecars to output folder")

    args = parser.parse_args()
    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: Input path '{input_path}' does not exist.", file=sys.stderr)
        return 1

    if input_path.is_dir():
        html_files = sorted(input_path.glob("*.html"))
        if not html_files:
            print(f"Error: No HTML files found in '{input_path}'.", file=sys.stderr)
            return 1
        print(f"Found {len(html_files)} HTML files in '{input_path}'.")
        out_dir = Path(args.output) if args.output else input_path / "markdown_out"
        out_dir.mkdir(parents=True, exist_ok=True)

        success_count = 0
        for html_file in html_files:
            content, enc = read_html(html_file)
            out_file = out_dir / (html_file.stem + ".md")
            conv = HtmlToMarkdownConverter(
                source_url=args.source_url,
                generate_toc=not args.no_toc,
                input_dir=html_file.parent,
                output_dir=out_dir,
                copy_assets=args.copy_assets,
            )
            md = conv.convert_document(content)
            with open(out_file, "w", encoding="utf-8") as stream:
                stream.write(md)
            print(f"  [OK] Converted '{html_file.name}' -> '{out_file.name}' ({len(md.splitlines())} lines)")
            success_count += 1

        print(f"Successfully converted {success_count} files into '{out_dir}'.")
        return 0

    html_content, encoding = read_html(input_path)
    print(f"Loaded '{input_path}' ({len(html_content)} characters, encoding: {encoding})")

    output_path = Path(args.output) if args.output else None

    converter = HtmlToMarkdownConverter(
        title=args.title,
        author=args.author,
        source_url=args.source_url,
        generate_toc=not args.no_toc,
        input_dir=input_path.parent,
        output_dir=output_path.parent if output_path else None,
        copy_assets=args.copy_assets,
    )

    if args.inspect:
        soup = BeautifulSoup(html_content, "html.parser")
        title, author = converter.extract_metadata(soup)
        print(f"Title   : {title}")
        print(f"Author  : {author}")
        print(f"H tags  : {len(soup.find_all(['h1', 'h2', 'h3', 'h4']))}")
        print(f"Images  : {len(soup.find_all('img'))}")
        print(f"Tables  : {len(soup.find_all('table'))}")
        return 0

    markdown = converter.convert_document(html_content)

    if output_path:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as stream:
            stream.write(markdown)
        print(f"Wrote converted markdown to '{output_path}' ({len(markdown.splitlines())} lines)")
    else:
        print(markdown)

    return 0


if __name__ == "__main__":
    sys.exit(main())
