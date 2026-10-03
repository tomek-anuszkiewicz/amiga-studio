"""Stage 17: Proofread Page (Helper & Status Verification).

Inspects embedded per-page Markdown files (build/01_page_layout/page_XXXX-embed.md)
and monitors finalized proofread files (build/01_page_layout/page_XXXX-proofread.md).
Provides sliding window context (Page N-1 read-only) formatting for Agent inference.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py "path/to/manual_dir" --status
    python .agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py "path/to/manual_dir" --pending
    python .agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py "path/to/manual_dir" --show-prompt 14
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


TARGET_MANUALS: List[str] = [
    "68000 Programmer's Reference Manual",
    "68000 User's Manual",
    "A500 A2000 Technical Reference Manual",
    "Hardware Reference Manual",
    "Test Book example-4567",
]


def parse_page_range(range_str: str) -> Set[int]:
    """Parse range string like '1-10', '1..10', '5', '1,3,5-7', 'page_0001,page_0002'."""
    pages: Set[int] = set()
    for part in range_str.replace(";", ",").split(","):
        part = part.strip()
        if not part:
            continue
        part = re.sub(r"^page_0*", "", part, flags=re.IGNORECASE)
        if ".." in part:
            s_str, e_str = part.split("..", 1)
            pages.update(range(int(s_str.strip()), int(e_str.strip()) + 1))
        elif "-" in part:
            s_str, e_str = part.split("-", 1)
            pages.update(range(int(s_str.strip()), int(e_str.strip()) + 1))
        else:
            pages.add(int(part))
    return pages


def verify_proofread_pages(manual_dir: Path, layout_dir: Path, range_str: str) -> bool:
    """Verify that specified proofread pages exist and are non-empty on disk."""
    target_pages = sorted(list(parse_page_range(range_str)))
    if not target_pages:
        sys.stderr.write("[STAGE 17 ERROR] No valid target pages parsed from input.\n")
        return False

    missing_or_empty = []
    for p in target_pages:
        p_file = layout_dir / f"page_{p:04d}-proofread.md"
        if not p_file.exists() or p_file.stat().st_size == 0:
            missing_or_empty.append(f"page_{p:04d}-proofread.md")

    if missing_or_empty:
        sys.stderr.write(
            f"[STAGE 17 ERROR] Missing or empty proofread file(s) on disk ({len(missing_or_empty)} of {len(target_pages)}):\n"
            f"  {missing_or_empty}\n"
        )
        return False

    print(f"[STAGE 17] Verified {len(target_pages)} proofread page(s) on disk for '{manual_dir.name}': ALL_OK")
    return True


def resolve_manual_paths(target_input: str) -> Tuple[Path, Optional[Path], Path, Optional[Path], Optional[Path]]:
    """Resolve directory, pdf, layout directory, queue path, and chapters path."""
    target = Path(target_input)
    if not target.is_absolute():
        target = (Path.cwd() / target).resolve()

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
        manual_dir = target
        pdf_path = None

    layout_dir = manual_dir / "build" / "01_page_layout"
    stem = pdf_path.stem if pdf_path else manual_dir.name
    build_dir = manual_dir / "build"

    candidate_q = build_dir / f"{stem}_queue.json"
    queue_path = candidate_q if candidate_q.exists() else None
    if not queue_path and build_dir.is_dir():
        for q in build_dir.glob("*_queue.json"):
            if not q.name.endswith("_assets_queue.json"):
                queue_path = q
                break

    candidate_ch = build_dir / f"{stem}_chapters.json"
    chapters_path = candidate_ch if candidate_ch.exists() else None
    if not chapters_path and build_dir.is_dir():
        for c in build_dir.glob("*_chapters.json"):
            chapters_path = c
            break

    return manual_dir, pdf_path, layout_dir, queue_path, chapters_path


def get_stage17_status(manual_dir: Path, layout_dir: Path, queue_path: Optional[Path]) -> Dict[str, Any]:
    """Inspect disk state for Stage 17 proofreading."""
    embed_files = sorted(list(layout_dir.glob("page_*-embed.md"))) if layout_dir.exists() else []
    proofread_files = sorted(list(layout_dir.glob("page_*-proofread.md"))) if layout_dir.exists() else []

    embed_pages = set()
    for f in embed_files:
        m = re.match(r"^page_(\d+)-embed\.md$", f.name)
        if m:
            embed_pages.add(int(m.group(1)))

    proofread_pages = set()
    for f in proofread_files:
        m = re.match(r"^page_(\d+)-proofread\.md$", f.name)
        if m and f.stat().st_size > 0:
            proofread_pages.add(int(m.group(1)))

    # Discover expected pages
    total_expected = len(embed_pages)
    if queue_path and queue_path.exists():
        try:
            with open(queue_path, "r", encoding="utf-8") as qf:
                q_data = json.load(qf)
                total_expected = max(total_expected, q_data.get("total_pages", 0))
        except Exception:
            pass

    pending_pages = sorted(list(embed_pages - proofread_pages))

    return {
        "manual": manual_dir.name,
        "layout_dir": str(layout_dir),
        "total_embedded": len(embed_pages),
        "total_proofread": len(proofread_pages),
        "total_expected": total_expected,
        "pending_count": len(pending_pages),
        "pending_pages": pending_pages,
        "ready": (len(embed_pages) > 0 and len(pending_pages) == 0),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 17: Proofread Page (Helper & Status Verification).")
    parser.add_argument("manual", nargs="?", default=".", help="Path to manual directory.")
    parser.add_argument("--status", action="store_true", help="Print proofreading status.")
    parser.add_argument("--pending", action="store_true", help="List all pending pages awaiting proofreading.")
    parser.add_argument("--pages", "-p", help="Page range filter (e.g. '1-10', '14')")
    parser.add_argument("--verify-pages", help="Verify specified pages exist on disk and are non-empty (e.g. '1-30', '1..10', '1,2,3', 'page_0001,page_0002')")

    args = parser.parse_args()

    manual_input = args.manual
    if args.verify_pages:
        manual_dir, _, layout_dir, _, _ = resolve_manual_paths(manual_input)
        ok = verify_proofread_pages(manual_dir, layout_dir, args.verify_pages)
        if not ok:
            sys.exit(1)
        return
    if manual_input in (".", "all") and not Path(manual_input).is_dir():
        targets = TARGET_MANUALS
    else:
        targets = [manual_input]

    for tgt in targets:
        manual_dir, pdf_path, layout_dir, queue_path, _ = resolve_manual_paths(tgt)
        if not manual_dir.exists():
            continue

        status = get_stage17_status(manual_dir, layout_dir, queue_path)

        if args.pending:
            print(f"Pending Stage 17 Pages for '{status['manual']}':")
            if status["pending_pages"]:
                print(f"  * {len(status['pending_pages'])} page(s) pending: {status['pending_pages'][:20]}{'...' if len(status['pending_pages']) > 20 else ''}")
            else:
                print("  * None! All embedded pages proofread.")
            continue

        print(f"Stage 17 Proofread Page Status for '{status['manual']}':")
        print(f"  - Embedded Pages (Stage 16):   {status['total_embedded']}")
        print(f"  - Proofread Pages (Stage 17):  {status['total_proofread']}")
        print(f"  - Pending Pages:               {status['pending_count']}")
        print(f"  - Ready for Stage 18 Bundling: {'YES' if status['ready'] else 'NO'}")
        if status["pending_pages"] and len(status["pending_pages"]) <= 10:
            print(f"    Pending: {status['pending_pages']}")
        elif status["pending_pages"]:
            print(f"    Pending (first 10): {status['pending_pages'][:10]} ...")


if __name__ == "__main__":
    main()
