"""Stage 15: Render Asset Frames (Post-Conversion & Classification Review).

Generates full-page review images in build/01_page_layout/asset_frames/
displaying final asset bounding boxes annotated with their classified semantic types:
- MD:      GFM Markdown table (table_markdown or reduced table_html)
- HTML:    HTML table converted in Stage 12 (conversion_stage: 12)
- IMG:     Image / schematic / figure (image, conversion_stage: 14)
- ERR:     Asset missing valid conversion_stage (fallback)
- UNSET:   Pending classification / conversion

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py "path/to/manual_dir"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py "path/to/manual_dir" --pages 17,26
    python .agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py "path/to/manual_dir" --status
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

    from PIL import Image, ImageDraw, ImageFont

except ImportError:

    Image = None

    ImageDraw = None

    ImageFont = None



# Safe console encoding on Windows

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





def get_asset_type_badge(item: Dict[str, Any]) -> str:
    """Return shorthand asset type badge: MD, HTML, IMG, or ERR/UNSET."""
    raw_type = item.get("detected_type")
    if not raw_type:
        return "UNSET"

    dtype = str(raw_type).strip().lower()
    if dtype in ("table_html", "html"):
        if item.get("reduced_to_markdown") is True:
            return "MD"
        stage = item.get("conversion_stage")
        if stage in (12, 13):
            return "HTML"
        return "ERR"

    elif dtype in ("image", "img"):
        return "IMG"

    elif dtype in ("table_markdown", "md", "markdown"):
        return "MD"

    else:
        return "ERR"





def render_asset_frames(

    manual_dir: Path,

    layout_dir: Path,

    queue_data: Dict[str, Any],

    pages_filter: Optional[Set[int]] = None,

    force: bool = False,

    queue_path: Optional[Path] = None,

) -> int:

    """Generate full-page PNGs in asset_frames/ annotated with classified semantic types."""

    if Image is None or ImageDraw is None:

        sys.stderr.write("Error: Pillow is required. Install via 'pip install pillow'.\n")

        sys.exit(1)



    asset_frames_dir = layout_dir / "asset_frames"

    asset_frames_dir.mkdir(parents=True, exist_ok=True)



    items = queue_data.get("queue", [])

    by_page: Dict[int, List[Dict[str, Any]]] = {}

    for it in items:

        p_num = it.get("page_number", 0)

        if p_num <= 0:

            continue

        by_page.setdefault(p_num, []).append(it)



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



    font = None

    font_large = None

    try:

        if ImageFont is not None:

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



        asset_frame_img_path = asset_frames_dir / f"page_{p_num:04d}_asset_frame.png"

        page_items = by_page.get(p_num, [])



        if asset_frame_img_path.is_file() and not force:

            # Check if existing frame is fresh and properly annotated

            is_stale = False

            try:

                frame_stat = asset_frame_img_path.stat()

                page_stat = page_png_path.stat()

                q_mtime = queue_path.stat().st_mtime if queue_path and queue_path.is_file() else 0

                if frame_stat.st_mtime < page_stat.st_mtime or frame_stat.st_mtime < q_mtime:

                    is_stale = True

                elif page_items and frame_stat.st_size == page_stat.st_size:

                    # Un-annotated copy of the base page preview despite having visual assets

                    is_stale = True

            except OSError:

                is_stale = True



            if not is_stale:

                continue



        # Pages without visual assets: copy clean preview

        if not page_items:

            try:

                shutil.copy2(str(page_png_path), str(asset_frame_img_path))

                frames_rendered += 1

            except Exception as e:

                sys.stderr.write(f"Error copying {page_png_path.name} to asset_frames: {e}\n")

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

            type_badge = get_asset_type_badge(it)

            is_full_page = (box == [0, 0, 1000, 1000] or (xmin == 0 and ymin == 0 and xmax == 1000 and ymax == 1000))



            if is_full_page:

                draw.rectangle([0, 0, img_w - 1, img_h - 1], outline=(255, 0, 0), width=tw)

                if font_large:

                    try:

                        banner_text = f"FULL-PAGE CROP (NO VISIBLE BORDER) [{type_badge}]"

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



            # Draw classified semantic badge (HTML, IMG, UNSET)

            if font:

                try:

                    bbox = draw.textbbox((0, 0), type_badge, font=font)

                    th = bbox[3] - bbox[1] + 8

                    tw_badge = bbox[2] - bbox[0] + 16

                    bx0 = max(0, x0 - tw)

                    by0 = max(0, y0 - th - tw) if y0 - th - tw >= 0 else y0 + tw

                    bx1 = min(img_w - 1, bx0 + tw_badge)

                    by1 = by0 + th

                    draw.rectangle([bx0, by0, bx1, by1], fill=(255, 0, 0))

                    draw.text((bx0 + 8, by0 + 3), type_badge, fill=(255, 255, 255), font=font)

                except Exception:

                    pass



        base_img.save(str(asset_frame_img_path), format="PNG")

        frames_rendered += 1



    return frames_rendered





def display_status(queue_path: Path, queue_data: Dict[str, Any], layout_dir: Path) -> None:

    """Display classification status and asset_frames coverage."""

    items = queue_data.get("queue", [])

    total = len(items)



    classified = sum(1 for it in items if it.get("detected_type"))

    by_type: Dict[str, int] = {}

    for it in items:

        dt = it.get("detected_type")

        if dt:

            by_type[dt] = by_type.get(dt, 0) + 1



    asset_frames_dir = layout_dir / "asset_frames"

    rendered_frames = len(list(asset_frames_dir.glob("page_*_asset_frame.png"))) if asset_frames_dir.is_dir() else 0



    print(f"\n=======================================================")
    print(f" STAGE 16: ASSET FRAMES DASHBOARD: {queue_path.name}")
    print(f"=======================================================")

    print(f"Total Visual Assets:      {total}")

    print(f"Classified / Converted:   {classified}/{total} ({classified/total*100.0 if total else 0:.1f}%)")

    print(f"Pending Classification:   {total - classified}")

    for t_name, count in sorted(by_type.items()):

        print(f"  - {t_name:15s}: {count}")

    print(f"Rendered Asset Frames:    {rendered_frames} page(s) in: build/01_page_layout/asset_frames/")

    print(f"=======================================================\n")





def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 15: Render Classified Asset Frames (MD, HTML, IMG, ERR).")
    parser.add_argument("target", help="Path to manual directory or PDF file")
    parser.add_argument("--pages", "-p", help="Optional page range to filter (e.g. '1-10', '17,26')")
    parser.add_argument("--force", action="store_true", help="Force re-rendering of existing asset frames")
    parser.add_argument("--status", action="store_true", help="Display classification stats and frame counts")

    args = parser.parse_args()

    manual_dir, pdf_path, layout_dir, queue_path = resolve_manual_paths(args.target)
    queue_data = load_queue(queue_path)
    pages_filter = parse_page_range(args.pages) if args.pages else None

    if args.status:
        display_status(queue_path, queue_data, layout_dir)
        return

    frames = render_asset_frames(manual_dir, layout_dir, queue_data, pages_filter=pages_filter, force=args.force, queue_path=queue_path)
    print(f"[STAGE 15] Rendered {frames} classified asset frame(s) in: build/01_page_layout/asset_frames/")
    display_status(queue_path, queue_data, layout_dir)





if __name__ == "__main__":

    main()

