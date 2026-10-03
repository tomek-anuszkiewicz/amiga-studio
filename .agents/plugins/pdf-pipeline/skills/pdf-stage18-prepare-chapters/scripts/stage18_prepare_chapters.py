"""Stage 18: Prepare Chapters.

Deterministically bundles per-page Markdown files (build/01_page_layout/page_XXXX-proofread.md,
falling back to page_XXXX-embed.md) into chapter drafts in build/02_detect_cont_chapters/ with
<continuation-marker> on a new line between consecutive pages.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage18-prepare-chapters/scripts/stage18_prepare_chapters.py "path/to/manual_dir"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage18-prepare-chapters/scripts/stage18_prepare_chapters.py "path/to/manual_dir" --chapter 1
    python .agents/plugins/pdf-pipeline/skills/pdf-stage18-prepare-chapters/scripts/stage18_prepare_chapters.py "path/to/manual_dir" --status
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

# Safe console output on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


def parse_page_range(range_str: str) -> Set[int]:
    """Parse range string like '1-10', '5', '1,3,5-7'."""
    pages: Set[int] = set()
    for part in range_str.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            s_str, e_str = part.split("-", 1)
            pages.update(range(int(s_str.strip()), int(e_str.strip()) + 1))
        else:
            pages.add(int(part))
    return pages


def sanitize_filename(name: str) -> str:
    """Sanitize filename by stripping illegal filesystem characters while preserving spaces and hyphens."""
    clean = re.sub(r'[<>:"/\\|?*]', "", name)
    clean = re.sub(r"\s+", " ", clean).strip()
    return clean


def get_chapter_filename(ch: Dict[str, Any], ch_num: int) -> str:
    """Determine clean chapter filename honoring 'filename', 'clean_title', or fallback."""
    if ch.get("filename"):
        raw_name = ch["filename"].strip()
        if not raw_name.lower().endswith(".md"):
            raw_name = f"{raw_name}.md"
        return sanitize_filename(raw_name)

    clean_title = ch.get("clean_title") or ch.get("title", f"Chapter {ch_num}")
    return sanitize_filename(f"{ch_num:02d} - {clean_title}.md")


def slugify(text: str) -> str:
    """Generate clean filesystem slug from chapter title."""
    s = re.sub(r"[^\w\s-]", "", text).strip().lower()
    s = re.sub(r"[-\s]+", "_", s)
    return s[:60] if s else "chapter"


def resolve_manual_paths(target: Path) -> Tuple[Path, Optional[Path], Path, Path, Path]:
    """Resolve directory, pdf, layout directory, queue path, and chapters path."""
    target = target.resolve()
    if target.is_file() and target.suffix.lower() == ".pdf":
        manual_dir = target.parent.parent if target.parent.name == "build" else target.parent
        pdf_path = target
    elif target.is_dir():
        manual_dir = target.parent if target.name == "build" else target
        pdf_files = []
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            pdf_files.extend(list(build_dir.glob("*.pdf")))
        pdf_files.extend(list(manual_dir.glob("*.pdf")))
        ocr_candidates = [p for p in pdf_files if "_ocr" in p.name]
        pdf_path = ocr_candidates[0] if ocr_candidates else (pdf_files[0] if pdf_files else None)
    else:
        manual_dir = target.parent.parent if target.parent.name == "build" else target.parent
        pdf_path = None

    layout_dir = manual_dir / "build" / "01_page_layout"
    stem = pdf_path.stem if pdf_path else manual_dir.name
    build_dir = manual_dir / "build"

    candidate_q = build_dir / f"{stem}_assets_queue.json"
    if candidate_q.exists():
        queue_path = candidate_q
    elif (manual_dir / f"{stem}_assets_queue.json").exists():
        queue_path = manual_dir / f"{stem}_assets_queue.json"
    else:
        found_q = None
        if build_dir.is_dir():
            for q in build_dir.glob("*_assets_queue.json"):
                found_q = q
                break
        if not found_q:
            for q in manual_dir.glob("*_assets_queue.json"):
                found_q = q
                break
        queue_path = found_q or candidate_q

    candidate_ch = build_dir / f"{stem}_chapters.json"
    if candidate_ch.exists():
        chapters_path = candidate_ch
    elif (manual_dir / f"{stem}_chapters.json").exists():
        chapters_path = manual_dir / f"{stem}_chapters.json"
    else:
        found_ch = None
        if build_dir.is_dir():
            for c in build_dir.glob("*_chapters.json"):
                found_ch = c
                break
        if not found_ch:
            for c in manual_dir.glob("*_chapters.json"):
                found_ch = c
                break
        chapters_path = found_ch or candidate_ch

    return manual_dir, pdf_path, layout_dir, queue_path, chapters_path


def load_json(path: Path) -> Dict[str, Any]:
    """Load JSON from file."""
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def prepare_chapters(
    manual_dir: Path,
    layout_dir: Path,
    chapters_path: Path,
    target_chapter: Optional[int] = None,
    pages_filter: Optional[Set[int]] = None,
    force: bool = False,
) -> List[Path]:
    """Deterministically assemble chapter drafts into build/02_detect_cont_chapters/ with <continuation-marker>."""
    if not chapters_path.exists():
        sys.stderr.write(f"Error: Chapters map not found: {chapters_path}\n")
        sys.exit(1)

    chapters_data = load_json(chapters_path)
    ch_list = chapters_data.get("chapters", [])
    total_pages = chapters_data.get("total_pages", 1000)

    out_dir = manual_dir / "build" / "02_detect_cont_chapters"
    out_dir.mkdir(parents=True, exist_ok=True)

    generated_files: List[Path] = []

    for idx, ch in enumerate(ch_list):
        ch_num = idx + 1
        if target_chapter is not None and ch_num != target_chapter:
            continue

        title = ch.get("title", f"Chapter {ch_num}")
        start_page = ch.get("start_page", 1)
        next_start = ch_list[idx + 1].get("start_page", total_pages + 1) if (idx + 1) < len(ch_list) else (total_pages + 1)
        end_page = next_start - 1

        if pages_filter is not None:
            ch_pages = set(range(start_page, end_page + 1))
            if not ch_pages.intersection(pages_filter):
                continue

        ch_filename = get_chapter_filename(ch, ch_num)
        out_file = out_dir / ch_filename

        if out_file.exists() and not force:
            generated_files.append(out_file)
            continue

        page_contents: List[str] = []

        for p in range(start_page, end_page + 1):
            cand_files = [
                layout_dir / f"page_{p:04d}-proofread.md",
                layout_dir / f"page_{p:04d}-embed.md",
                layout_dir / f"page_{p:04d}-images-clip_final.md",
                layout_dir / f"page_{p:04d}-images.md",
                layout_dir / f"page_{p:04d}.md",
            ]

            content = None
            for cand in cand_files:
                if cand.exists() and cand.stat().st_size > 0:
                    content = cand.read_text(encoding="utf-8", errors="replace").strip()
                    break

            if not content:
                continue

            # Strip any legacy top-level continuation tag if present
            content = re.sub(r"^<(continuation-(?:paragraph|table|code|index))>\s*\n?", "", content, flags=re.IGNORECASE)

            # Adjust relative asset paths from 02_detect_cont_chapters to 01_page_layout/assets
            content = re.sub(r"\]\((?:\.\./01_page_layout/)?assets/", "](../01_page_layout/assets/", content)
            content = re.sub(r'src=["\'](?:\.\./01_page_layout/)?assets/', 'src="../01_page_layout/assets/', content)

            content = content.strip()
            if content:
                page_contents.append(content)

        if page_contents:
            joined_content = "\n\n<continuation-marker>\n\n".join(page_contents)
            out_file.write_text(joined_content + "\n", encoding="utf-8")
            generated_files.append(out_file)

    return generated_files


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 18: Prepare Chapters.")
    parser.add_argument("manual", nargs="?", default=".", help="Path to manual directory.")
    parser.add_argument("--chapter", type=int, default=None, help="Target specific chapter index (1-based).")
    parser.add_argument("--status", action="store_true", help="Print prepared chapter drafts status.")
    parser.add_argument("--pages", "-p", help="Page range filter to assemble overlapping chapters (e.g. '1-10', '73')")
    parser.add_argument("--force", "-f", action="store_true", help="Overwrite existing draft files.")

    args = parser.parse_args()
    manual_dir, pdf_path, layout_dir, queue_path, chapters_path = resolve_manual_paths(Path(args.manual))
    pages_filter = parse_page_range(args.pages) if args.pages else None

    if args.status:
        out_dir = manual_dir / "build" / "02_detect_cont_chapters"
        drafts = list(out_dir.glob("*.md")) if out_dir.exists() else []
        expected_count = 0
        if chapters_path.exists():
            try:
                ch_data = load_json(chapters_path)
                expected_count = len(ch_data.get("chapters", []))
            except Exception:
                pass

        print(f"Stage 18 Prepare Chapters Status for '{manual_dir.name}':")
        print(f"  - Output Directory: build/02_detect_cont_chapters/")
        print(f"  - Prepared Chapter Drafts: {len(drafts)} of {expected_count} expected")
        for f in sorted(drafts):
            text = f.read_text(encoding="utf-8", errors="replace")
            marker_count = len(re.findall(r"^<continuation-marker>$", text, flags=re.MULTILINE))
            print(f"    * {f.name} ({f.stat().st_size // 1024} KB, {marker_count} markers)")
        return

    generated = prepare_chapters(
        manual_dir,
        layout_dir,
        chapters_path,
        target_chapter=args.chapter,
        pages_filter=pages_filter,
        force=args.force,
    )

    print(f"Stage 18 Prepare Chapters for '{manual_dir.name}':")
    print(f"  - Generated {len(generated)} chapter draft(s) in build/02_detect_cont_chapters/")
    for f in generated:
        print(f"    * {f.name}")


if __name__ == "__main__":
    main()
