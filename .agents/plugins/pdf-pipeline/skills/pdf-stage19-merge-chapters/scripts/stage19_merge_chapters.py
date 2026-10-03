"""Stage 19: Merge Final Chapters (Helper & Status Verification).

Inspects chapter drafts in build/02_detect_cont_chapters/ and verifies finalized,
publication-ready chapters in build/02_final_chapters/.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "path/to/manual_dir" --status
    python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "path/to/manual_dir" --pending
    python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "path/to/manual_dir" --chapter 1
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


def inspect_stage19_status(manual_dir: Path, target_chapter: Optional[int] = None) -> Dict[str, Any]:
    """Inspect status of Stage 19 chapter assembly."""
    draft_dir = manual_dir / "build" / "02_detect_cont_chapters"
    final_dir = manual_dir / "build" / "02_final_chapters"

    draft_files = sorted(list(draft_dir.glob("*.md"))) if draft_dir.exists() else []
    final_files = sorted(list(final_dir.glob("*.md"))) if final_dir.exists() else []

    final_map = {f.name: f for f in final_files if f.stat().st_size > 0}

    completed: List[Dict[str, Any]] = []
    pending: List[Dict[str, Any]] = []

    for df in draft_files:
        if target_chapter is not None:
            prefix = f"{target_chapter:02d}"
            if not (df.name.startswith(f"{prefix} - ") or df.name.startswith(f"{prefix}_") or df.name.startswith(f"{prefix}.")):
                continue

        final_f = final_map.get(df.name)
        if final_f:
            text = final_f.read_text(encoding="utf-8", errors="replace")
            remaining_markers = len(re.findall(r"^<continuation-marker>$", text, flags=re.MULTILINE))
            if remaining_markers == 0:
                completed.append({
                    "name": df.name,
                    "draft_size": df.stat().st_size,
                    "final_size": final_f.stat().st_size,
                })
                continue
            else:
                pending.append({
                    "name": df.name,
                    "reason": f"Contains {remaining_markers} unmerged <continuation-marker> tags",
                    "draft_file": str(df),
                    "final_file": str(final_f),
                })
                continue

        pending.append({
            "name": df.name,
            "reason": "Final chapter not yet generated",
            "draft_file": str(df),
            "final_file": str(final_dir / df.name),
        })

    return {
        "manual": manual_dir.name,
        "draft_count": len(draft_files),
        "completed_count": len(completed),
        "pending_count": len(pending),
        "completed": completed,
        "pending": pending,
    }


def verify_final_chapters(manual_dir: Path, chapter_targets: List[str]) -> bool:
    """Verify that specified final chapters exist, are non-empty, and contain zero <continuation-marker> tags."""
    final_dir = manual_dir / "build" / "02_final_chapters"
    if not final_dir.exists():
        sys.stderr.write(f"[STAGE 19 ERROR] Final chapters directory not found: {final_dir}\n")
        return False

    failures = []
    verified = []

    for target in chapter_targets:
        target_name = Path(target).name
        candidate = final_dir / target_name
        if not candidate.exists():
            matches = list(final_dir.glob(f"{target_name}*.md"))
            if matches:
                candidate = matches[0]
            else:
                failures.append(f"{target_name} (File not found on disk)")
                continue

        if candidate.stat().st_size == 0:
            failures.append(f"{candidate.name} (File is empty, 0 bytes)")
            continue

        text = candidate.read_text(encoding="utf-8", errors="replace")
        remaining_markers = len(re.findall(r"^<continuation-marker>$", text, flags=re.MULTILINE))
        if remaining_markers > 0:
            failures.append(f"{candidate.name} (Contains {remaining_markers} unmerged <continuation-marker> tags)")
            continue

        verified.append(candidate.name)

    if failures:
        sys.stderr.write(
            f"[STAGE 19 ERROR] Final chapter verification failed ({len(failures)} issue(s)):\n"
        )
        for f in failures:
            sys.stderr.write(f"  - {f}\n")
        return False

    print(f"[STAGE 19] Verified {len(verified)} final chapter(s) on disk (non-empty, zero <continuation-marker>): ALL_OK")
    for v in verified:
        print(f"  * {v}")
    return True


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 19: Merge Final Chapters (Helper & Status).")
    parser.add_argument("manual", nargs="?", default=".", help="Path to manual directory.")
    parser.add_argument("--chapter", type=int, default=None, help="Target specific chapter index (1-based).")
    parser.add_argument("--status", action="store_true", help="Print assembly status.")
    parser.add_argument("--pending", action="store_true", help="List all pending chapters requiring Agent inference.")
    parser.add_argument("--verify-chapters", nargs="+", help="Verify specified final chapter file(s) exist, are non-empty, and contain zero continuation markers.")

    args = parser.parse_args()
    manual_dir, _, _, _, _ = resolve_manual_paths(Path(args.manual))

    if args.verify_chapters:
        ok = verify_final_chapters(manual_dir, args.verify_chapters)
        if not ok:
            sys.exit(1)
        return

    res = inspect_stage19_status(manual_dir, target_chapter=args.chapter)

    if args.pending:
        print(f"Pending Stage 19 Chapters for '{res['manual']}':")
        for it in res["pending"]:
            print(f"  * {it['name']} ({it['reason']})")
        return

    print(f"Stage 19 Chapter Merge Status for '{res['manual']}':")
    print(f"  - Total Prepared Drafts: {res['draft_count']}")
    print(f"  - Completed Chapters:    {res['completed_count']}")
    print(f"  - Pending Chapters:      {res['pending_count']}")

    if res["completed"]:
        print("\nCompleted Chapters in build/02_final_chapters/:")
        for it in res["completed"]:
            print(f"  [DONE] {it['name']} ({it['final_size'] // 1024} KB)")

    if res["pending"]:
        print("\nPending Chapters (Awaiting Agent Multimodal Assembly):")
        for it in res["pending"]:
            print(f"  [PENDING] {it['name']} - {it['reason']}")


if __name__ == "__main__":
    main()
