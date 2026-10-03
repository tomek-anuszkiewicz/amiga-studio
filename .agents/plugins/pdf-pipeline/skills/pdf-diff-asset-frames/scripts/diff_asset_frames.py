"""Asset Frame Review Diff & Selective Staging Tool.

Compares modified asset review frames (page_XXXX_asset_frame.png) between
git HEAD and the working tree. Identifies frames where changes are purely cosmetic
or negligible margin shifts (same semantic type badge, same asset count,
and identical trimmed ink content), and stages them (`git add`) into the git index
while keeping significant changes (semantic tag changes, asset additions/deletions,
or actual content differences) unstaged for manual user inspection.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-diff-asset-frames/scripts/diff_asset_frames.py
    python .agents/plugins/pdf-pipeline/skills/pdf-diff-asset-frames/scripts/diff_asset_frames.py "path/to/manual"
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

try:
    from PIL import Image, ImageChops
    import numpy as np
except ImportError:
    Image = None
    ImageChops = None
    np = None

# Safe console encoding on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


def run_git_command(args: List[str], cwd: Path) -> str:
    """Run a git command and return stripped stdout."""
    res = subprocess.run(
        ["git"] + args,
        cwd=str(cwd),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=True,
    )
    return res.stdout.strip()


def get_modified_asset_frames(repo_root: Path, target_manual: Optional[Path] = None) -> List[str]:
    """Retrieve list of modified asset_frame.png paths relative to repo root."""
    output = run_git_command(["diff", "--name-only", "HEAD", "--", "*asset_frame.png"], repo_root)
    files = [f.strip().replace("\\", "/") for f in output.splitlines() if f.strip()]
    if target_manual:
        rel_target = str(target_manual.resolve().relative_to(repo_root.resolve())).replace("\\", "/")
        files = [f for f in files if f.startswith(rel_target + "/") or f == rel_target]
    return files


def load_queue_file(queue_path: Path) -> List[Dict[str, Any]]:
    """Load asset queue items from file."""
    if not queue_path.is_file():
        return []
    with open(queue_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    if isinstance(data, dict):
        return data.get("queue", [])
    elif isinstance(data, list):
        return data
    return []


def load_git_head_queue(repo_root: Path, rel_queue_path: str) -> List[Dict[str, Any]]:
    """Load asset queue items from git HEAD."""
    try:
        content = run_git_command(["show", f"HEAD:{rel_queue_path}"], repo_root)
        data = json.loads(content)
        if isinstance(data, dict):
            return data.get("queue", [])
        elif isinstance(data, list):
            return data
    except Exception:
        return []
    return []


def compare_asset_content(
    base_img: Image.Image,
    hb: List[int],
    wb: List[int],
    tolerance_pixels: int = 15,
) -> Tuple[bool, str]:
    """Compare the ink content of two bounding boxes on the same base page.

    Trims white paper margins down to ink boundaries and verifies that the
    trimmed printed content matches pixel-for-pixel (within small anti-aliasing noise).
    """
    w_px, h_px = base_img.size

    # Full page special case
    if hb == [0, 0, 1000, 1000] or wb == [0, 0, 1000, 1000]:
        if hb == wb:
            return True, "identical full-page"
        return False, "full-page bounds mismatch"

    x0_h = max(0, min(w_px - 1, int(round(hb[1] / 1000.0 * w_px))))
    y0_h = max(0, min(h_px - 1, int(round(hb[0] / 1000.0 * h_px))))
    x1_h = max(1, min(w_px, int(round(hb[3] / 1000.0 * w_px))))
    y1_h = max(1, min(h_px, int(round(hb[2] / 1000.0 * h_px))))

    x0_w = max(0, min(w_px - 1, int(round(wb[1] / 1000.0 * w_px))))
    y0_w = max(0, min(h_px - 1, int(round(wb[0] / 1000.0 * h_px))))
    x1_w = max(1, min(w_px, int(round(wb[3] / 1000.0 * w_px))))
    y1_w = max(1, min(h_px, int(round(wb[2] / 1000.0 * h_px))))

    if x1_h <= x0_h or y1_h <= y0_h or x1_w <= x0_w or y1_w <= y0_w:
        return False, "zero-area box"

    crop_h = base_img.crop((x0_h, y0_h, x1_h, y1_h))
    crop_w = base_img.crop((x0_w, y0_w, x1_w, y1_w))

    arr_h = np.array(crop_h)
    arr_w = np.array(crop_w)

    # Invert and mask background (ink < 200)
    ink_h_arr = np.where(arr_h < 200, 255 - arr_h, 0).astype(np.uint8)
    ink_w_arr = np.where(arr_w < 200, 255 - arr_w, 0).astype(np.uint8)

    ink_h = Image.fromarray(ink_h_arr)
    ink_w = Image.fromarray(ink_w_arr)

    bbox_h = ink_h.getbbox()
    bbox_w = ink_w.getbbox()

    if bbox_h is None and bbox_w is None:
        return True, "both empty"
    if bbox_h is None or bbox_w is None:
        return False, "ink missing in one version"

    trim_h = ink_h.crop(bbox_h)
    trim_w = ink_w.crop(bbox_w)

    if trim_h.size != trim_w.size:
        return False, f"trimmed size mismatch: {trim_h.size} vs {trim_w.size}"

    diff = ImageChops.difference(trim_h, trim_w)
    diff_arr = np.array(diff)
    differing_pixels = int(np.sum(diff_arr > 20))

    if differing_pixels > tolerance_pixels:
        return False, f"ink diff ({differing_pixels} px > {tolerance_pixels})"

    return True, "identical ink"


def evaluate_asset_frame(
    rel_frame_path: str,
    repo_root: Path,
    head_map: Dict[int, List[Dict[str, Any]]],
    work_map: Dict[int, List[Dict[str, Any]]],
    manual_dir: Path,
    tolerance_pixels: int = 15,
) -> Tuple[bool, str]:
    """Evaluate whether an asset_frame.png difference is insignificant.

    Returns:
        (is_insignificant, reason_description)
    """
    p_name = Path(rel_frame_path).name
    try:
        p_num = int(p_name.split("_")[1])
    except (IndexError, ValueError):
        return False, "invalid filename"

    h_items = head_map.get(p_num, [])
    w_items = work_map.get(p_num, [])

    # 1. Asset count check
    if len(h_items) != len(w_items):
        return False, f"asset count changed ({len(h_items)} -> {len(w_items)})"

    # Pages with 0 visual assets: verify clean page frame
    if len(h_items) == 0:
        return True, "zero assets on page"

    # 2. Semantic type badge check (MD, HTML, IMG)
    h_types = [str(x.get("detected_type", "")).strip().lower() for x in h_items]
    w_types = [str(x.get("detected_type", "")).strip().lower() for x in w_items]
    if h_types != w_types:
        return False, f"semantic tag changed ({h_types} -> {w_types})"

    # 3. Content comparison on underlying page
    page_img_path = manual_dir / "build" / "01_page_layout" / f"page_{p_num:04d}.png"
    if not page_img_path.is_file():
        return False, f"missing base page {page_img_path.name}"

    work_frame_path = repo_root / rel_frame_path
    if len(w_items) > 0 and work_frame_path.is_file():
        try:
            if work_frame_path.stat().st_size == page_img_path.stat().st_size:
                return False, "frame is un-annotated copy of base page"
        except OSError:
            pass

    try:
        base_img = Image.open(page_img_path).convert("L")
    except Exception as e:
        return False, f"cannot open base page: {e}"

    for i, (h_it, w_it) in enumerate(zip(h_items, w_items)):
        hb = h_it.get("active_box")
        wb = w_it.get("active_box")
        if not hb or not wb or len(hb) != 4 or len(wb) != 4:
            return False, f"asset {i}: invalid bounding box"

        if hb == wb:
            continue

        is_same_content, reason = compare_asset_content(
            base_img, hb, wb, tolerance_pixels=tolerance_pixels
        )
        if not is_same_content:
            return False, f"asset {i} {reason}"

    return True, "same tag and identical ink content"


def process_manual(
    manual_name: str,
    files: List[str],
    repo_root: Path,
    tolerance_pixels: int = 15,
) -> Dict[str, Any]:
    """Process all modified asset frames for a single manual."""
    manual_dir = repo_root / manual_name
    q_candidates = list((manual_dir / "build").glob("*_assets_queue.json")) or list(manual_dir.glob("*_assets_queue.json"))
    if not q_candidates:
        return {
            "manual": manual_name,
            "total": len(files),
            "staged": [],
            "unstaged": [(f, "no queue file found") for f in files],
        }

    q_file = q_candidates[0]
    rel_q_path = str(q_file.relative_to(repo_root)).replace("\\", "/")

    work_queue = load_queue_file(q_file)
    head_queue = load_git_head_queue(repo_root, rel_q_path)

    head_map: Dict[int, List[Dict[str, Any]]] = {}
    for item in head_queue:
        head_map.setdefault(item.get("page_number", 0), []).append(item)

    work_map: Dict[int, List[Dict[str, Any]]] = {}
    for item in work_queue:
        work_map.setdefault(item.get("page_number", 0), []).append(item)

    staged: List[str] = []
    unstaged: List[Tuple[str, str]] = []

    for rel_frame in files:
        is_insig, reason = evaluate_asset_frame(
            rel_frame, repo_root, head_map, work_map, manual_dir, tolerance_pixels
        )
        if is_insig:
            staged.append(rel_frame)
        else:
            unstaged.append((rel_frame, reason))

    if staged:
        # Batch git add in chunks
        chunk_size = 50
        for i in range(0, len(staged), chunk_size):
            chunk = staged[i : i + chunk_size]
            run_git_command(["add"] + chunk, repo_root)

    return {
        "manual": manual_name,
        "total": len(files),
        "staged": staged,
        "unstaged": unstaged,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Stage insignificant asset frame modifications while preserving significant changes for review."
    )
    parser.add_argument(
        "manual_dir",
        nargs="?",
        default=None,
        help="Optional path to target manual directory. Defaults to all manuals in repo.",
    )
    parser.add_argument(
        "--tolerance",
        type=int,
        default=15,
        help="Pixel tolerance threshold for anti-aliasing noise during ink difference (default: 15).",
    )
    args = parser.parse_args()

    repo_root = Path.cwd().resolve()
    target_manual = Path(args.manual_dir).resolve() if args.manual_dir else None

    modified_files = get_modified_asset_frames(repo_root, target_manual)
    if not modified_files:
        print("No modified asset_frame.png files detected in git diff HEAD.")
        sys.exit(0)

    # Group by manual
    by_manual: Dict[str, List[str]] = {}
    for f in modified_files:
        man = f.split("/")[0]
        by_manual.setdefault(man, []).append(f)

    total_insignificant = 0
    total_significant = 0

    print("=" * 72)
    print("ASSET FRAME CHANGE CLASSIFIER - Mode: STAGING TO GIT (git add)")
    print("=" * 72)

    for manual_name, m_files in sorted(by_manual.items()):
        res = process_manual(
            manual_name,
            m_files,
            repo_root,
            tolerance_pixels=args.tolerance,
        )
        staged_count = len(res["staged"])
        unstaged_count = len(res["unstaged"])
        total_insignificant += staged_count
        total_significant += unstaged_count

        print(f"\nManual: {manual_name}")
        print(f"  Total modified:              {res['total']}")
        print(f"  Insignificant (Staged):      {staged_count}")
        print(f"  Significant (Remain unstaged): {unstaged_count}")

        # Show detailed categorization of unstaged items
        tag_changed = [x for x in res["unstaged"] if "semantic tag changed" in x[1]]
        count_changed = [x for x in res["unstaged"] if "asset count changed" in x[1]]
        content_changed = [x for x in res["unstaged"] if "semantic tag changed" not in x[1] and "asset count changed" not in x[1]]

        print(f"    - Semantic tag changed:     {len(tag_changed)}")
        print(f"    - Asset count changed:       {len(count_changed)}")
        print(f"    - Ink/content difference:    {len(content_changed)}")

        if content_changed[:3]:
            print("    Sample content changes:")
            for path, reason in content_changed[:3]:
                print(f"      * {Path(path).name}: {reason}")

    print("\n" + "=" * 72)
    print(f"SUMMARY: {total_insignificant} frames staged into git index.")
    print(f"         {total_significant} frames preserved unstaged in working tree for review.")
    print("=" * 72)


if __name__ == "__main__":
    main()
