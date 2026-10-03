"""Stage 13: Reduce HTML Tables to Markdown (Deterministic Gatekeeper Filter).

This script acts as the Step 1 deterministic gatekeeper for Stage 13:
1. Analyzes HTML tables (assets/<asset_id>_html.md) generated in Stage 12.
2. Identifies tables that MUST remain HTML:
   - Contains rowspan > 1 or colspan > 1 across any th or td.
   - Contains multiline or block tags (<br>, <p>, <div>, <ul>, <ol>, nested <table>).
   - Contains ragged / non-uniform column counts across rows.
   - Contains layout CSS positioning (float, flex, display).
   -> Sets reduced_to_markdown: False with explicit reduction_reason.
3. Identifies tables eligible for clean Markdown reduction:
   - Flat, uniform rectangular grid with single-line cells.
   -> Sets reduced_to_markdown: None (pending Agent multimodal transcription)
      or preserves True if assets/<asset_id>_reduced.md is already synthesized.
4. Step 2 (In-Session Multimodal Agent) then transcribes the pending candidate visual
   crops into pure GFM Markdown with native LaTeX math per prompt.md.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py "path/to/manual_dir"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py "path/to/manual_dir" --dry-run
    python .agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py "path/to/manual_dir" --status
    python .agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py "path/to/manual_dir" --pages 186
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

try:
    from bs4 import BeautifulSoup
except ImportError:
    BeautifulSoup = None

# Safe console output on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


def to_relative_posix(target_path: Path, base_dir: Path) -> str:
    """Format path relative to base_dir using forward slashes."""
    t = target_path.resolve()
    b = base_dir.resolve()
    try:
        return str(t.relative_to(b)).replace("\\", "/")
    except ValueError:
        try:
            return str(t.relative_to(Path.cwd().resolve())).replace("\\", "/")
        except ValueError:
            return str(target_path.name)


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


def resolve_manual_paths(target: Path) -> Tuple[Path, Optional[Path], Path, Path]:
    """Resolve directory, pdf, layout directory, and queue path."""
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

    return manual_dir, pdf_path, layout_dir, queue_path


def load_queue(queue_path: Path) -> Dict[str, Any]:
    """Load and parse the assets queue JSON."""
    with open(queue_path, "r", encoding="utf-8") as f:
        return json.load(f)


def save_queue(queue_path: Path, data: Dict[str, Any]) -> None:
    """Save the updated queue JSON with clean formatting."""
    temp_path = queue_path.with_suffix(".tmp")
    with open(temp_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    temp_path.replace(queue_path)


def analyze_html_table(html_content: str) -> Tuple[bool, str]:
    """Analyze HTML content to determine if it is eligible for reduction to GFM Markdown.

    Returns:
        (can_reduce, reason)
    """
    if BeautifulSoup is None:
        return False, "BeautifulSoup not available for HTML parsing"

    soup = BeautifulSoup(html_content, "html.parser")
    tables = soup.find_all("table")
    if not tables:
        return False, "No <table> element found in file"

    if len(tables) > 1:
        return False, f"Multiple <table> elements found ({len(tables)})"

    table = tables[0]

    # Check for multiline / block elements inside table cells
    block_tags = ["p", "ul", "ol", "blockquote", "table", "br", "div"]
    found_blocks = table.find_all(block_tags)
    if found_blocks:
        tag_names = ", ".join(sorted(set(t.name for t in found_blocks)))
        return False, f"Contains block or multiline elements ({tag_names})"

    rows = table.find_all("tr")
    if not rows:
        return False, "Table contains no <tr> rows"

    expected_cols: Optional[int] = None

    for r_idx, row in enumerate(rows):
        cells = row.find_all(["th", "td"])
        row_cols = 0

        for cell in cells:
            colspan_str = cell.get("colspan", "1")
            rowspan_str = cell.get("rowspan", "1")

            try:
                colspan = int(colspan_str)
            except ValueError:
                colspan = 1

            try:
                rowspan = int(rowspan_str)
            except ValueError:
                rowspan = 1

            if colspan > 1:
                return False, f"Contains colspan={colspan} in row {r_idx + 1}"
            if rowspan > 1:
                return False, f"Contains rowspan={rowspan} in row {r_idx + 1}"

            # Check for layout CSS styles
            style = cell.get("style", "").lower()
            if any(prop in style for prop in ["float", "flex", "display", "position"]):
                return False, f"Contains layout CSS styles in row {r_idx + 1}"

            row_cols += 1

        if expected_cols is None:
            expected_cols = row_cols
        elif row_cols != expected_cols:
            return False, f"Non-uniform column count: expected {expected_cols}, got {row_cols} in row {r_idx + 1}"

    if expected_cols is None or expected_cols < 1 or len(rows) < 2:
        return False, "Insufficient rows or columns to construct GFM table"

    return True, "Eligible rectangular table: uniform grid with single-line cells"


def run_reduction_filter(
    queue_path: Path,
    manual_dir: Path,
    dry_run: bool = False,
    pages_filter: Optional[Set[int]] = None,
) -> Tuple[int, int, int, List[Dict[str, Any]]]:
    """Execute Stage 14 deterministic gatekeeper filter across all table_html assets."""
    data = load_queue(queue_path)
    items = data.get("queue", [])
    layout_dir = manual_dir / "build" / "01_page_layout"
    assets_dir = layout_dir / "assets"

    reduced_count = 0
    kept_html_count = 0
    pending_agent_count = 0
    pending_candidates: List[Dict[str, Any]] = []

    for it in items:
        if pages_filter is not None:
            p_num = it.get("page_number") or it.get("page")
            if p_num is not None and p_num not in pages_filter:
                continue
        if it.get("detected_type") != "table_html":
            continue

        asset_id = it.get("asset_id")
        p_num = it.get("page_number") or it.get("page")
        gen_md_rel = it.get("generated_markdown_html") or it.get("generated_markdown")
        if not gen_md_rel:
            continue

        full_md_path = manual_dir / gen_md_rel
        if not full_md_path.exists() or full_md_path.stat().st_size == 0:
            continue

        # Check if already reduced to Markdown and reduced file exists
        reduced_file = assets_dir / f"{asset_id}_reduced.md"
        if it.get("reduced_to_markdown") is True and reduced_file.exists() and reduced_file.stat().st_size > 0:
            reduced_count += 1
            continue

        html_content = full_md_path.read_text(encoding="utf-8", errors="replace")
        can_reduce, reason = analyze_html_table(html_content)

        if not can_reduce:
            # Disqualified: MUST remain HTML
            kept_html_count += 1
            if not dry_run:
                it["reduced_to_markdown"] = False
                it["reduction_reason"] = reason
                it["generated_markdown"] = gen_md_rel
                it["generated_markdown_html"] = gen_md_rel
        else:
            # Eligible candidate for Agent multimodal transcription
            pending_agent_count += 1
            pending_candidates.append({
                "asset_id": asset_id,
                "page": p_num,
                "crop": f"assets/{asset_id}_clip_final.png",
                "output": f"assets/{asset_id}_reduced.md",
            })
            if not dry_run:
                it["reduced_to_markdown"] = None
                it["reduction_reason"] = "Eligible candidate: awaiting Agent multimodal transcription"
                it["generated_markdown"] = gen_md_rel
                it["generated_markdown_html"] = gen_md_rel

    if not dry_run:
        # Refresh reduction stats in queue
        reduc_stats = data.get("reduction_stats", {})
        reduc_stats["total_html_tables"] = reduced_count + kept_html_count + pending_agent_count
        reduc_stats["reduced_to_markdown"] = reduced_count
        reduc_stats["kept_as_html"] = kept_html_count
        reduc_stats["pending_agent_reduction"] = pending_agent_count
        data["reduction_stats"] = reduc_stats
        save_queue(queue_path, data)

    return reduced_count, kept_html_count, pending_agent_count, pending_candidates


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 13: Reduce HTML Tables to Markdown (Gatekeeper Filter).")
    parser.add_argument("manual", nargs="?", default=".", help="Path to manual directory.")
    parser.add_argument("--dry-run", action="store_true", help="Analyze tables without modifying queue.")
    parser.add_argument("--status", action="store_true", help="Print reduction status summary.")
    parser.add_argument("--pages", "-p", help="Page range filter (e.g. '1-10', '186')")

    args = parser.parse_args()
    manual_dir, pdf_path, layout_dir, queue_path = resolve_manual_paths(Path(args.manual))
    pages_filter = parse_page_range(args.pages) if args.pages else None

    if args.status:
        data = load_queue(queue_path)
        stats = data.get("reduction_stats", {})
        print(f"Stage 13 Reduction Status for '{manual_dir.name}':")
        print(f"  - Total HTML Candidates:     {stats.get('total_html_tables', 0)}")
        print(f"  - Kept as HTML (Complex):    {stats.get('kept_as_html', 0)}")
        print(f"  - Reduced to GFM Markdown:   {stats.get('reduced_to_markdown', 0)}")
        print(f"  - Pending Agent Reduction:   {stats.get('pending_agent_reduction', 0)}")
        return

    reduced, kept, pending, candidates = run_reduction_filter(
        queue_path, manual_dir, dry_run=args.dry_run, pages_filter=pages_filter
    )
    prefix = "[DRY-RUN] " if args.dry_run else ""
    print(f"{prefix}Stage 13 Gatekeeper Results for '{manual_dir.name}':")
    print(f"  - Kept as HTML (Complex geometry / multiline): {kept}")
    print(f"  - Already Reduced to GFM Markdown:             {reduced}")
    print(f"  - Pending Agent Multimodal Reduction:          {pending}")

    if candidates:
        print(f"\nEligible candidates ready for Agent multimodal transcription (first 10):")
        for cand in candidates[:10]:
            print(f"  * Page {cand['page']:4d} | {cand['asset_id']:30s} -> {cand['crop']}")
        if len(candidates) > 10:
            print(f"  ... and {len(candidates) - 10} more.")


if __name__ == "__main__":
    main()
