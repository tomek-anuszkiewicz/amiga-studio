#!/usr/bin/env python3
"""
stages/04_stream_reduction/reduce_stream.py:
Normalizes the sequential node stream:
1. Suppresses all header and footer nodes.
2. Identifies and unifies contiguous graphic fragments on the same page into a single diagram asset using Gemini Vision.
3. Welds consecutive prose nodes across page breaks and performs de-hyphenation.
4. Emits workspace/04_reduced_stream/reduced_stream.json.
"""

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Optional
import yaml

# Import GeminiClient and extract_assets_for_nodes
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

STAGE3_DIR = SKILL_ROOT / "stages" / "03_build_raw_stream"
if str(STAGE3_DIR) not in sys.path:
    sys.path.insert(0, str(STAGE3_DIR))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None

try:
    from extract_initial_assets import extract_assets_for_nodes
except ImportError:
    extract_assets_for_nodes = None


def weld_prose_with_gemini(text1: str, text2: str, gemini: Optional[GeminiClient], prompt_template: str) -> str:
    """
    Uses Gemini LLM to evaluate cross-page paragraph continuation and perform accurate de-hyphenation.
    """
    t1 = text1.rstrip()
    t2 = text2.lstrip()

    if gemini and gemini.is_available() and prompt_template:
        prompt = (
            f"{prompt_template}\n\n"
            f"## Tail Text of Preceding Page:\n```text\n{t1[-300:]}\n```\n\n"
            f"## Head Text of Next Page:\n```text\n{t2[:300]}\n```\n"
        )
        res = gemini.generate_json(prompt)
        if isinstance(res, dict) and res.get("is_continuation"):
            dehyphen = res.get("de_hyphenated_word")
            if dehyphen and "-" in t1[-12:]:
                base1 = re.sub(r"\b\w+-\s*$", "", t1)
                rest2 = re.sub(r"^\w+\s*", "", t2)
                return f"{base1}{dehyphen} {rest2}"
            return f"{t1} {t2}"

    # Default clean separation
    return f"{t1}\n\n{t2}"


def reduce_contiguous_graphics(
    nodes: list,
    workspace_dir: Path,
    gemini: Optional[GeminiClient],
    graphics_prompt: str,
    padding_ratio: float = 0.10,
) -> tuple:
    """
    Scans sequential nodes for contiguous runs of graphic nodes on the same page.
    Queries Gemini Vision with the page PNG and the bounding box union.
    If verified as a single cohesive graphic:
      1. Purges obsolete individual asset files from workspace/assets/.
      2. Crops a single unified asset (PNG/SVG) with 10% safety margin.
      3. Replaces the entire sequence of old nodes with a single unified graphic node.
    """
    pages_dir = workspace_dir / "01_pages" if (workspace_dir / "01_pages").exists() else (workspace_dir / "pages")
    assets_dir = workspace_dir / "assets"

    reduced_nodes = []
    total_unifications = 0
    total_collapsed_nodes = 0
    i = 0

    while i < len(nodes):
        node = nodes[i]

        if node.get("type") == "graphic":
            # Detect contiguous run of graphic nodes on the same page
            run = [node]
            j = i + 1
            while j < len(nodes) and nodes[j].get("type") == "graphic" and nodes[j].get("page") == node.get("page"):
                run.append(nodes[j])
                j += 1

            if len(run) >= 2:
                page_num = node["page"]
                # Compute bounding box union in points
                min_x0 = min(n["bbox"][0] for n in run if n.get("bbox"))
                min_y0 = min(n["bbox"][1] for n in run if n.get("bbox"))
                max_x1 = max(n["bbox"][2] for n in run if n.get("bbox"))
                max_y1 = max(n["bbox"][3] for n in run if n.get("bbox"))
                union_bbox = [round(min_x0, 2), round(min_y0, 2), round(max_x1, 2), round(max_y1, 2)]

                # Compute normalized bounding box union
                min_norm_x0 = min(n["bbox_norm"][0] for n in run if n.get("bbox_norm"))
                min_norm_y0 = min(n["bbox_norm"][1] for n in run if n.get("bbox_norm"))
                max_norm_x1 = max(n["bbox_norm"][2] for n in run if n.get("bbox_norm"))
                max_norm_y1 = max(n["bbox_norm"][3] for n in run if n.get("bbox_norm"))
                union_bbox_norm = [round(min_norm_x0, 4), round(min_norm_y0, 4), round(max_norm_x1, 4), round(max_norm_y1, 4)]

                labels = [n.get("raw_text", "").strip() for n in run if n.get("raw_text", "").strip()]
                png_path = pages_dir / f"page_{page_num:04d}.png"

                is_single = False
                res_title = ""

                if gemini and gemini.is_available() and png_path.exists() and graphics_prompt:
                    prompt = (
                        f"{graphics_prompt}\n\n"
                        f"Page: {page_num}\n"
                        f"Candidate Union Bounding Box (normalized [x0, y0, x1, y1]): {union_bbox_norm}\n"
                        f"Extracted Labels inside cluster ({len(labels)} fragments):\n"
                        + "\n".join(f"- {l}" for l in labels[:35])
                    )
                    res = gemini.generate_json(prompt, image_path=png_path)
                    if isinstance(res, dict) and res.get("is_single_graphic"):
                        is_single = True
                        res_title = res.get("title", "")

                if is_single:
                    first_node = run[0]
                    title = res_title or (labels[-1] if labels else f"Figure on Page {page_num}")
                    combined_text = "\n".join(labels)
                    full_raw_text = f"{title}\n\n{combined_text}" if title and title not in combined_text else combined_text

                    # 1. Delete old individual asset files in workspace/assets/
                    if assets_dir.exists():
                        for old_node in run:
                            old_id = old_node.get("node_id")
                            if old_id:
                                for ext in [".png", ".txt", ".svg", ".png.txt"]:
                                    f_asset = assets_dir / f"asset_{old_id}{ext}"
                                    try:
                                        f_asset.unlink(missing_ok=True)
                                    except Exception:
                                        pass

                    # 2. Build the single consolidated node
                    unified_node = {
                        "node_id": first_node["node_id"],
                        "page": page_num,
                        "type": "graphic",
                        "heading_level": None,
                        "bbox": union_bbox,
                        "bbox_norm": union_bbox_norm,
                        "raw_text": full_raw_text,
                        "rendered_markdown": None,
                        "continuation_status": None,
                        "metadata": {
                            "unified_graphic": True,
                            "title": title,
                            "constituent_count": len(run)
                        },
                        "welded_nodes": [n["node_id"] for n in run]
                    }

                    # 3. Crop/extract the unified asset with safety margin
                    if extract_assets_for_nodes:
                        extract_assets_for_nodes(workspace_dir, [unified_node], padding_ratio=padding_ratio)

                    print(f"    [+] Unified {len(run)} graphic nodes on page {page_num} into {unified_node['node_id']}: '{title}'")
                    total_unifications += 1
                    total_collapsed_nodes += (len(run) - 1)

                    # 4. Replace the entire old sequence with ONLY the unified node (deleting old nodes from stream)
                    reduced_nodes.append(unified_node)
                    i = j
                    continue
                else:
                    # Not verified as a single graphic: retain individual nodes
                    reduced_nodes.extend(run)
                    i = j
                    continue

        # Single node (or non-graphic)
        reduced_nodes.append(node)
        i += 1

    return reduced_nodes, total_unifications, total_collapsed_nodes


def reduce_stream(workspace_dir: Path, config: dict):
    raw_stream_path = workspace_dir / "03_raw_stream" / "raw_stream.json"
    if not raw_stream_path.exists():
        raise FileNotFoundError(f"Missing raw_stream.json in {workspace_dir / '03_raw_stream'}")

    seam_prompt_path = Path(__file__).resolve().parent / "prompt_seam.md"
    seam_prompt_template = seam_prompt_path.read_text(encoding="utf-8") if seam_prompt_path.exists() else ""

    graphics_prompt_path = Path(__file__).resolve().parent / "prompt_graphics_union.md"
    graphics_prompt_template = graphics_prompt_path.read_text(encoding="utf-8") if graphics_prompt_path.exists() else ""

    gemini = GeminiClient(config) if GeminiClient else None
    if not gemini or not gemini.is_available():
        raise RuntimeError("GEMINI_API_KEY environment variable is required for Stage 04 stream reduction.")

    with open(raw_stream_path, "r", encoding="utf-8") as f:
        raw_nodes = json.load(f)

    print(f"[*] Reducing stream of {len(raw_nodes)} nodes from {raw_stream_path.name}...")

    # Step 1: Suppress headers and footers
    active_nodes = []
    skipped_count = 0
    for node in raw_nodes:
        if node.get("type") in ("header", "footer"):
            skipped_count += 1
        else:
            active_nodes.append(node)

    print(f"[*] Suppressed {skipped_count} header/footer nodes.")

    # Step 2: Unify contiguous graphic nodes on identical pages
    padding = config.get("render", {}).get("padding_margin_ratio", 0.10)
    nodes_after_graphics, num_unifications, num_collapsed_graphics = reduce_contiguous_graphics(
        active_nodes,
        workspace_dir,
        gemini,
        graphics_prompt_template,
        padding_ratio=padding,
    )
    if num_unifications > 0:
        print(f"[*] Graphic Reduction: Consolidated {num_collapsed_graphics + num_unifications} fragments into {num_unifications} unified diagram(s) (eliminated {num_collapsed_graphics} fragmented nodes).")

    # Step 3: Weld consecutive prose + prose nodes across page breaks
    final_nodes = []
    welded_prose_count = 0
    for node in nodes_after_graphics:
        n_type = node.get("type")
        if final_nodes and final_nodes[-1]["type"] == "prose" and n_type == "prose":
            prev = final_nodes[-1]
            prev["raw_text"] = weld_prose_with_gemini(prev["raw_text"], node["raw_text"], gemini, seam_prompt_template)
            prev["page_end"] = node["page"]
            if "welded_nodes" not in prev:
                prev["welded_nodes"] = [prev["node_id"]]
            prev["welded_nodes"].append(node["node_id"])
            welded_prose_count += 1
            continue

        node_copy = dict(node)
        if "page_start" not in node_copy:
            node_copy["page_start"] = node["page"]
        if "page_end" not in node_copy:
            node_copy["page_end"] = node["page"]
        final_nodes.append(node_copy)

    if welded_prose_count > 0:
        print(f"[*] Prose Welding: Welded {welded_prose_count} consecutive prose segments.")

    out_dir = workspace_dir / "04_reduced_stream"
    out_dir.mkdir(parents=True, exist_ok=True)
    reduced_stream_path = out_dir / "reduced_stream.json"
    with open(reduced_stream_path, "w", encoding="utf-8") as f:
        json.dump(final_nodes, f, indent=2)

    print(f"[+] Stage 04 complete. Stream reduced from {len(raw_nodes)} -> {len(final_nodes)} nodes in {reduced_stream_path}")


def main():
    parser = argparse.ArgumentParser(description="Stage 04: Stream reduction, graphic unification, and prose welding")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    reduce_stream(workspace_dir, config)


if __name__ == "__main__":
    main()
