"""Find Chapter Start Pages in Technical PDFs.

Scans PDF bookmarks (TOC) and in-page text layout to identify where each chapter,
section, or appendix begins. Writes the resulting metadata to a JSON file in the
same directory as the source PDF.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage4-find-chapters/scripts/find_chapters.py "path/to/manual_ocr.pdf"
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

# Ensure safe console output on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

try:
    import pymupdf as fitz
except ImportError:
    try:
        import fitz  # type: ignore
    except ImportError:
        sys.stderr.write("Error: PyMuPDF (fitz) is required. Install via 'pip install pymupdf'.\n")
        sys.exit(1)


def is_noise_title(title: str) -> bool:
    """Check if bookmark title is an internal scanner artifact, file path, or filename."""
    t = re.sub(r"[\u200e\u200f\s]+", " ", title).strip().lower()
    return bool(
        t.endswith((".tif", ".tiff", ".png", ".jpg", ".jpeg", ".pdf", ".bmp"))
        or "\\" in t
        or "/" in t
        or re.match(r"^_[0-9]+$", t)
        or re.match(r"^[a-z0-9_-]+_[0-9]+[a-z]?$", t)
        or re.match(r"^[0-9]*[a-z0-9_-]*(?:trm|scan|ocr|img|raw)[a-z0-9_-]*$", t)
        or re.match(r"^[a-z0-9]+-[0-9]+[a-z]?$", t)
        or len(t) < 2
    )


def clean_noise_line(line: str) -> bool:
    """Check if a line in a text block is OCR noise or decorative artifact."""
    l = line.strip()
    if not l:
        return True
    if re.match(r"^[^\w\s]+$", l):
        return True
    if re.match(r"^[a-zA-Z][.:,]?$", l):
        return True
    if re.match(
        r"^(?:ee+|eee+|a\s+i|a\s+ae|ln|nn|eee\s+eee|oe|oo|wn\s+em|er\s+renee)$",
        l,
        re.IGNORECASE,
    ):
        return True
    return False


def clean_lines(lines: List[str]) -> List[str]:
    """Filter out OCR artifacts and empty lines from block text."""
    cleaned = []
    for l in lines:
        if not clean_noise_line(l):
            cleaned.append(l.strip())
    return cleaned


def clean_subtitle(subtitle: str) -> str:
    """Normalize subtitle text, punctuation, and common OCR substitutions."""
    s = re.sub(r"[\u200e\u200f]+", " ", subtitle)
    s = re.sub(r"\s+", " ", s).strip()
    s = re.sub(r"^[-–—:.]\s*", "", s).strip()
    s = re.sub(r"\bSCS!\b", "SCSI", s)
    s = re.sub(r"\bBlOS\b", "BIOS", s)
    return s


def extract_authoritative_title(
    unit_str: str,
    raw_lines: List[str],
    line_idx: int,
    same_line_title: str,
    blocks: List[Any],
    b_idx: int,
    page_h: float,
) -> str:
    """Extract chapter/section title taking following lines and blocks into account."""
    subsequent = clean_lines(raw_lines[line_idx + 1 :])
    subtitle = ""
    if same_line_title and not clean_noise_line(same_line_title):
        subtitle = same_line_title
    elif subsequent:
        subtitle = " ".join(subsequent[:2])
    elif b_idx + 1 < len(blocks):
        nb = blocks[b_idx + 1]
        if nb[1] - blocks[b_idx][3] < 120 and nb[3] < page_h - 55:
            nb_lines = clean_lines([l.strip() for l in nb[4].splitlines() if l.strip()])
            if nb_lines:
                subtitle = " ".join(nb_lines[:2])

    cleaned_sub = clean_subtitle(subtitle)
    return f"{unit_str} - {cleaned_sub}" if cleaned_sub else unit_str


def get_on_page_heading(page: fitz.Page, fallback_title: str) -> str:
    """Extract authoritative chapter title directly from the page blocks."""
    heading_regex = re.compile(
        r"^(?:(SECTION|CHAPTER)\s+(\d+(?:\.\d+)*|[IVXLCDM]+)|(APPENDIX|APPX\.?)\s*([A-Z0-9]+(?:\.[0-9]+)*))(?:\s*[:.\-–—]\s*(.*)|\s*$)",
        re.IGNORECASE,
    )
    toc_regex = re.compile(r"^(TABLE\s+OF\s+CONTENTS|CONTENTS)$", re.IGNORECASE)

    blocks = page.get_text("blocks")
    blocks.sort(key=lambda b: (b[1], b[0]))
    page_h = page.rect.height

    for b_idx, b in enumerate(blocks):
        if b[3] <= 55 or b[1] >= page_h - 55:
            continue
        lines = [l.strip() for l in b[4].splitlines() if l.strip()]
        if not lines:
            continue

        for line_idx, line in enumerate(lines[:4]):
            m = heading_regex.match(line)
            if m:
                if m.group(1):
                    unit_word = m.group(1).title() if not m.group(1).isupper() else m.group(1)
                    unit = f"{unit_word} {m.group(2).upper()}"
                    same_line = m.group(5).strip() if m.group(5) else ""
                else:
                    unit_word = "Appendix" if not m.group(3).isupper() else "APPENDIX"
                    unit = f"{unit_word} {m.group(4).upper()}"
                    same_line = m.group(5).strip() if m.group(5) else ""

                return extract_authoritative_title(
                    unit, lines, line_idx, same_line, blocks, b_idx, page_h
                )

        if toc_regex.match(lines[0]):
            return "Table of Contents"

    norm = re.sub(r"(?i)\bsec\.\s*(\d+)[\s:-]*", r"SECTION \1 - ", fallback_title)
    norm = re.sub(r"(?i)\bappx\.\s*([A-Z])[\s:-]*", r"APPENDIX \1 - ", norm)
    return clean_subtitle(norm)


def get_front_matter_title(doc: fitz.Document, end_page: int) -> str:
    """Inspect pre-chapter pages to determine if Preface / Foreword exists."""
    for p in range(min(end_page, len(doc))):
        text = doc[p].get_text()
        if re.search(r"\b(preface|foreword)\b", text, re.IGNORECASE):
            return "Front Matter & Preface"
    return "Front Matter"


def determine_entry_type(title: str) -> str:
    """Classify entry type based on title keywords."""
    t = title.strip().lower()
    if "front matter" in t:
        return "front_matter"
    if "contents" in t:
        return "contents"
    if t.startswith("section"):
        return "section"
    if t.startswith("chapter"):
        return "chapter"
    if t.startswith("appendix") or t.startswith("appx"):
        return "appendix"
    if "index" in t:
        return "index"
    if "glossary" in t:
        return "glossary"
    return "chapter"


# Acronyms and technical abbreviations preserved in uppercase
ACRONYMS: Set[str] = {
    "CPU", "DMA", "RAM", "ROM", "PAL", "CIA", "ALU", "MMU", "FPU",
    "I/O", "IO", "MIDI", "SCSI", "BIOS", "ASIC", "TTL", "MOS", "NMOS",
    "HMOS", "CMOS", "DSP", "LSI", "VLSI", "VME", "CRT", "RGB", "NTSC",
    "SECAM", "CD-ROM", "PC/XT", "PC", "XT", "AT", "DIP", "PLCC", "PGA", "UART",
}

# Proper nouns or brand names needing specific capitalization
SPECIAL_CASES: Dict[str, str] = {
    "AMIGA": "Amiga",
    "MOTOROLA": "Motorola",
    "COMMODORE": "Commodore",
    "AGNUS": "Agnus",
    "DENISE": "Denise",
    "PAULA": "Paula",
}

# Standard English lower-case words in title case (unless first or last word of title/subtitle)
LOWERCASE_WORDS: Set[str] = {
    "a", "an", "the", "and", "but", "or", "nor", "for", "so", "yet",
    "as", "at", "by", "in", "of", "off", "on", "per", "to", "up", "via", "with",
}

ROMAN_REGEX = re.compile(
    r"^(?=[MDCLXVI]+$)M*(C[MD]|D?C{0,3})(X[CL]|L?X{0,3})(I[XV]|V?I{0,3})$",
    re.IGNORECASE,
)
NON_ROMAN = {"IN", "AM", "IT", "IS", "AT", "ON", "DO", "ME", "ID"}


def is_roman_numeral(w: str) -> bool:
    """Check if word is a valid Roman numeral (excluding common English words like 'IN')."""
    u = w.upper()
    return bool(ROMAN_REGEX.match(u) and u not in NON_ROMAN)


def is_model_or_code(w: str) -> bool:
    """Check if word is a hardware model, chip part number, or alphanumeric code (e.g. MC68000, A2000)."""
    return bool(re.match(r"^[A-Za-z]+[0-9]+[A-Za-z0-9]*$", w))


def title_case_word(w: str, is_first: bool, is_last: bool, prev_word: str) -> str:
    """Format single word into title case while honoring acronyms, Roman numerals, and punctuation."""
    if not w:
        return w
    m = re.match(r"^([^\w]*)(.*?)([^\w]*)$", w)
    if not m:
        return w
    prefix, core, suffix = m.groups()
    if not core:
        return w

    u = core.upper()
    if u in SPECIAL_CASES:
        return prefix + SPECIAL_CASES[u] + suffix
    if u in ACRONYMS:
        return prefix + u + suffix

    if "/" in core:
        subparts = [
            title_case_word(p, is_first and i == 0, is_last and i == len(core.split("/")) - 1, prev_word)
            for i, p in enumerate(core.split("/"))
        ]
        return prefix + "/".join(subparts) + suffix

    if "-" in core:
        subparts = [
            title_case_word(p, is_first and i == 0, is_last and i == len(core.split("-")) - 1, prev_word)
            for i, p in enumerate(core.split("-"))
        ]
        return prefix + "-".join(subparts) + suffix

    if is_roman_numeral(core):
        return prefix + u + suffix
    if is_model_or_code(core):
        return prefix + core.upper() + suffix
    if u in {"A", "B", "C", "D", "E", "F"} and prev_word.lower() in {
        "appendix", "section", "part", "annex", "table", "figure"
    }:
        return prefix + u + suffix
    if not is_first and not is_last and core.lower() in LOWERCASE_WORDS:
        return prefix + core.lower() + suffix
    if not core.isupper() and not core.islower():
        return prefix + core + suffix
    return prefix + core.capitalize() + suffix


def to_title_case(title: str) -> str:
    """Convert title string to standard English Title Case preserving acronyms, Roman numerals, and subtitles."""
    segments = re.split(r"(\s*[-–—:]\s*)", title)
    processed_segments = []
    prev_w = ""
    for seg in segments:
        if re.match(r"^\s*[-–—:]\s*$", seg):
            processed_segments.append(seg)
            prev_w = ""
            continue
        words = seg.split(" ")
        out_words = []
        for i, w in enumerate(words):
            if not w:
                out_words.append(w)
                continue
            is_first = (i == 0)
            is_last = (i == len(words) - 1)
            cw = title_case_word(w, is_first, is_last, prev_w)
            out_words.append(cw)
            m_core = re.search(r"\w+", w)
            if m_core:
                prev_w = m_core.group(0)
        processed_segments.append(" ".join(out_words))
    return "".join(processed_segments)


def extract_from_bookmarks(doc: fitz.Document) -> Optional[List[Dict[str, Any]]]:
    """Attempt chapter extraction using document outline / bookmarks."""
    toc = doc.get_toc()
    if not toc:
        return None

    valid_entries = []
    for level, title, page in toc:
        cleaned = re.sub(r"[\u200e\u200f]+", "", title).strip()
        if not is_noise_title(cleaned) and page > 0:
            valid_entries.append((level, cleaned, page))

    if not valid_entries:
        return None

    pattern = re.compile(
        r"(?i)^(chapter|sec\.|section|appendix|appx\.|table\s+of\s+contents|contents|index|glossary|preface|foreword)"
    )

    matching_entries = [(lvl, t, p) for lvl, t, p in valid_entries if pattern.search(t)]
    has_real_chapters = any(
        re.search(r"(?i)^(chapter|sec\.|section)\s*\d+", t) for _, t, _ in matching_entries
    )

    if len(doc) > 20 and not has_real_chapters:
        return None

    matching_levels = [lvl for lvl, _, _ in matching_entries]
    target_level = min(matching_levels) if matching_levels else 1

    chapters = []
    seen_pages: Set[int] = set()
    for level, title, page in valid_entries:
        if level <= target_level and page not in seen_pages:
            if pattern.search(title) or (level == 1 and not re.search(r"\.pdf$", title, re.I)):
                page_obj = doc[page - 1]
                authoritative_title = get_on_page_heading(page_obj, title)
                chapters.append({
                    "title": authoritative_title,
                    "start_page": page,
                    "type": determine_entry_type(authoritative_title),
                })
                seen_pages.add(page)

    if chapters and chapters[0]["start_page"] > 1:
        first_page = chapters[0]["start_page"]
        fm_title = get_front_matter_title(doc, first_page - 1)
        chapters.insert(0, {
            "title": fm_title,
            "start_page": 1,
            "type": "front_matter",
        })

    return chapters if len(chapters) >= 2 else None


def extract_from_text_layout(doc: fitz.Document) -> List[Dict[str, Any]]:
    """Scan page text blocks for prominent chapter / section headings."""
    candidates: List[Dict[str, Any]] = []
    heading_regex = re.compile(
        r"^(?:(SECTION|CHAPTER)\s+(\d+(?:\.\d+)*|[IVXLCDM]+)|(APPENDIX|APPX\.?)\s*([A-Z0-9]+(?:\.[0-9]+)*))(?:\s*[:.\-–—]\s*(.*)|\s*$)",
        re.IGNORECASE,
    )
    endmatter_regex = re.compile(
        r"^(GLOSSARY|INDEX|BIBLIOGRAPHY)$",
        re.IGNORECASE,
    )
    toc_heading_regex = re.compile(
        r"^(TABLE\s+OF\s+CONTENTS|CONTENTS)$",
        re.IGNORECASE,
    )

    toc_page = None
    for p in range(min(30, len(doc))):
        for b in doc[p].get_text("blocks"):
            if b[3] <= 55 or b[1] >= doc[p].rect.height - 55:
                continue
            text = b[4].strip()
            if toc_heading_regex.search(text):
                toc_page = p + 1
                break
        if toc_page:
            break

    first_chapter_page = None
    search_start = toc_page if toc_page else 0
    for p in range(search_start, len(doc)):
        page_h = doc[p].rect.height
        for b in doc[p].get_text("blocks"):
            if b[3] <= 55 or b[1] >= page_h - 55:
                continue
            lines = [l.strip() for l in b[4].splitlines() if l.strip()]
            for l in lines:
                if re.match(r"^(?:SECTION|CHAPTER)\s*1(?:\.0)?(?:[.:\s]|$)", l, re.I):
                    all_headings = [
                        line for blk in doc[p].get_text("blocks")
                        for line in blk[4].splitlines()
                        if heading_regex.match(line.strip())
                    ]
                    if len(all_headings) <= 1:
                        first_chapter_page = p + 1
                        break
            if first_chapter_page:
                break
        if first_chapter_page:
            break

    fm_end = toc_page if toc_page else (first_chapter_page or 1)
    fm_title = get_front_matter_title(doc, fm_end - 1)
    candidates.append({
        "title": fm_title,
        "start_page": 1,
        "type": "front_matter",
    })
    if toc_page and (first_chapter_page is None or toc_page < first_chapter_page):
        candidates.append({
            "title": "Table of Contents",
            "start_page": toc_page,
            "type": "contents",
        })

    scan_start = (first_chapter_page - 1) if first_chapter_page else 0
    seen_keys: Set[str] = set()

    for p in range(scan_start, len(doc)):
        page_num = p + 1
        blocks = doc[p].get_text("blocks")
        blocks.sort(key=lambda b: (b[1], b[0]))
        page_h = doc[p].rect.height

        for b_idx, b in enumerate(blocks):
            bx0, by0, bx1, by1, text, bno, btype = b
            if by1 <= 55 or by0 >= (page_h - 55):
                continue

            lines = [l.strip() for l in text.splitlines() if l.strip()]
            if not lines:
                continue

            for line_idx, line in enumerate(lines[:4]):
                m = heading_regex.match(line)
                m_end = endmatter_regex.match(line) if not m else None

                if m:
                    same_line = m.group(5).strip() if m.group(5) else ""
                    if same_line and re.match(
                        r"^(?:contains|for|to|is|in|see|that|which|with|as)\b",
                        same_line,
                        re.IGNORECASE,
                    ):
                        continue

                    if m.group(1):
                        unit_type = m.group(1).title() if not m.group(1).isupper() else m.group(1)
                        unit_num = m.group(2).upper()
                    else:
                        unit_type = "Appendix" if not m.group(3).isupper() else "APPENDIX"
                        unit_num = m.group(4).upper()

                    ch_key = f"{unit_type.upper()} {unit_num}"
                    display_unit = f"{unit_type} {unit_num}"
                    if ch_key in seen_keys:
                        continue

                    full_title = extract_authoritative_title(
                        display_unit, lines, line_idx, same_line, blocks, b_idx, page_h
                    )

                    candidates.append({
                        "title": full_title,
                        "start_page": page_num,
                        "type": unit_type.lower(),
                    })
                    seen_keys.add(ch_key)
                    break

                elif m_end and len(lines) <= 2:
                    unit = m_end.group(1).upper()
                    if unit not in seen_keys:
                        candidates.append({
                            "title": unit,
                            "start_page": page_num,
                            "type": unit.lower(),
                        })
                        seen_keys.add(unit)
                        break

    return candidates


def find_chapters(pdf_path: Path) -> Dict[str, Any]:
    """Analyze PDF and return dictionary of chapter start pages."""
    doc = fitz.open(str(pdf_path))
    total_pages = len(doc)

    sample_text_len = sum(len(doc[i].get_text().strip()) for i in range(min(5, total_pages)))
    has_text = sample_text_len > 40

    try:
        rel_pdf = str(pdf_path.relative_to(Path.cwd()))
    except ValueError:
        rel_pdf = pdf_path.name

    if not has_text:
        return {
            "pdf_file": pdf_path.name,
            "pdf_path": rel_pdf,
            "total_pages": total_pages,
            "has_text_layer": False,
            "detection_method": "none",
            "message": "Document contains no extractable text layer. Run Stage 3 (pdf-stage3-ocr) first.",
            "chapters": [],
        }

    def _enrich_chapters(ch_list: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        for idx, ch in enumerate(ch_list):
            order = idx + 1
            ch["order"] = order
            raw_title = ch.get("clean_title") or ch.get("title", f"Chapter {order}")
            ch["clean_title"] = to_title_case(raw_title)
            safe_title = re.sub(r'[<>:"/\\|?*]', "", ch["clean_title"])
            safe_title = re.sub(r"\s+", " ", safe_title).strip()
            ch["filename"] = f"{order:02d} - {safe_title}.md"
        return ch_list

    bookmark_chapters = extract_from_bookmarks(doc)
    if bookmark_chapters:
        enriched = _enrich_chapters(bookmark_chapters)
        return {
            "pdf_file": pdf_path.name,
            "pdf_path": rel_pdf,
            "total_pages": total_pages,
            "has_text_layer": True,
            "detection_method": "pdf_bookmarks",
            "chapter_count": len(enriched),
            "chapters": enriched,
        }

    text_chapters = extract_from_text_layout(doc)
    enriched = _enrich_chapters(text_chapters)
    return {
        "pdf_file": pdf_path.name,
        "pdf_path": rel_pdf,
        "total_pages": total_pages,
        "has_text_layer": True,
        "detection_method": "text_layout",
        "chapter_count": len(enriched),
        "chapters": enriched,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Find chapter starting pages in a technical PDF and save as JSON."
    )
    parser.add_argument("pdf_path", type=Path, help="Path to input PDF file")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=None,
        help="Custom output JSON path (default: <pdf_dir>/<pdf_stem>_chapters.json)",
    )
    parser.add_argument(
        "--pages",
        "-p",
        type=str,
        default=None,
        help="Optional page range to filter chapters, e.g. '1-50', '4'",
    )

    args = parser.parse_args()

    pdf_path = args.pdf_path
    if not pdf_path.exists():
        sys.stderr.write(f"Error: File not found: {pdf_path}\n")
        sys.exit(1)

    if pdf_path.is_dir():
        manual_dir = pdf_path.parent if pdf_path.name == "build" else pdf_path
        candidate = None
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            for f in build_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    candidate = f
                    break
            if not candidate:
                for f in build_dir.glob("*.pdf"):
                    if not f.name.endswith("_ocr.pdf"):
                        if candidate is None or len(f.name) < len(candidate.name):
                            candidate = f
        if not candidate:
            for f in manual_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    candidate = f
                    break
                else:
                    if candidate is None or len(f.name) < len(candidate.name):
                        candidate = f
        if candidate is None:
            sys.stderr.write(f"Error: No PDF found in directory: {manual_dir} or {build_dir}\n")
            sys.exit(1)
        pdf_path = candidate

    print(f"[STAGE 4] Analyzing '{pdf_path.name}' ({pdf_path})...")
    result = find_chapters(pdf_path)

    if args.pages:
        from typing import Set
        allowed_pages: Set[int] = set()
        for part in args.pages.split(","):
            part = part.strip()
            if "-" in part:
                s, e = part.split("-", 1)
                allowed_pages.update(range(int(s.strip()), int(e.strip()) + 1))
            elif part:
                allowed_pages.add(int(part))
        result["chapters"] = [ch for ch in result["chapters"] if ch["start_page"] in allowed_pages]
        result["chapter_count"] = len(result["chapters"])

    if args.output:
        out_file = args.output
    else:
        if args.pdf_path.parent.name == "build":
            out_file = args.pdf_path.parent / f"{args.pdf_path.stem}_chapters.json"
        else:
            build_dir = args.pdf_path.parent / "build"
            out_file = build_dir / f"{args.pdf_path.stem}_chapters.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    out_file.write_text(json.dumps(result, indent=2, ensure_ascii=False), encoding="utf-8")

    print(f"\n[RESULT] Detection method: {result['detection_method']}")
    print(f"[RESULT] Total pages: {result['total_pages']}")
    print(f"[RESULT] Found {len(result['chapters'])} chapter/section entries:")
    for ch in result["chapters"]:
        print(f"  - Page {ch['start_page']:>3}: {ch['title']}")

    print(f"\n[STAGE 4 SUCCESS] Chapter map saved to: {out_file}")


if __name__ == "__main__":
    main()
