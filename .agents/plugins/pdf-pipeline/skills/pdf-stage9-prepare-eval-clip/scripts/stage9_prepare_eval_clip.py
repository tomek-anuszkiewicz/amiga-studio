"""Stage 9: Prepare Eval Frames & Apply Recrops (eval-clip Loop Governor).

Coordinates the deterministic side of the Stage 9 <-> Stage 10 evaluation loop:
1. --prep-frames:
   Renders full-page evaluation images where the active target asset is highlighted
   with a thick RED rectangle, and any sibling assets on the same page are marked with
   BLUE rectangles. Saves to build/01_page_layout/eval_frames/page_XXXX_crop_Y_eval.png.
2. --prep-final-frames / --eval-final:
   Renders unified full-page evaluation images where ALL assets on that page are
   highlighted with thick RED rectangles representing their final bounding boxes.
   Saves to build/01_page_layout/eval_final/page_XXXX_eval_final.png.
3. --apply-recrops:
   Reads items flagged by Stage 10 with corrected coordinates, cuts new 300 DPI crops
   as _v1.png, _v2.png, updates the queue bounding box, and renders revised evaluation frames.
4. --finalize:
   Locks assets confirmed 'ok' into _clip_final.png, updates image links in
   build/01_page_layout/page_XXXX-images.md, and refreshes eval_final frames.
5. --status:
   Displays the exact state of the loop and reports whether Stage 9 or Stage 10 is next.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --prep-frames
    python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --eval-final
    python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --apply-recrops
    python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --finalize
    python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --status
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

try:
    import pymupdf as fitz
except ImportError:
    try:
        import fitz  # type: ignore
    except ImportError:
        fitz = None

try:
    from PIL import Image, ImageDraw
except ImportError:
    Image = None
    ImageDraw = None

try:
    import numpy as np
except ImportError:
    np = None

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


def draw_outward_rectangle(
    draw: ImageDraw.ImageDraw,
    inner_box: Tuple[float, float, float, float],
    outline: Tuple[int, int, int],
    width: int,
    max_w: int,
    max_h: int,
) -> None:
    """Draw a rectangular border strictly OUTSIDE the inner bounding box.

    The inner boundary matches the exact bounding box so the border never
    encroaches onto the asset content.
    """
    x0, y0, x1, y1 = inner_box
    bx0 = max(0, int(round(x0)) - width)
    by0 = max(0, int(round(y0)) - width)
    bx1 = min(max_w - 1, int(round(x1)) + width)
    by1 = min(max_h - 1, int(round(y1)) + width)
    draw.rectangle([bx0, by0, bx1, by1], outline=outline, width=width)


def resolve_manual_paths(target_input: str) -> Tuple[Path, Optional[Path], Path, Path]:
    """Resolve manual directory, PDF file, layout directory, and queue JSON path."""
    p = Path(target_input)
    if not p.is_absolute():
        p = (Path.cwd() / p).resolve()

    if p.is_file() and p.suffix.lower() == ".pdf":
        manual_dir = p.parent.parent if p.parent.name == "build" else p.parent
        pdf_path: Optional[Path] = p
    elif p.is_dir():
        manual_dir = p.parent if p.name == "build" else p
        pdf_path = None
    else:
        candidate = Path.cwd() / target_input
        if candidate.is_dir():
            manual_dir = candidate.parent.resolve() if candidate.name == "build" else candidate.resolve()
            pdf_path = None
        else:
            manual_dir = p
            pdf_path = None

    if pdf_path is None and manual_dir.exists():
        ocr_candidate = None
        std_candidate = None
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            for f in build_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    ocr_candidate = f
                    break
                elif not f.name.endswith("_ocr.pdf"):
                    if std_candidate is None or len(f.name) < len(std_candidate.name):
                        std_candidate = f
        for f in manual_dir.glob("*.pdf"):
            if f.name.endswith("_ocr.pdf") and not ocr_candidate:
                ocr_candidate = f
                break
            elif not f.name.endswith("_ocr.pdf"):
                if std_candidate is None or len(f.name) < len(std_candidate.name):
                    std_candidate = f
        pdf_path = ocr_candidate or std_candidate

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
    if not queue_path.exists():
        sys.stderr.write(f"Error: Queue file not found: {queue_path}\n")
        sys.exit(1)
    with open(queue_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        data = {"queue": data, "stats": {}}
    return data


def save_queue(queue_path: Path, data: Dict[str, Any]) -> None:
    """Save the updated queue JSON with recalculated statistics."""
    items = data.get("queue", [])
    completed = sum(1 for it in items if it.get("final_asset") is not None or it.get("status") == "completed")
    checked = sum(1 for it in items if it.get("eval_clip") == "ok")
    pending = len(items) - completed

    by_status: Dict[str, int] = {}
    by_type: Dict[str, int] = {}
    by_eval_clip: Dict[str, int] = {}

    for it in items:
        st = it.get("status", "pending")
        by_status[st] = by_status.get(st, 0) + 1
        dt = it.get("detected_type")
        if dt:
            by_type[dt] = by_type.get(dt, 0) + 1
        ev = it.get("eval_clip")
        if ev:
            by_eval_clip[ev] = by_eval_clip.get(ev, 0) + 1

    data["stats"] = {
        "total": len(items),
        "completed": completed,
        "checked": checked,
        "pending": pending,
        "by_status": by_status,
        "by_type": by_type,
        "by_eval_clip": by_eval_clip,
    }

    with open(queue_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)


def check_asset_white_buffer(
    page_img: Image.Image,
    box: List[int],
    buffer_units: int = 50,
    dark_threshold: int = 200,
) -> Tuple[bool, Optional[List[int]], List[str]]:
    """Inspect asset bounding box perimeter and safe horizontal and vertical white buffers.

    Returns:
        (has_defect, expanded_box, defect_reasons)
    """
    if np is None or box == [0, 0, 1000, 1000] or len(box) != 4:
        return False, None, []

    ymin, xmin, ymax, xmax = box
    try:
        arr = np.array(page_img.convert("L"))
    except Exception:
        return False, None, []

    ph, pw = arr.shape
    py0 = max(0, min(ph - 1, int(round(ymin * ph / 1000.0))))
    px0 = max(0, min(pw - 1, int(round(xmin * pw / 1000.0))))
    py1 = max(py0 + 1, min(ph, int(round(ymax * ph / 1000.0))))
    px1 = max(px0 + 1, min(pw, int(round(xmax * pw / 1000.0))))

    # 1. Outer 1-pixel perimeter inspection
    top_dark = int(np.sum(arr[py0, px0:px1] < dark_threshold))
    bot_dark = int(np.sum(arr[py1 - 1, px0:px1] < dark_threshold))
    left_dark = int(np.sum(arr[py0:py1, px0] < dark_threshold))
    right_dark = int(np.sum(arr[py0:py1, px1 - 1] < dark_threshold))

    # 2. Horizontal safe white buffer (50 units = 5% page width)
    buf_px = int(round(buffer_units * pw / 1000.0))

    # Right buffer zone
    r_buf_end = min(pw, px1 + buf_px)
    right_buf = arr[py0:py1, px1:r_buf_end]
    right_buf_dark = int(np.sum(right_buf < dark_threshold)) if right_buf.size > 0 else 0

    # Left buffer zone
    l_buf_start = max(0, px0 - buf_px)
    left_buf = arr[py0:py1, l_buf_start:px0]
    left_buf_dark = int(np.sum(left_buf < dark_threshold)) if left_buf.size > 0 else 0

    # Check for immediate vertical white gutter separating asset from side text
    # (No-Side-Wrapping rule: >= 15 continuous blank pixel columns immediately
    # outside the crop indicates a column gutter, not a sliced word).
    right_has_gutter = False
    if right_dark == 0 and right_buf.size > 0:
        g_count = 0
        for cx in range(right_buf.shape[1]):
            if np.all(right_buf[:, cx] >= dark_threshold):
                g_count += 1
            else:
                break
        if g_count >= 15:
            right_has_gutter = True

    left_has_gutter = False
    if left_dark == 0 and left_buf.size > 0:
        g_count = 0
        for cx in range(left_buf.shape[1] - 1, -1, -1):
            if np.all(left_buf[:, cx] >= dark_threshold):
                g_count += 1
            else:
                break
        if g_count >= 15:
            left_has_gutter = True

    # 3. Vertical safe white buffer (~15-20 pixels, or ~15 units)
    v_buf_px = max(15, int(round(15 * ph / 1000.0)))

    # Top buffer zone
    t_buf_start = max(0, py0 - v_buf_px)
    top_buf = arr[t_buf_start:py0, px0:px1]
    top_buf_dark = int(np.sum(top_buf < dark_threshold)) if top_buf.size > 0 else 0

    # Bottom buffer zone
    b_buf_end = min(ph, py1 + v_buf_px)
    bot_buf = arr[py1:b_buf_end, px0:px1]
    bot_buf_dark = int(np.sum(bot_buf < dark_threshold)) if bot_buf.size > 0 else 0

    # Check for continuous horizontal white gutter separating asset from preceding/following text
    # (>= 8 continuous blank pixel rows immediately outside the crop indicates a paragraph/caption gutter)
    top_has_gutter = False
    if top_dark == 0 and top_buf.size > 0:
        g_count = 0
        for ry in range(py0 - 1, t_buf_start - 1, -1):
            if np.all(arr[ry, px0:px1] >= dark_threshold):
                g_count += 1
            else:
                break
        if g_count >= 8:
            top_has_gutter = True

    bot_has_gutter = False
    if bot_dark == 0 and bot_buf.size > 0:
        g_count = 0
        for ry in range(py1, b_buf_end):
            if np.all(arr[ry, px0:px1] >= dark_threshold):
                g_count += 1
            else:
                break
        if g_count >= 4:
            bot_has_gutter = True

    defects: List[str] = []
    if top_dark > 0:
        defects.append("clipped_top")
    elif top_buf_dark > 10 and not top_has_gutter:
        defects.append("sliced_top_buffer")

    if bot_dark > 0:
        defects.append("clipped_bottom")
    elif bot_buf_dark > 10 and not bot_has_gutter:
        defects.append("sliced_bottom_buffer")

    if left_dark > 0:
        defects.append("clipped_left")
    elif left_buf_dark > 10 and not left_has_gutter:
        defects.append("sliced_left_buffer")

    if right_dark > 0:
        defects.append("clipped_right")
    elif right_buf_dark > 10 and not right_has_gutter:
        defects.append("sliced_right_buffer")

    if not defects:
        return False, None, []

    # Calculate expanded candidate box
    new_box = list(box)

    # Expand rightward across text and small gaps up to clean whitespace margin
    if "clipped_right" in defects or "sliced_right_buffer" in defects:
        cur_x = px1
        last_ink_x = px1
        gap = 0
        while cur_x < pw:
            col = arr[py0:py1, cur_x]
            if np.any(col < dark_threshold):
                last_ink_x = cur_x
                gap = 0
            else:
                gap += 1
                if gap >= 18:  # column gutter or margin reached
                    break
            cur_x += 1
        new_px1 = min(pw - 1, last_ink_x + 8)
        new_xmax = int(round(new_px1 * 1000.0 / pw))
        if new_xmax > xmax:
            new_box[3] = new_xmax

    # Expand leftward across text and small gaps up to clean whitespace margin
    if "clipped_left" in defects or "sliced_left_buffer" in defects:
        cur_x = px0
        first_ink_x = px0
        gap = 0
        while cur_x >= 0:
            col = arr[py0:py1, cur_x]
            if np.any(col < dark_threshold):
                first_ink_x = cur_x
                gap = 0
            else:
                gap += 1
                if gap >= 18:
                    break
            cur_x -= 1
        new_px0 = max(0, first_ink_x - 8)
        new_xmin = int(round(new_px0 * 1000.0 / pw))
        if new_xmin < xmin:
            new_box[1] = new_xmin

    # Expand top across text and small gaps up to clean whitespace margin
    if "clipped_top" in defects or "sliced_top_buffer" in defects:
        cur_y = py0
        first_ink_y = py0
        gap = 0
        while cur_y >= 0:
            row = arr[cur_y, px0:px1]
            if np.any(row < dark_threshold):
                first_ink_y = cur_y
                gap = 0
            else:
                gap += 1
                if gap >= 12:  # clean margin/gutter reached
                    break
            cur_y -= 1
        new_py0 = max(0, first_ink_y - 8)
        new_ymin = int(round(new_py0 * 1000.0 / ph))
        if new_ymin < ymin:
            new_box[0] = new_ymin

    # Expand bottom across text and small gaps up to clean whitespace margin
    if "clipped_bottom" in defects or "sliced_bottom_buffer" in defects:
        cur_y = py1
        last_ink_y = py1
        gap = 0
        while cur_y < ph:
            row = arr[cur_y, px0:px1]
            if np.any(row < dark_threshold):
                last_ink_y = cur_y
                gap = 0
            else:
                gap += 1
                if gap >= 12:  # clean margin/gutter reached
                    break
            cur_y += 1
        new_py1 = min(ph - 1, last_ink_y + 8)
        new_ymax = int(round(new_py1 * 1000.0 / ph))
        if new_ymax > ymax:
            new_box[2] = new_ymax

    exp_box = new_box if (new_box != list(box) and max(abs(n - b) for n, b in zip(new_box, box)) > 3) else None
    return True, exp_box, defects


def render_eval_frames(
    manual_dir: Path,
    layout_dir: Path,
    queue_data: Dict[str, Any],
    pages_filter: Optional[Set[int]] = None,
    force: bool = False,
) -> int:
    """Generate annotated full-page PNGs with Red (target) and Blue (siblings) frames."""
    if Image is None or ImageDraw is None:
        sys.stderr.write("Error: Pillow is required. Install via 'pip install pillow'.\n")
        sys.exit(1)

    eval_frames_dir = layout_dir / "eval_frames"
    eval_frames_dir.mkdir(parents=True, exist_ok=True)

    items = queue_data.get("queue", [])
    # Group items by page number
    by_page: Dict[int, List[Dict[str, Any]]] = {}
    for it in items:
        p_num = it.get("page_number", 0)
        if pages_filter and p_num not in pages_filter:
            continue
        by_page.setdefault(p_num, []).append(it)

    frames_rendered = 0

    for p_num, page_items in sorted(by_page.items()):
        page_png_path = layout_dir / f"page_{p_num:04d}.png"
        if not page_png_path.is_file():
            sys.stderr.write(f"Warning: Page preview missing for page {p_num}: {page_png_path.name}\n")
            continue

        for target_item in page_items:
            t_box = target_item.get("active_box")
            # Auto-approve full-page assets [0, 0, 1000, 1000]
            if t_box == [0, 0, 1000, 1000]:
                if target_item.get("eval_clip") != "ok":
                    target_item["eval_clip"] = "ok"
                    target_item["status"] = "eval_ok"
                target_item["margin_defect"] = False
                continue

            # Skip if already finalized or confirmed ok and not forcing
            if (target_item.get("final_asset") or target_item.get("eval_clip") == "ok") and not force:
                continue

            aid = target_item.get("asset_id") or Path(target_item.get("asset", "")).stem
            eval_img_name = f"{aid}_eval.png"
            eval_img_path = eval_frames_dir / eval_img_name

            # Check if frame exists and matches current version
            if eval_img_path.is_file() and not force and target_item.get("eval_frame"):
                continue

            try:
                base_img = Image.open(page_png_path).convert("RGB")
            except Exception as e:
                sys.stderr.write(f"Error opening {page_png_path}: {e}\n")
                continue

            draw = ImageDraw.Draw(base_img)
            img_w, img_h = base_img.size

            # Evaluate white buffer margin
            has_defect, exp_box, defect_list = check_asset_white_buffer(base_img, t_box)
            target_item["margin_defect"] = has_defect
            target_item["expanded_box"] = exp_box if has_defect else None
            target_item["defects_found"] = ", ".join(defect_list) if has_defect else None

            # Keep status as pending for Stage 10 multimodal Agent evaluation unless already awaiting recrop
            if not target_item.get("final_asset") and target_item.get("eval_clip") != "ok":
                if target_item.get("status") != "awaiting_reclip":
                    target_item["status"] = "pending"
                    target_item["eval_clip"] = None
                    target_item["corrected_box"] = None

            # Draw sibling boxes in BLUE first
            for sib in page_items:
                if sib is target_item:
                    continue
                s_box = sib.get("active_box")
                if s_box and len(s_box) == 4:
                    s_ymin, s_xmin, s_ymax, s_xmax = s_box
                    sx0 = (s_xmin / 1000.0) * img_w
                    sy0 = (s_ymin / 1000.0) * img_h
                    sx1 = (s_xmax / 1000.0) * img_w
                    sy1 = (s_ymax / 1000.0) * img_h
                    sw = 4
                    draw_outward_rectangle(draw, (sx0, sy0, sx1, sy1), (0, 102, 255), sw, img_w, img_h)

            # Draw active target box in solid RED on top
            draw_box = t_box
            if draw_box and len(draw_box) == 4:
                d_ymin, d_xmin, d_ymax, d_xmax = draw_box
                dx0 = (d_xmin / 1000.0) * img_w
                dy0 = (d_ymin / 1000.0) * img_h
                dx1 = (d_xmax / 1000.0) * img_w
                dy1 = (d_ymax / 1000.0) * img_h
                tw = 6
                draw_outward_rectangle(draw, (dx0, dy0, dx1, dy1), (255, 0, 0), tw, img_w, img_h)

            base_img.save(str(eval_img_path), format="PNG")
            rel_eval_frame = to_relative_posix(eval_img_path, manual_dir)
            target_item["eval_frame"] = rel_eval_frame
            frames_rendered += 1

    return frames_rendered


def get_asset_type_badge(item: Dict[str, Any]) -> str:
    """Return shorthand asset type badge: MD, HTML, or IMG."""
    dtype = str(item.get("detected_type") or "").strip().lower()
    if dtype in ("table_markdown", "md", "markdown"):
        return "MD"
    elif dtype in ("table_html", "html"):
        return "HTML"
    else:
        return "IMG"


def render_eval_final_frames(
    manual_dir: Path,
    layout_dir: Path,
    queue_data: Dict[str, Any],
    pages_filter: Optional[Set[int]] = None,
    force: bool = False,
    queue_path: Optional[Path] = None,
) -> int:
    """Generate unified full-page PNGs in eval_final/ for all document pages.

    Pages with assets show RED bounding boxes with MD/HTML/MERM/IMG type badges,
    full-page assets display a prominent top banner, and pages without assets are
    included cleanly so the reviewer can audit every page sequentially.
    """
    if Image is None or ImageDraw is None:
        sys.stderr.write("Error: Pillow is required. Install via 'pip install pillow'.\n")
        sys.exit(1)

    eval_final_dir = layout_dir / "eval_final"
    eval_final_dir.mkdir(parents=True, exist_ok=True)

    items = queue_data.get("queue", [])
    # Group items by page number
    by_page: Dict[int, List[Dict[str, Any]]] = {}
    for it in items:
        p_num = it.get("page_number", 0)
        if p_num <= 0:
            continue
        by_page.setdefault(p_num, []).append(it)

    # Discover all extracted page PNGs in layout_dir
    page_files = sorted(layout_dir.glob("page_[0-9][0-9][0-9][0-9].png"))
    all_pages: Set[int] = set()
    for pf in page_files:
        m = re.match(r"^page_(\d+)\.png$", pf.name)
        if m:
            all_pages.add(int(m.group(1)))

    if pages_filter:
        target_pages = sorted(all_pages.intersection(pages_filter)) if all_pages else sorted(pages_filter)
    else:
        target_pages = sorted(all_pages) if all_pages else sorted(by_page.keys())

    frames_rendered = 0

    # Try loading crisp fonts for labels
    font = None
    font_large = None
    try:
        from PIL import ImageFont
        try:
            font = ImageFont.load_default(size=18)
            font_large = ImageFont.load_default(size=22)
        except TypeError:
            font = ImageFont.load_default()
            font_large = font
    except Exception:
        font = None
        font_large = None

    tw = 6  # Outward border stroke thickness

    for p_num in target_pages:
        page_png_path = layout_dir / f"page_{p_num:04d}.png"
        if not page_png_path.is_file():
            continue

        eval_final_img_path = eval_final_dir / f"page_{p_num:04d}_eval_final.png"
        page_items = by_page.get(p_num, [])

        if eval_final_img_path.is_file() and not force:
            is_stale = False
            try:
                frame_stat = eval_final_img_path.stat()
                page_stat = page_png_path.stat()
                q_mtime = queue_path.stat().st_mtime if queue_path.is_file() else 0
                if frame_stat.st_mtime < page_stat.st_mtime or frame_stat.st_mtime < q_mtime:
                    is_stale = True
                elif page_items and frame_stat.st_size == page_stat.st_size:
                    # Un-annotated copy of the base page preview despite having visual assets
                    is_stale = True
            except OSError:
                is_stale = True

            if not is_stale:
                continue

        # If page has no visual assets, copy the clean page preview into eval_final
        if not page_items:
            try:
                shutil.copy2(str(page_png_path), str(eval_final_img_path))
                frames_rendered += 1
            except Exception as e:
                sys.stderr.write(f"Error copying {page_png_path.name} to eval_final: {e}\n")
            continue

        try:
            base_img = Image.open(page_png_path).convert("RGB")
        except Exception as e:
            sys.stderr.write(f"Error opening {page_png_path}: {e}\n")
            continue

        draw = ImageDraw.Draw(base_img)
        img_w, img_h = base_img.size

        for it in page_items:
            box = it.get("active_box")
            if not box or len(box) != 4:
                continue

            ymin, xmin, ymax, xmax = box
            aid = it.get("asset_id") or Path(it.get("asset", "")).stem
            short_id = re.sub(r"^page_\d+_", "", aid)
            is_full_page = (box == [0, 0, 1000, 1000] or (xmin == 0 and ymin == 0 and xmax == 1000 and ymax == 1000))

            if is_full_page:
                # Full-page asset: border touches image edges so add prominent informational top banner
                draw.rectangle([0, 0, img_w - 1, img_h - 1], outline=(255, 0, 0), width=tw)
                if font_large:
                    try:
                        banner_text = f"FULL-PAGE CROP (NO VISIBLE BORDER) [{short_id}]"
                        bbox = draw.textbbox((0, 0), banner_text, font=font_large)
                        bw = bbox[2] - bbox[0] + 36
                        bh = bbox[3] - bbox[1] + 16
                        bx0 = (img_w - bw) // 2
                        by0 = 16
                        draw.rectangle([bx0, by0, bx0 + bw, by0 + bh], fill=(255, 0, 0), outline=(255, 255, 255), width=2)
                        draw.text((bx0 + 18, by0 + 8), banner_text, fill=(255, 255, 255), font=font_large)
                    except Exception:
                        pass
                continue

            x0 = (xmin / 1000.0) * img_w
            y0 = (ymin / 1000.0) * img_h
            x1 = (xmax / 1000.0) * img_w
            y1 = (ymax / 1000.0) * img_h

            # Draw outward RED rectangle strictly outside bounding box
            draw_outward_rectangle(draw, (x0, y0, x1, y1), (255, 0, 0), tw, img_w, img_h)

            # Draw crop label badge (e.g. crop_1, crop_2, crop_3) at top-left
            if font:
                try:
                    bbox = draw.textbbox((0, 0), short_id, font=font)
                    th = bbox[3] - bbox[1] + 8
                    tw_badge = bbox[2] - bbox[0] + 16
                    bx0 = max(0, x0 - tw)
                    by0 = max(0, y0 - th - tw) if y0 - th - tw >= 0 else y0 + tw
                    bx1 = min(img_w - 1, bx0 + tw_badge)
                    by1 = by0 + th
                    draw.rectangle([bx0, by0, bx1, by1], fill=(255, 0, 0))
                    draw.text((bx0 + 8, by0 + 3), short_id, fill=(255, 255, 255), font=font)
                except Exception:
                    pass

        base_img.save(str(eval_final_img_path), format="PNG")
        rel_final_frame = to_relative_posix(eval_final_img_path, manual_dir)
        for it in page_items:
            it["eval_final_frame"] = rel_final_frame
        frames_rendered += 1

    return frames_rendered


def apply_recrops(
    manual_dir: Path,
    pdf_path: Path,
    layout_dir: Path,
    queue_data: Dict[str, Any],
    dpi: int = 300,
    pages_filter: Optional[Set[int]] = None,
) -> int:
    """Execute recrops for items where Stage 10 assigned corrected coordinates."""
    if fitz is None:
        sys.stderr.write("Error: PyMuPDF is required. Install via 'pip install pymupdf'.\n")
        sys.exit(1)

    doc = fitz.open(pdf_path)
    assets_dir = layout_dir / "assets"
    assets_dir.mkdir(parents=True, exist_ok=True)

    items = queue_data.get("queue", [])
    recrops_applied = 0

    for it in items:
        p_num = it.get("page_number", 0)
        if pages_filter and p_num not in pages_filter:
            continue

        # Check if item has a pending reclip with corrected_box
        corrected_box = it.get("corrected_box")
        eval_clip = str(it.get("eval_clip", "")).lower()
        needs_reclip = (
            corrected_box is not None
            or eval_clip in {"reclip_needed", "clipped_edge", "severed_text", "bad_bbox", "excess_whitespace"}
            or str(it.get("status", "")).lower() == "awaiting_reclip"
        )

        if not needs_reclip or not corrected_box or len(corrected_box) != 4:
            continue

        aid = it.get("asset_id") or Path(it.get("asset", "")).stem
        active_box = it.get("active_box")

        # 1. LOOP GUARD: Check if corrected_box is effectively identical to active_box
        if active_box and len(active_box) == 4:
            max_diff = max(abs(c - a) for c, a in zip(corrected_box, active_box))
            if max_diff <= 3:  # within <= 0.3% of page dimension
                print(f"[LOOP GUARD] Asset '{aid}': proposed box {corrected_box} is identical/saturated vs active box {active_box}. Auto-approving as 'ok'.")
                it["eval_clip"] = "ok"
                it["corrected_box"] = None
                it["status"] = "eval_ok"
                continue

        # 2. LOOP GUARD: Cap maximum reclip cycles (max 3 versions)
        curr_ver = it.get("current_version", 0)
        if curr_ver >= 3:
            print(f"[LOOP GUARD] Asset '{aid}': reached maximum reclip limit ({curr_ver} versions). Auto-locking active version as 'ok'.")
            it["eval_clip"] = "ok"
            it["corrected_box"] = None
            it["status"] = "eval_ok"
            continue

        p_num = it.get("page_number", 1)
        pdf_idx = max(0, min(len(doc) - 1, p_num - 1))
        page = doc[pdf_idx]
        page_w = page.rect.width
        page_h = page.rect.height

        ymin_n, xmin_n, ymax_n, xmax_n = corrected_box
        x0 = (xmin_n / 1000.0) * page_w
        y0 = (ymin_n / 1000.0) * page_h
        x1 = (xmax_n / 1000.0) * page_w
        y1 = (ymax_n / 1000.0) * page_h

        # Add subtle 1.5 - 2.0 pt padding so adjacent text isn't grazed
        p_rect = page.rect
        pad_x = 2.0
        pad_y = 1.5
        crop_rect = fitz.Rect(
            max(p_rect.x0, x0 - pad_x),
            max(p_rect.y0, y0 - pad_y),
            min(p_rect.x1, x1 + pad_x),
            min(p_rect.y1, y1 + pad_y),
        )

        # Increment version
        new_version = it.get("current_version", 0) + 1
        raw_aid = it.get("asset_id") or Path(it.get("asset", "")).stem
        base_aid = re.sub(r"(_v\d+|_clip_final)$", "", raw_aid)
        new_filename = f"{base_aid}_v{new_version}.png"
        new_path = assets_dir / new_filename

        scale = dpi / 72.0
        mat = fitz.Matrix(scale, scale)
        pix = page.get_pixmap(matrix=mat, clip=crop_rect, alpha=False)
        pix.save(str(new_path))

        rel_asset = to_relative_posix(new_path, manual_dir)
        it["current_version"] = new_version
        it["active_asset"] = rel_asset
        it["asset"] = rel_asset
        it["active_box"] = corrected_box
        it["corrected_box"] = None
        it["eval_clip"] = None  # Reset so it gets re-evaluated in loop
        it["status"] = "pending"

        versions = it.setdefault("versions", [])
        versions.append({
            "version": new_version,
            "box": corrected_box,
            "asset_file": rel_asset,
            "source": f"stage10_reclip_v{new_version}",
            "eval_result": None,
        })

        recrops_applied += 1

    doc.close()
    return recrops_applied


def finalize_assets(
    manual_dir: Path,
    layout_dir: Path,
    queue_data: Dict[str, Any],
    pages_filter: Optional[Set[int]] = None,
) -> int:
    """Lock confirmed 'ok' assets to _clip_final.png and update markdown links."""
    assets_dir = layout_dir / "assets"
    items = queue_data.get("queue", [])
    finalized_count = 0

    # Group updates per markdown file to minimize disk writes
    md_replacements: Dict[Path, List[Tuple[str, str]]] = {}

    for it in items:
        p_num = it.get("page_number")
        if pages_filter is not None and p_num not in pages_filter:
            continue

        is_ok = it.get("eval_clip") == "ok"
        has_final = it.get("final_asset") is not None
        
        if not is_ok and not has_final:
            continue

        aid = it.get("asset_id") or Path(it.get("asset", "")).stem
        original_aid = aid.replace("_clip_final", "")
        final_filename = f"{original_aid}_clip_final.png"
        final_path = assets_dir / final_filename

        did_work = False

        active_path = manual_dir / it.get("active_asset", it.get("asset", ""))
        if active_path.is_file():
            needs_copy = not final_path.exists()
            if not needs_copy and active_path != final_path:
                needs_copy = (active_path.stat().st_mtime > final_path.stat().st_mtime) or ("_v" in active_path.name)
            if needs_copy:
                shutil.copy2(str(active_path), str(final_path))
                did_work = True
            rel_final = to_relative_posix(final_path, manual_dir)
            if it.get("final_asset") != rel_final or it.get("asset") != rel_final:
                it["final_asset"] = rel_final
                it["asset"] = rel_final
                it["status"] = "eval_ok"
                did_work = True

        # Check and fix markdown replacements
        # Check and fix markdown replacements
        p_num = it.get("page_number", 0)
        target_md_name = f"page_{p_num:04d}-images-clip_final.md"
        target_md_path = layout_dir / target_md_name
        rel_target_md = f"build/01_page_layout/{target_md_name}"

        if not target_md_path.exists() or it.get("md_file") != rel_target_md:
            orig_md_path = layout_dir / f"page_{p_num:04d}-images.md"
            if not orig_md_path.exists():
                orig_md_path = layout_dir / f"page_{p_num:04d}.md"
            source_md = target_md_path if target_md_path.exists() else orig_md_path
            if source_md.is_file():
                old_ref = Path(it.get("original_asset", original_aid)).name
                old_ref = old_ref.replace("_clip_final", "")
                new_ref = final_filename
                md_replacements.setdefault(source_md, []).append((old_ref, new_ref))
                it["md_file"] = rel_target_md
                did_work = True

        if did_work:
            finalized_count += 1

    # Apply markdown replacements
    for md_path, repls in md_replacements.items():
        try:
            # Create new filename by appending -clip_final before the extension
            new_name = md_path.name.replace("-images.md", "-images-clip_final.md")
            if "-images-clip_final" not in new_name:
                new_name = md_path.name.replace(".md", "-clip_final.md")
            
            new_md_path = md_path.with_name(new_name)
            
            # Read from the -clip_final file if it was created in a previous pass for another asset
            source_path = new_md_path if new_md_path.exists() else md_path
            content = source_path.read_text(encoding="utf-8")
            
            for old_name, new_name in repls:
                content = content.replace(f"assets/{old_name}", f"assets/{new_name}")
                content = content.replace(old_name, new_name)
            
            new_md_path.write_text(content, encoding="utf-8")
        except Exception as e:
            sys.stderr.write(f"Error updating markdown links for {md_path.name}: {e}\n")

    # Ensure all pages in layout_dir have corresponding *-images-clip_final.md
    if layout_dir.exists():
        source_md_files = sorted(layout_dir.glob("page_*-images.md"))
        if not source_md_files:
            source_md_files = sorted(f for f in layout_dir.glob("page_*.md") if not f.name.endswith("-clip_final.md"))
        
        for src_md in source_md_files:
            m = re.match(r"page_(\d+)", src_md.name)
            if m and pages_filter is not None and int(m.group(1)) not in pages_filter:
                continue
            cf_name = src_md.name.replace("-images.md", "-images-clip_final.md")
            if "-images-clip_final" not in cf_name:
                cf_name = src_md.name.replace(".md", "-clip_final.md")
            cf_path = src_md.with_name(cf_name)
            
            if not cf_path.exists():
                try:
                    content = src_md.read_text(encoding="utf-8")
                    cf_path.write_text(content, encoding="utf-8")
                except Exception as e:
                    sys.stderr.write(f"Error copying final markdown for {src_md.name}: {e}\n")

    # Final pass across all *-images-clip_final.md: ensure all image references point to _clip_final.png
    if layout_dir.exists():
        for cf_path in sorted(layout_dir.glob("*-images-clip_final.md")):
            m = re.match(r"page_(\d+)", cf_path.name)
            if m and pages_filter is not None and int(m.group(1)) not in pages_filter:
                continue
            try:
                txt = cf_path.read_text(encoding="utf-8")
                updated_txt = txt
                for m_img in re.finditer(r'(assets/)(page_\d+_[^)\s"\']+?)(_v\d+)?(\.png)', txt):
                    prefix = m_img.group(1)
                    stem = m_img.group(2)
                    ext = m_img.group(4)
                    if not stem.endswith("_clip_final"):
                        expected_final = f"{prefix}{stem}_clip_final{ext}"
                        if (layout_dir / expected_final).is_file():
                            old_match = m_img.group(0)
                            updated_txt = updated_txt.replace(old_match, expected_final)
                if updated_txt != txt:
                    cf_path.write_text(updated_txt, encoding="utf-8")
            except Exception as e:
                sys.stderr.write(f"Error verifying clip_final references in {cf_path.name}: {e}\n")

    return finalized_count


def display_status(queue_path: Path, queue_data: Dict[str, Any], layout_dir: Optional[Path] = None) -> None:
    """Display current loop status and declare next action."""
    manual_dir = queue_path.parent.parent if queue_path.parent.name == "build" else queue_path.parent
    if layout_dir is None:
        layout_dir = manual_dir / "build" / "01_page_layout"

    items = queue_data.get("queue", [])
    total = len(items)

    finalized = sum(1 for it in items if it.get("final_asset") is not None)
    eval_ok = sum(1 for it in items if it.get("eval_clip") == "ok")
    needs_recrop = sum(
        1 for it in items
        if it.get("corrected_box") is not None
        or it.get("eval_clip") in {"reclip_needed", "clipped_edge", "severed_text", "bad_bbox", "excess_whitespace"}
        or it.get("status") == "awaiting_reclip"
    )
    needs_frames = sum(
        1 for it in items
        if not it.get("eval_frame")
        and not it.get("final_asset")
        and it.get("eval_clip") != "ok"
        and it.get("active_box") != [0, 0, 1000, 1000]
    )
    pending_eval = sum(
        1 for it in items
        if it.get("eval_clip") is None
        and it.get("final_asset") is None
    )

    missing_md_updates = 0
    for it in items:
        if it.get("eval_clip") == "ok" or it.get("final_asset") is not None:
            md_rel = it.get("md_file")
            if md_rel:
                if "-clip_final.md" not in md_rel:
                    missing_md_updates += 1
                else:
                    md_path = manual_dir / md_rel
                    if not md_path.exists():
                        missing_md_updates += 1

    # Check total expected pages vs *-images-clip_final.md documents
    missing_final_docs = 0
    unfinalized_refs = 0
    if layout_dir.exists():
        expected_md = sorted(layout_dir.glob("page_*-images.md"))
        if not expected_md:
            expected_md = sorted(f for f in layout_dir.glob("page_*.md") if not f.name.endswith("-clip_final.md"))
        
        for em in expected_md:
            cf_name = em.name.replace("-images.md", "-images-clip_final.md")
            if "-images-clip_final" not in cf_name:
                cf_name = em.name.replace(".md", "-clip_final.md")
            cf_file = em.with_name(cf_name)
            if not cf_file.is_file() or cf_file.stat().st_size == 0:
                missing_final_docs += 1
            else:
                try:
                    c_txt = cf_file.read_text(encoding="utf-8", errors="replace")
                    for m in re.finditer(r'!\[.*?\]\(([^)]+)\)|<img\s+[^>]*src=["\']([^"\']+)["\']', c_txt, re.IGNORECASE):
                        r = m.group(1) or m.group(2)
                        if r and not r.startswith("http") and not r.startswith("data:"):
                            clean_r = r.split()[0].strip().strip("<>\"'").replace("\\", "/")
                            if not clean_r.endswith("_clip_final.png"):
                                unfinalized_refs += 1
                except Exception:
                    pass

    eval_final_dir = layout_dir / "eval_final"
    final_frames_count = len(list(eval_final_dir.glob("page_*_eval_final.png"))) if eval_final_dir.is_dir() else 0

    print(f"\n=======================================================")
    print(f" STAGE 9: EVAL-CLIP LOOP DASHBOARD: {queue_path.name}")
    print(f"=======================================================")
    print(f"Total Visual Assets:      {total}")
    print(f"Finalized (_clip_final):  {finalized}/{total} ({finalized/total*100.0 if total else 0:.1f}%)")
    print(f"Confirmed 'ok':           {eval_ok}/{total}")
    print(f"Pending Multimodal Eval:  {pending_eval}")
    print(f"Awaiting Recrop:          {needs_recrop}")
    print(f"Missing Eval Frames:      {needs_frames}")
    if missing_md_updates > 0:
        print(f"Missing Markdown Updates: {missing_md_updates} (NEEDS FINALIZE)")
    if missing_final_docs > 0:
        print(f"Missing Final Docs:       {missing_final_docs} (*-images-clip_final.md)")
    if unfinalized_refs > 0:
        print(f"Unfinalized Image Refs:   {unfinalized_refs} (expected _clip_final.png)")
    print(f"Final Eval Frames:        {final_frames_count} page(s) (eval_final/)")
    print(f"-------------------------------------------------------")

    # Determine Next Recommended Pipeline Action
    if needs_recrop > 0:
        print(f"[NEXT ACTION] >>> Run Stage 9 Recrop: python .../stage9_prepare_eval_clip.py ... --apply-recrops")
    elif needs_frames > 0:
        print(f"[NEXT ACTION] >>> Run Stage 9 Frames: python .../stage9_prepare_eval_clip.py ... --prep-frames")
    elif pending_eval > 0:
        print(f"[NEXT ACTION] >>> Run Stage 10: In-Session Agent multimodal evaluation of pending assets")
    elif eval_ok > finalized or missing_md_updates > 0 or missing_final_docs > 0 or unfinalized_refs > 0:
        print(f"[NEXT ACTION] >>> Run Stage 9 Finalize: python .../stage9_prepare_eval_clip.py ... --finalize")
    elif finalized == total and total > 0 and missing_md_updates == 0 and missing_final_docs == 0 and unfinalized_refs == 0:
        print(f"[STATUS] >>> All assets validated and locked into _clip_final.png & all final docs verified! Ready for Stage 11/12.")
    else:
        print(f"[STATUS] >>> Queue ready for processing.")
    print(f"=======================================================\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 9: Prepare Eval Frames & Apply Recrops.")
    parser.add_argument("target", help="Path to manual directory or PDF file")
    parser.add_argument("--prep-frames", action="store_true", help="Generate Red/Blue full-page annotated evaluation frames")
    parser.add_argument("--prep-final-frames", "--eval-final", action="store_true", dest="prep_final_frames", help="Generate unified full-page evaluation frames in eval_final/ with all assets framed in red")
    parser.add_argument("--apply-recrops", action="store_true", help="Execute recrops for items with corrected bounding boxes")
    parser.add_argument("--finalize", action="store_true", help="Lock confirmed 'ok' assets as _clip_final.png, update markdown, and refresh eval_final frames")
    parser.add_argument("--status", action="store_true", help="Display loop status and next recommended action")
    parser.add_argument("--pages", "-p", help="Optional page range to filter (e.g. '1-10', '14')")
    parser.add_argument("--dpi", type=int, default=300, help="Output DPI for recrops (default: 300)")
    parser.add_argument("--force", action="store_true", help="Force re-rendering of existing frames")
    parser.add_argument("--record-verdicts", help="JSON string or path to JSON file containing asset verdicts")

    args = parser.parse_args()

    manual_dir, pdf_path, layout_dir, queue_path = resolve_manual_paths(args.target)
    queue_data = load_queue(queue_path)
    pages_filter = parse_page_range(args.pages) if args.pages else None

    if args.status:
        display_status(queue_path, queue_data, layout_dir)
        return

    modified = False

    if args.record_verdicts:
        raw_v = args.record_verdicts.strip()
        if Path(raw_v).is_file():
            verdicts = json.loads(Path(raw_v).read_text(encoding="utf-8"))
        else:
            try:
                verdicts = json.loads(raw_v)
            except Exception:
                # Fallback for Windows PowerShell stripped quotes
                fixed_v = re.sub(r'([{,]\s*)([a-zA-Z0-9_]+)(\s*:)', r'\1"\2"\3', raw_v)
                fixed_v = re.sub(r'(:\s*)([a-zA-Z0-9_]+)(\s*[,}])', r'\1"\2"\3', fixed_v)
                verdicts = json.loads(fixed_v)

        queue_items = queue_data.get("queue", [])
        item_map = {it.get("asset_id"): it for it in queue_items if it.get("asset_id")}
        updated = 0

        for aid, info in verdicts.items():
            if aid in item_map:
                it = item_map[aid]
                if isinstance(info, dict):
                    v_text = str(info.get("verdict", "")).lower()
                    box = info.get("box") or info.get("corrected_box")
                elif isinstance(info, list) and len(info) == 4:
                    v_text = "reclip_needed"
                    box = info
                else:
                    v_text = str(info).lower()
                    box = None

                if v_text == "ok":
                    it["eval_clip"] = "ok"
                    it["corrected_box"] = None
                    it["status"] = "eval_ok"
                    updated += 1
                elif v_text in {"reclip_needed", "clipped_edge", "severed_text", "bad_bbox", "excess_whitespace"}:
                    it["eval_clip"] = "reclip_needed"
                    it["status"] = "awaiting_reclip"
                    if box and len(box) == 4:
                        it["corrected_box"] = box
                    updated += 1

        print(f"[STAGE 9] Recorded verdicts for {updated} asset(s) in '{manual_dir.name}'.")
        modified = True

    elif args.apply_recrops:
        if not pdf_path or not pdf_path.is_file():
            sys.stderr.write(f"Error: Valid PDF file required for recropping. Found: {pdf_path}\n")
            sys.exit(1)
        recrops = apply_recrops(manual_dir, pdf_path, layout_dir, queue_data, dpi=args.dpi, pages_filter=pages_filter)
        print(f"[STAGE 9] Applied {recrops} recrop(s) into versioned assets.")
        modified = True
        # Immediately refresh frames for the newly recropped assets
        frames = render_eval_frames(manual_dir, layout_dir, queue_data, pages_filter=pages_filter, force=True)
        print(f"[STAGE 9] Refreshed {frames} evaluation frame(s).")

    elif args.prep_final_frames:
        frames = render_eval_final_frames(manual_dir, layout_dir, queue_data, pages_filter=pages_filter, force=args.force, queue_path=queue_path)
        print(f"[STAGE 9] Rendered {frames} unified final evaluation frame(s) in: build/01_page_layout/eval_final/")
        modified = True

    elif args.prep_frames:
        frames = render_eval_frames(manual_dir, layout_dir, queue_data, pages_filter=pages_filter, force=args.force)
        print(f"[STAGE 9] Rendered {frames} full-page evaluation frame(s) in: build/01_page_layout/eval_frames/")
        modified = True

    elif args.finalize:
        finalized = finalize_assets(manual_dir, layout_dir, queue_data, pages_filter=pages_filter)
        print(f"[STAGE 9] Finalized {finalized} asset(s) as _clip_final.png and updated markdown links.")
        final_frames = render_eval_final_frames(manual_dir, layout_dir, queue_data, pages_filter=pages_filter, force=True, queue_path=queue_path)
        print(f"[STAGE 9] Refreshed {final_frames} final evaluation frame(s) in: build/01_page_layout/eval_final/")
        modified = True

    else:
        # Default behavior: run status
        display_status(queue_path, queue_data, layout_dir)
        return

    if modified:
        save_queue(queue_path, queue_data)
        display_status(queue_path, queue_data, layout_dir)


if __name__ == "__main__":
    main()
