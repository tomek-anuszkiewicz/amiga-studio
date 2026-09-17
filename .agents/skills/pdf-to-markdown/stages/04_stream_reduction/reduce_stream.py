#!/usr/bin/env python3
"""
stages/04_stream_reduction/reduce_stream.py:
Normalizes the sequential node stream:
1. Suppresses all header and footer nodes.
2. Identifies and unifies contiguous graphic fragments on the same page into a single diagram asset using Gemini Vision.
3. Welds consecutive prose nodes across page breaks and performs de-hyphenation.
4. Emits workspace/04_stream_reduction/reduced_stream.json.
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
    reduced_assets_dir: Path = None,
    concurrency: int = 8,
) -> tuple:
    """
    Identifies runs of contiguous graphic nodes on the same page.
    Validates with Vision LLM whether the candidate cluster forms a single unified diagram.
    Uses ThreadPoolExecutor to verify clusters concurrently across all pages.
    """
    pages_dir = workspace_dir / "01_preprocess"
    if reduced_assets_dir is None:
        reduced_assets_dir = workspace_dir / "04_stream_reduction" / "assets"
    reduced_assets_dir.mkdir(parents=True, exist_ok=True)

    # Pass 1: Identify all contiguous candidate runs on the same page
    runs = []
    i = 0
    while i < len(nodes):
        node = nodes[i]
        if node.get("type") == "graphic":
            run = [node]
            j = i + 1
            while j < len(nodes) and nodes[j].get("type") == "graphic" and nodes[j].get("page") == node.get("page"):
                run.append(nodes[j])
                j += 1
            if len(run) >= 2:
                runs.append((i, run))
            i = j
        else:
            i += 1

    # Prepare parallel LLM verification tasks
    llm_tasks = []
    for start_idx, run in runs:
        node = run[0]
        page_num = node["page"]
        min_norm_x0 = min(n["bbox_norm"][0] for n in run if n.get("bbox_norm"))
        min_norm_y0 = min(n["bbox_norm"][1] for n in run if n.get("bbox_norm"))
        max_norm_x1 = max(n["bbox_norm"][2] for n in run if n.get("bbox_norm"))
        max_norm_y1 = max(n["bbox_norm"][3] for n in run if n.get("bbox_norm"))
        union_bbox_norm = [round(min_norm_x0, 4), round(min_norm_y0, 4), round(max_norm_x1, 4), round(max_norm_y1, 4)]

        labels = [n.get("raw_text", "").strip() for n in run if n.get("raw_text", "").strip()]
        png_path = pages_dir / f"page_{page_num:04d}.png"

        if gemini and gemini.is_available() and png_path.exists() and graphics_prompt:
            prompt = (
                f"{graphics_prompt}\n\n"
                f"Page: {page_num}\n"
                f"Candidate Union Bounding Box (normalized [x0, y0, x1, y1]): {union_bbox_norm}\n"
                f"Extracted Labels inside cluster ({len(labels)} fragments):\n"
                + "\n".join(f"- {l}" for l in labels[:35])
            )
            llm_tasks.append((start_idx, prompt, png_path))

    cluster_evals = {}
    if llm_tasks:
        from concurrent.futures import ThreadPoolExecutor

        def _eval_cluster(task):
            start_idx, prompt, png_path = task
            res = gemini.generate_json(prompt, image_path=png_path, stage="04_stream_reduction")
            is_single = False
            res_title = ""
            if isinstance(res, dict) and res.get("is_single_graphic"):
                is_single = True
                res_title = res.get("title", "")
            return start_idx, is_single, res_title

        print(f"[*] Verifying {len(llm_tasks)} graphic clusters across stream (concurrency={concurrency})...")
        with ThreadPoolExecutor(max_workers=min(len(llm_tasks), concurrency)) as executor:
            results = list(executor.map(_eval_cluster, llm_tasks))
        cluster_evals = {start_idx: (is_single, res_title) for (start_idx, is_single, res_title) in results}

    # Pass 2: Reconstruct reduced_nodes strictly preserving original stream ordering
    candidate_map = {start_idx: run for start_idx, run in runs}
    reduced_nodes = []
    total_unifications = 0
    total_collapsed_nodes = 0
    i = 0

    while i < len(nodes):
        if i in candidate_map:
            run = candidate_map[i]
            is_single, res_title = cluster_evals.get(i, (False, ""))
            if is_single:
                first_node = run[0]
                page_num = first_node["page"]
                min_x0 = min(n["bbox"][0] for n in run if n.get("bbox"))
                min_y0 = min(n["bbox"][1] for n in run if n.get("bbox"))
                max_x1 = max(n["bbox"][2] for n in run if n.get("bbox"))
                max_y1 = max(n["bbox"][3] for n in run if n.get("bbox"))
                union_bbox = [round(min_x0, 2), round(min_y0, 2), round(max_x1, 2), round(max_y1, 2)]

                min_norm_x0 = min(n["bbox_norm"][0] for n in run if n.get("bbox_norm"))
                min_norm_y0 = min(n["bbox_norm"][1] for n in run if n.get("bbox_norm"))
                max_norm_x1 = max(n["bbox_norm"][2] for n in run if n.get("bbox_norm"))
                max_norm_y1 = max(n["bbox_norm"][3] for n in run if n.get("bbox_norm"))
                union_bbox_norm = [round(min_norm_x0, 4), round(min_norm_y0, 4), round(max_norm_x1, 4), round(max_norm_y1, 4)]

                labels = [n.get("raw_text", "").strip() for n in run if n.get("raw_text", "").strip()]
                title = res_title or (labels[-1] if labels else f"Figure on Page {page_num}")
                combined_text = "\n".join(labels)
                full_raw_text = f"{title}\n\n{combined_text}" if title and title not in combined_text else combined_text

                # 1. Delete old individual asset files in 04_stream_reduction/assets/
                if reduced_assets_dir.exists():
                    for old_node in run:
                        old_id = old_node.get("node_id")
                        if old_id:
                            for ext in [".png", ".txt", ".svg", ".png.txt"]:
                                f_asset = reduced_assets_dir / f"asset_{old_id}{ext}"
                                try:
                                    f_asset.unlink(missing_ok=True)
                                except Exception:
                                    pass

                # 2. Build single consolidated node
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

                # 3. Crop unified asset
                if extract_assets_for_nodes:
                    extract_assets_for_nodes(
                        workspace_dir,
                        [unified_node],
                        padding_ratio=padding_ratio,
                        assets_dir=reduced_assets_dir,
                        rel_prefix="04_stream_reduction/assets"
                    )

                print(f"    [+] Unified {len(run)} graphic nodes on page {page_num} into {unified_node['node_id']}: '{title}'")
                total_unifications += 1
                total_collapsed_nodes += (len(run) - 1)
                reduced_nodes.append(unified_node)
            else:
                reduced_nodes.extend(run)
            i += len(run)
        else:
            reduced_nodes.append(nodes[i])
            i += 1

    return reduced_nodes, total_unifications, total_collapsed_nodes


def reduce_contiguous_tables(
    nodes: list,
    workspace_dir: Path,
    padding_ratio: float = 0.10,
    reduced_assets_dir: Path = None,
) -> tuple:
    """
    Identifies runs of contiguous table nodes on the same page.
    Unifies them into a single consolidated table node, combining bounding boxes,
    concatenating underlying text, purging fragmented crop assets, and generating
    a single unified crop asset.
    """
    if reduced_assets_dir is None:
        reduced_assets_dir = workspace_dir / "04_stream_reduction" / "assets"
    reduced_assets_dir.mkdir(parents=True, exist_ok=True)

    reduced_nodes = []
    total_unifications = 0
    total_collapsed_nodes = 0
    i = 0

    while i < len(nodes):
        node = nodes[i]

        if node.get("type") == "table":
            # Detect contiguous run of table nodes on the same page
            run = [node]
            j = i + 1
            while j < len(nodes) and nodes[j].get("type") == "table" and nodes[j].get("page") == node.get("page"):
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

                first_node = run[0]
                combined_text = "\n\n".join(n.get("raw_text", "").strip() for n in run if n.get("raw_text", "").strip())

                # 1. Delete old individual asset files in 04_stream_reduction/assets/
                if reduced_assets_dir.exists():
                    for old_node in run:
                        old_id = old_node.get("node_id")
                        if old_id:
                            for ext in [".png", ".txt", ".svg", ".png.txt"]:
                                f_asset = reduced_assets_dir / f"asset_{old_id}{ext}"
                                try:
                                    f_asset.unlink(missing_ok=True)
                                except Exception:
                                    pass

                # 2. Build the single consolidated node
                unified_node = {
                    "node_id": first_node["node_id"],
                    "page": page_num,
                    "type": "table",
                    "heading_level": None,
                    "bbox": union_bbox,
                    "bbox_norm": union_bbox_norm,
                    "raw_text": combined_text,
                    "rendered_markdown": None,
                    "continuation_status": None,
                    "metadata": {
                        "unified_table": True,
                        "constituent_count": len(run)
                    },
                    "welded_nodes": [n["node_id"] for n in run]
                }

                # 3. Crop/extract the unified asset with safety margin into 04_stream_reduction/assets/
                if extract_assets_for_nodes:
                    extract_assets_for_nodes(
                        workspace_dir,
                        [unified_node],
                        padding_ratio=padding_ratio,
                        assets_dir=reduced_assets_dir,
                        rel_prefix="04_stream_reduction/assets"
                    )

                print(f"    [+] Unified {len(run)} table nodes on page {page_num} into {unified_node['node_id']}")
                total_unifications += 1
                total_collapsed_nodes += (len(run) - 1)

                reduced_nodes.append(unified_node)
                i = j
                continue

        # Single node (or non-table)
        reduced_nodes.append(node)
        i += 1

    return reduced_nodes, total_unifications, total_collapsed_nodes


def reduce_stream(workspace_dir: Path, config: dict):
    raw_stream_path = workspace_dir / "03_build_raw_stream" / "raw_stream.json"

    if not raw_stream_path.exists():
        raise FileNotFoundError(f"Missing raw_stream.json in {raw_stream_path.parent}")

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

    # Prepare Stage 04 assets directory by synchronizing from Stage 03
    import shutil
    out_dir = workspace_dir / "04_stream_reduction"
    out_dir.mkdir(parents=True, exist_ok=True)
    reduced_assets_dir = out_dir / "assets"
    reduced_assets_dir.mkdir(parents=True, exist_ok=True)
    raw_assets_dir = workspace_dir / "03_build_raw_stream" / "assets"

    for old_f in reduced_assets_dir.glob("*"):
        if old_f.is_file():
            old_f.unlink()
    if raw_assets_dir.exists():
        for asset_f in raw_assets_dir.glob("*"):
            if asset_f.is_file():
                shutil.copy2(asset_f, reduced_assets_dir / asset_f.name)

    for n in active_nodes:
        for k in ("png_path", "svg_path", "raw_text_path"):
            val = n.get(k)
            if val and "03_build_raw_stream/assets" in val:
                n[k] = val.replace("03_build_raw_stream/assets", "04_stream_reduction/assets")
            elif val and val.startswith("assets/"):
                n[k] = f"04_stream_reduction/{val}"

    # Step 2: Unify contiguous graphic nodes on identical pages
    concurrency = int(config.get("llm", {}).get("concurrency", 8))
    padding = config.get("render", {}).get("padding_margin_ratio", 0.10)
    nodes_after_graphics, num_unifications, num_collapsed_graphics = reduce_contiguous_graphics(
        active_nodes,
        workspace_dir,
        gemini,
        graphics_prompt_template,
        padding_ratio=padding,
        reduced_assets_dir=reduced_assets_dir,
        concurrency=concurrency,
    )
    if num_unifications > 0:
        print(f"[*] Graphic Reduction: Consolidated {num_collapsed_graphics + num_unifications} fragments into {num_unifications} unified diagram(s) (eliminated {num_collapsed_graphics} fragmented nodes).")

    # Step 2b: Unify contiguous table nodes on identical pages
    nodes_after_tables, num_table_unifications, num_collapsed_tables = reduce_contiguous_tables(
        nodes_after_graphics,
        workspace_dir,
        padding_ratio=padding,
        reduced_assets_dir=reduced_assets_dir,
    )
    if num_table_unifications > 0:
        print(f"[*] Table Reduction: Consolidated {num_collapsed_tables + num_table_unifications} fragments into {num_table_unifications} unified table(s) (eliminated {num_collapsed_tables} fragmented nodes).")

    # Step 3: Weld consecutive prose nodes and consecutive code_block nodes
    # Phase 3a: Fast same-page local welds in memory
    condensed = []
    welded_prose_count = 0
    welded_code_count = 0

    for node in nodes_after_tables:
        n_type = node.get("type")
        if condensed and condensed[-1]["type"] == "prose" and n_type == "prose":
            prev = condensed[-1]
            prev_page = prev.get("page_end", prev.get("page"))
            curr_page = node.get("page")
            if prev_page == curr_page:
                prev["raw_text"] = prev["raw_text"].rstrip() + "\n\n" + node["raw_text"].lstrip()
                prev["page_end"] = curr_page
                if "welded_nodes" not in prev:
                    prev["welded_nodes"] = [prev["node_id"]]
                prev["welded_nodes"].append(node["node_id"])
                welded_prose_count += 1
                continue

        if condensed and condensed[-1]["type"] == "code_block" and n_type == "code_block" and condensed[-1]["page"] == node["page"]:
            prev = condensed[-1]
            prev["raw_text"] = prev["raw_text"].rstrip() + "\n" + node["raw_text"].lstrip()
            if "welded_nodes" not in prev:
                prev["welded_nodes"] = [prev["node_id"]]
            prev["welded_nodes"].append(node["node_id"])
            if prev.get("bbox") and node.get("bbox"):
                prev["bbox"] = [
                    round(min(prev["bbox"][0], node["bbox"][0]), 2),
                    round(min(prev["bbox"][1], node["bbox"][1]), 2),
                    round(max(prev["bbox"][2], node["bbox"][2]), 2),
                    round(max(prev["bbox"][3], node["bbox"][3]), 2),
                ]
            welded_code_count += 1
            continue

        node_copy = dict(node)
        if "page_start" not in node_copy:
            node_copy["page_start"] = node["page"]
        if "page_end" not in node_copy:
            node_copy["page_end"] = node["page"]
        condensed.append(node_copy)

    # Phase 3b: Evaluate cross-page prose seams concurrently
    seam_tasks = []
    for k in range(len(condensed) - 1):
        if condensed[k]["type"] == "prose" and condensed[k + 1]["type"] == "prose":
            t1 = condensed[k]["raw_text"]
            t2 = condensed[k + 1]["raw_text"]
            seam_tasks.append((k, t1, t2))

    seam_results = {}
    if seam_tasks and gemini and gemini.is_available() and seam_prompt_template:
        from concurrent.futures import ThreadPoolExecutor

        def _eval_seam_task(task):
            idx, text1, text2 = task
            welded = weld_prose_with_gemini(text1, text2, gemini, seam_prompt_template)
            return idx, welded

        print(f"[*] Evaluating {len(seam_tasks)} cross-page prose seams (concurrency={concurrency})...")
        with ThreadPoolExecutor(max_workers=min(len(seam_tasks), concurrency)) as executor:
            seam_results = dict(executor.map(_eval_seam_task, seam_tasks))

    final_nodes = []
    idx = 0
    while idx < len(condensed):
        cur_node = condensed[idx]
        if idx in seam_results:
            welded_text = seam_results[idx]
            next_node = condensed[idx + 1]
            cur_node["raw_text"] = welded_text
            cur_node["page_end"] = next_node.get("page_end", next_node.get("page"))
            if "welded_nodes" not in cur_node:
                cur_node["welded_nodes"] = [cur_node["node_id"]]
            cur_node["welded_nodes"].extend(next_node.get("welded_nodes", [next_node["node_id"]]))
            welded_prose_count += 1
            final_nodes.append(cur_node)
            idx += 2
        else:
            final_nodes.append(cur_node)
            idx += 1

    if welded_prose_count > 0:
        print(f"[*] Prose Welding: Welded {welded_prose_count} consecutive prose segments.")
    if welded_code_count > 0:
        print(f"[*] Code Welding: Welded {welded_code_count} consecutive code block segments.")

    out_dir = workspace_dir / "04_stream_reduction"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Prune orphaned asset files that do not belong to any active node in final_nodes
    reduced_assets_dir = out_dir / "assets"
    if reduced_assets_dir.exists():
        active_node_ids = {n["node_id"] for n in final_nodes}
        for f in list(reduced_assets_dir.glob("asset_*")):
            m = re.match(r"asset_(node_\d+)", f.name)
            if m and m.group(1) not in active_node_ids:
                try:
                    f.unlink(missing_ok=True)
                except Exception:
                    pass

    reduced_stream_path = out_dir / "reduced_stream.json"
    with open(reduced_stream_path, "w", encoding="utf-8") as f:
        json.dump(final_nodes, f, indent=2)

    print(f"[+] Stage 04 complete. Stream reduced from {len(raw_nodes)} -> {len(final_nodes)} nodes in {reduced_stream_path}")


def main():
    parser = argparse.ArgumentParser(description="Stage 04: Stream reduction, graphic unification, and prose welding")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 04: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 04: Config file is empty or invalid: {config_path}")

    reduce_stream(workspace_dir, config)


if __name__ == "__main__":
    main()
