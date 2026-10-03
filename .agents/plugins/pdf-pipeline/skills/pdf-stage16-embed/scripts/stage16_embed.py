"""Stage 16: Embed Markdown.

Injects synthesized table fragments (GFM or HTML) and image descriptions (with collapsed breakdown)
directly into each page's Markdown document:
- Input:  build/01_page_layout/page_XXXX-images-clip_final.md
- Output: build/01_page_layout/page_XXXX-embed.md
- Updates: <manual_dir>/<stem>_assets_queue.json (is_embedded: True, embed_stats)

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage16-embed/scripts/stage16_embed.py "path/to/manual_dir"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage16-embed/scripts/stage16_embed.py "path/to/manual_dir" --pages 14-20
    python .agents/plugins/pdf-pipeline/skills/pdf-stage16-embed/scripts/stage16_embed.py "path/to/manual_dir" --status
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

    """Save the updated queue JSON."""

    temp_path = queue_path.with_suffix(".tmp")

    with open(temp_path, "w", encoding="utf-8") as f:

        json.dump(data, f, indent=2, ensure_ascii=False)

    temp_path.replace(queue_path)





def run_embedding(

    queue_path: Path,

    manual_dir: Path,

    target_pages: Optional[Set[int]] = None,

    force: bool = False,

) -> Tuple[int, int, int]:

    """Embed converted asset fragments into page_XXXX-embed.md."""

    data = load_queue(queue_path)

    items = data.get("queue", [])

    layout_dir = manual_dir / "build" / "01_page_layout"



    # Group items by page

    page_items: Dict[int, List[Dict[str, Any]]] = {}

    for it in items:

        p = it.get("page_number", 0)

        page_items.setdefault(p, []).append(it)



    embedded_pages = 0

    embedded_assets = 0

    skipped_assets = 0



    all_page_files = sorted(layout_dir.glob("page_*.md"))

    available_pages = set(page_items.keys())

    for f in all_page_files:

        m = re.match(r"page_(\d{4})\.md$", f.name)

        if m:

            available_pages.add(int(m.group(1)))



    processed_pages: Set[int] = set()

    for p in sorted(available_pages):

        assets = page_items.get(p, [])

        if target_pages and p not in target_pages:

            continue



        target_embed_file = layout_dir / f"page_{p:04d}-embed.md"

        if target_embed_file.exists() and not force:

            # Already embedded

            continue



        # Look for source markdown file with priority: -images-clip_final.md -> -images.md -> .md

        src_md_candidates = [

            layout_dir / f"page_{p:04d}-images-clip_final.md",

            layout_dir / f"page_{p:04d}-images.md",

            layout_dir / f"page_{p:04d}.md",

        ]

        source_md: Optional[Path] = None

        for cand in src_md_candidates:

            if cand.exists() and cand.stat().st_size > 0:

                source_md = cand

                break



        if not source_md:

            continue



        md_content = source_md.read_text(encoding="utf-8", errors="replace")

        page_updated = False



        for it in assets:

            asset_id = it.get("asset_id")

            gen_md_rel = it.get("generated_markdown")

            if not gen_md_rel:

                skipped_assets += 1

                continue



            full_gen_md = manual_dir / gen_md_rel

            if not full_gen_md.exists() or full_gen_md.stat().st_size == 0:

                skipped_assets += 1

                continue



            fragment_content = full_gen_md.read_text(encoding="utf-8", errors="replace").strip()

            # Ensure vertical breathing space after collapsible <details> in Markdown previews
            if re.search(r"</details>\s*$", fragment_content, re.IGNORECASE):
                fragment_content = fragment_content.rstrip() + "\n<br>"



            # Regex pattern to match image link for this asset

            # Matches ![...](assets/page_XXXX_crop_Y*.png)

            escaped_id = re.escape(asset_id)

            pattern = re.compile(rf"!\[[^\]]*\]\(assets/{escaped_id}[^\)]*\)")



            if pattern.search(md_content):

                md_content = pattern.sub(lambda _: fragment_content, md_content)

                it["is_embedded"] = True

                it["embedded_file"] = to_relative_posix(target_embed_file, manual_dir)

                embedded_assets += 1

                page_updated = True

            else:

                # Also check <crop ... /> tags just in case

                crop_pattern = re.compile(rf"<crop\s+[^>]*{escaped_id}[^>]*/>")

                if crop_pattern.search(md_content):

                    md_content = crop_pattern.sub(lambda _: fragment_content, md_content)

                    it["is_embedded"] = True

                    it["embedded_file"] = to_relative_posix(target_embed_file, manual_dir)

                    embedded_assets += 1

                    page_updated = True



        if page_updated or not assets:

            target_embed_file.write_text(md_content.strip() + "\n", encoding="utf-8")

            embedded_pages += 1

            processed_pages.add(p)



    # Refresh embed statistics

    embed_stats = data.get("embed_stats", {})

    embed_stats["embedded_assets"] = sum(1 for it in items if it.get("is_embedded") is True)

    embed_stats["pending_assets"] = len(items) - embed_stats["embedded_assets"]

    embed_stats["embedded_pages"] = len([p for p in layout_dir.glob("page_*-embed.md") if p.stat().st_size > 0])

    data["embed_stats"] = embed_stats



    save_queue(queue_path, data)

    return embedded_pages, embedded_assets, skipped_assets





def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 16: Embed Markdown.")
    parser.add_argument("manual", nargs="?", default=".", help="Path to manual directory.")
    parser.add_argument("--pages", type=str, default="", help="Page filter, e.g. '14,16' or '10-25'.")
    parser.add_argument("--force", action="store_true", help="Force overwrite of existing embed markdown files.")
    parser.add_argument("--status", action="store_true", help="Print embed status summary.")

    args = parser.parse_args()
    manual_dir, pdf_path, layout_dir, queue_path = resolve_manual_paths(Path(args.manual))

    if args.status:
        data = load_queue(queue_path)
        stats = data.get("embed_stats", {})
        print(f"Stage 16 Embedding Status for '{manual_dir.name}':")
        print(f"  - Embedded Assets: {stats.get('embedded_assets', 0)} / {stats.get('total_assets', 0)}")
        print(f"  - Embedded Pages:  {stats.get('embedded_pages', 0)} / {stats.get('total_pages', 0)}")
        return

    pages = parse_page_range(args.pages) if args.pages else None
    emb_pages, emb_assets, skipped = run_embedding(queue_path, manual_dir, target_pages=pages, force=args.force)
    print(f"Stage 16 Embedding Execution for '{manual_dir.name}':")
    print(f"  - Pages Generated: {emb_pages}")
    print(f"  - Assets Embedded: {emb_assets}")
    print(f"  - Skipped/Pending: {skipped}")





if __name__ == "__main__":

    main()

