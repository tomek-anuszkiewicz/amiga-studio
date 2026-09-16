#!/usr/bin/env python3
"""
stages/08_transform_graphics/transform_graphics.py:
Worker for graphic nodes across chapter streams:
1. Triages diagrams vs complex schematics/waveforms.
2. Converts state machines and flowcharts to Mermaid + collapsible ASCII callouts (> [!NOTE]-).
3. Preserves complex schematics as Obsidian wikilinks: ![[assets/{id}.svg]] (or .png).
4. Generates engineering sidecars in workspace/assets/{id}.png.txt for vector RAG search.
"""

import argparse
import json
import re
import shutil
import sys
from pathlib import Path
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def generate_default_sidecar(node_id: str, raw_text: str, page_num: int) -> str:
    lines = [l.strip() for l in raw_text.splitlines() if l.strip()]
    labels = "\n".join(f"- {l}" for l in lines) if lines else "- (No OCR labels detected)"
    return (
        f"Title: Technical Schematic / Circuit Diagram (node_{node_id})\n"
        f"Source Page: {page_num}\n"
        f"Labels & Text Elements:\n{labels}\n"
    )


def process_graphics(workspace_dir: Path, config: dict):
    input_candidates = [
        workspace_dir / "07_transform_tables",
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "08_transform_graphics"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in out_dir.glob("*.json"):
        f.unlink()

    out_assets_dir = out_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)
    for f in out_assets_dir.glob("*"):
        try:
            f.unlink()
        except Exception:
            pass

    asset_candidates = [
        workspace_dir / "07_transform_tables" / "assets",
        workspace_dir / "06_detect_continuations" / "assets",
        workspace_dir / "05_chapter_partition" / "assets",
        workspace_dir / "04_stream_reduction" / "assets",
        workspace_dir / "03_build_raw_stream" / "assets",
    ]
    src_assets = next((p for p in asset_candidates if p.exists()), None)
    if src_assets:
        for f in src_assets.glob("*"):
            if f.is_file():
                shutil.copy2(f, out_assets_dir / f.name)
    assets_dir = out_assets_dir

    gemini = GeminiClient(config) if GeminiClient else None
    if not gemini or not gemini.is_available():
        raise RuntimeError("GEMINI_API_KEY environment variable is required for Stage 08 graphics transformation.")

    triage_prompt_path = Path(__file__).resolve().parent / "prompt_triage.md"
    triage_prompt = triage_prompt_path.read_text(encoding="utf-8") if triage_prompt_path.exists() else ""

    mermaid_prompt_path = Path(__file__).resolve().parent / "prompt_mermaid.md"
    mermaid_prompt = mermaid_prompt_path.read_text(encoding="utf-8") if mermaid_prompt_path.exists() else ""

    ascii_prompt_path = Path(__file__).resolve().parent / "prompt_ascii_art.md"
    ascii_prompt = ascii_prompt_path.read_text(encoding="utf-8") if ascii_prompt_path.exists() else ""

    sidecar_prompt_path = Path(__file__).resolve().parent / "prompt_rag_sidecar.md"
    sidecar_prompt = sidecar_prompt_path.read_text(encoding="utf-8") if sidecar_prompt_path.exists() else ""

    from concurrent.futures import ThreadPoolExecutor

    concurrency = int(config.get("llm", {}).get("concurrency", 8))
    print(f"[*] Graphics Worker LLM active ({gemini.vision_model}). Transforming graphics (concurrency={concurrency})...")

    chapter_files = sorted(list(input_dir.glob("*.json")))
    print(f"[*] Transforming graphics across {len(chapter_files)} chapter files...")

    transformed_count = 0
    sidecar_count = 0

    def _transform_graphic_task(task):
        idx, node, has_dedicated_caption = task
        node_id = node.get("node_id", "asset")
        page_num = node.get("page", 1)
        raw_text = node.get("raw_text", "")

        svg_rel = node.get("svg_path")
        png_rel = node.get("png_path")
        asset_file = Path(svg_rel).name if svg_rel else (Path(png_rel).name if png_rel else f"asset_{node_id}.png")
        png_path = workspace_dir / png_rel if png_rel else None

        # First, classify with Gemini if this is a flowchart, ascii_art (register/bitfield), or circuit schematic
        triage = gemini.generate_json(triage_prompt, image_path=png_path, stage="08_transform_graphics") if png_path and png_path.exists() and triage_prompt else {}
        graphic_type = triage.get("type", "schematic") if isinstance(triage, dict) else "schematic"

        # Check for genuine figure caption from raw_text or separate caption nodes
        fig_match = re.search(r"(Figure\s+\d+[\-\.]\d+[:\s][^\n\r]+)", raw_text, re.IGNORECASE)
        genuine_caption = fig_match.group(1).strip() if fig_match else None
        if not genuine_caption and raw_text.strip():
            fig_lines = [l.strip() for l in raw_text.splitlines() if l.strip().lower().startswith("figure")]
            if fig_lines:
                genuine_caption = fig_lines[0]
        if genuine_caption:
            genuine_caption = re.sub(r"[\[\]|]", "", genuine_caption)

        # Metadata title strictly for RAG sidecar (never injected as visible body text if absent from book)
        sidecar_title = genuine_caption or (triage.get("caption") if isinstance(triage, dict) else None) or f"Figure on page {page_num}"
        sidecar_title = re.sub(r"[\[\]|]", "", sidecar_title)

        if graphic_type == "mermaid" and png_path and png_path.exists() and mermaid_prompt:
            mermaid_res = gemini.generate_vision(f"{mermaid_prompt}\n\nDiagram Labels:\n{raw_text}", png_path, stage="08_transform_graphics")
            if mermaid_res:
                return idx, {
                    "rendered_markdown": mermaid_res.strip() + "\n\n",
                    "prune_image": True,
                    "sidecar_name": None,
                    "sidecar_text": None,
                }

        if graphic_type == "ascii_art" and png_path and png_path.exists() and ascii_prompt:
            caption_hint = (
                "A dedicated caption node already exists in the document text, do not output any caption line."
                if has_dedicated_caption else
                (f"Include the genuine figure caption below the ASCII diagram: *{genuine_caption}*" if genuine_caption else "No caption line was printed in the book, do not output any caption.")
            )
            full_ascii_prompt = f"{ascii_prompt}\n\nCaption Guideline: {caption_hint}\n\nExtracted Labels:\n{raw_text}"
            ascii_res = gemini.generate_vision(full_ascii_prompt, png_path, stage="08_transform_graphics")
            if ascii_res:
                return idx, {
                    "rendered_markdown": ascii_res.strip() + "\n\n",
                    "prune_image": True,
                    "sidecar_name": None,
                    "sidecar_text": None,
                }

        # Fallback to Obsidian image embed + RAG sidecar
        if genuine_caption:
            if has_dedicated_caption:
                rendered_md = f"![[{asset_file}|{genuine_caption}]]\n\n"
            else:
                rendered_md = f"![[{asset_file}|{genuine_caption}]]\n\n*{genuine_caption}*\n\n"
        else:
            # No genuine caption in book: emit clean embed with zero artificial caption
            rendered_md = f"![[{asset_file}]]\n\n"

        # Generate technical engineering sidecar via Gemini Vision
        sidecar_name = f"{asset_file}.txt"
        sidecar_text = None
        if png_path and png_path.exists() and sidecar_prompt:
            sidecar_text = gemini.generate_vision(f"{sidecar_prompt}\n\nExtracted Labels:\n{raw_text}", png_path, stage="08_transform_graphics")

        if not sidecar_text:
            sidecar_text = (
                f"Title: {sidecar_title}\n"
                f"Page: {page_num}\n"
                f"Labels:\n{raw_text}\n"
            )

        return idx, {
            "rendered_markdown": rendered_md,
            "prune_image": False,
            "sidecar_name": sidecar_name,
            "sidecar_text": sidecar_text,
        }

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        tasks = []
        for idx, node in enumerate(nodes):
            if node.get("type") != "graphic":
                continue
            has_dedicated_caption = any(n.get("type") == "caption" and n.get("page") == node.get("page") for n in nodes)
            tasks.append((idx, node, has_dedicated_caption))

        if tasks:
            with ThreadPoolExecutor(max_workers=min(len(tasks), concurrency)) as executor:
                results = list(executor.map(_transform_graphic_task, tasks))

            for idx, res in results:
                node = nodes[idx]
                node_id = node.get("node_id", "asset")
                node["rendered_markdown"] = res["rendered_markdown"]
                if res["prune_image"]:
                    for asset_f in out_assets_dir.glob(f"asset_{node_id}.*"):
                        try:
                            asset_f.unlink()
                        except Exception:
                            pass
                    node["png_path"] = None
                    node["svg_path"] = None
                    node["sidecar_path"] = None
                else:
                    sidecar_name = res["sidecar_name"]
                    sidecar_text = res["sidecar_text"]
                    sidecar_path = assets_dir / sidecar_name
                    with open(sidecar_path, "w", encoding="utf-8") as sf:
                        sf.write(sidecar_text)
                    node["sidecar_path"] = f"{assets_dir.relative_to(workspace_dir).as_posix()}/{sidecar_name}"
                    sidecar_count += 1
                transformed_count += 1

        target_file = out_dir / c_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

    print(f"[+] Stage 08 complete. Transformed {transformed_count} graphics, generated {sidecar_count} RAG sidecars in {out_dir}.")


def prepare_graphics_tasks(workspace_dir: Path) -> int:
    """
    Extracts graphic nodes into workspace/tasks/graphics/{node_id}.json and {node_id}.md
    for the Agent to inspect image assets and author Mermaid or sidecars.
    """
    input_candidates = [
        workspace_dir / "07_transform_tables",
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    chapters_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not chapters_dir:
        raise FileNotFoundError(f"Missing chapters directory: {chapters_dir}")

    tasks_dir = workspace_dir / "tasks" / "graphics"
    tasks_dir.mkdir(parents=True, exist_ok=True)

    count = 0
    for c_file in sorted(list(chapters_dir.glob("*.json"))):
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        for node in nodes:
            if node.get("type") != "graphic":
                continue

            node_id = node.get("node_id", "asset")
            page_num = node.get("page", 1)
            raw_text = node.get("raw_text", "")

            svg_rel = node.get("svg_path")
            png_rel = node.get("png_path")
            asset_file = Path(svg_rel).name if svg_rel else (Path(png_rel).name if png_rel else f"asset_{node_id}.png")

            fig_match = re.search(r"(Figure\s+\d+[\-\.]\d+[:\s][^\n\r]+)", raw_text, re.IGNORECASE)
            genuine_caption = fig_match.group(1).strip() if fig_match else None
            if not genuine_caption and raw_text.strip():
                fig_lines = [l.strip() for l in raw_text.splitlines() if l.strip().lower().startswith("figure")]
                if fig_lines:
                    genuine_caption = fig_lines[0]
            if genuine_caption:
                genuine_caption = re.sub(r"[\[\]|]", "", genuine_caption)

            has_dedicated_caption = any(n.get("type") == "caption" and n.get("page") == node.get("page") for n in nodes)
            if genuine_caption:
                if has_dedicated_caption:
                    draft_md = f"![[{asset_file}|{genuine_caption}]]\n"
                else:
                    draft_md = f"![[{asset_file}|{genuine_caption}]]\n\n*{genuine_caption}*\n"
            else:
                draft_md = f"![[{asset_file}]]\n"
            draft_sidecar = generate_default_sidecar(node_id, raw_text, page_num)

            task_meta = {
                "node_id": node_id,
                "chapter_file": c_file.name,
                "page": page_num,
                "svg_path": svg_rel,
                "png_path": png_rel,
                "asset_file": asset_file,
                "raw_text": raw_text,
            }

            meta_file = tasks_dir / f"{node_id}.json"
            md_file = tasks_dir / f"{node_id}.md"
            sidecar_file = tasks_dir / f"{node_id}.sidecar.txt"

            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump(task_meta, f, indent=2)

            if not md_file.exists():
                with open(md_file, "w", encoding="utf-8") as f:
                    f.write(draft_md)

            if not sidecar_file.exists():
                with open(sidecar_file, "w", encoding="utf-8") as f:
                    f.write(draft_sidecar)

            count += 1

    print(f"[+] Prepared {count} graphic tasks in {tasks_dir}")
    return count


def apply_graphics_tasks(workspace_dir: Path) -> int:
    """
    Reads workspace/tasks/graphics/{node_id}.md and sidecars,
    updating workspace/08_transform_graphics/ and workspace/08_transform_graphics/assets/.
    """
    tasks_dir = workspace_dir / "tasks" / "graphics"
    out_dir = workspace_dir / "08_transform_graphics"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_assets_dir = out_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)

    asset_candidates = [
        workspace_dir / "07_transform_tables" / "assets",
        workspace_dir / "06_detect_continuations" / "assets",
        workspace_dir / "05_chapter_partition" / "assets",
        workspace_dir / "04_stream_reduction" / "assets",
        workspace_dir / "03_build_raw_stream" / "assets",
    ]
    src_assets = next((p for p in asset_candidates if p.exists()), None)
    if src_assets:
        for f in src_assets.glob("*"):
            if f.is_file():
                shutil.copy2(f, out_assets_dir / f.name)
    assets_dir = out_assets_dir

    if not tasks_dir.exists():
        print(f"[!] No graphic tasks directory found at {tasks_dir}")
        return 0

    input_candidates = [
        workspace_dir / "07_transform_tables",
        workspace_dir / "06_detect_continuations",
        workspace_dir / "05_chapter_partition",
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    # Collect rendered markdown and sidecars
    rendered_by_node = {}
    for meta_file in sorted(list(tasks_dir.glob("*.json"))):
        with open(meta_file, "r", encoding="utf-8") as f:
            meta = json.load(f)

        node_id = meta.get("node_id")
        asset_file = meta.get("asset_file", f"asset_{node_id}.png")
        md_file = tasks_dir / f"{node_id}.md"
        sidecar_file = tasks_dir / f"{node_id}.sidecar.txt"

        if md_file.exists():
            with open(md_file, "r", encoding="utf-8") as f:
                rendered_by_node[node_id] = f.read().strip()

        sidecar_name = f"{asset_file}.txt"
        if sidecar_file.exists():
            sidecar_dest = assets_dir / sidecar_name
            with open(sidecar_file, "r", encoding="utf-8") as sf:
                sidecar_dest.write_text(sf.read(), encoding="utf-8")

    applied_count = 0
    for c_file in sorted(list(input_dir.glob("*.json"))):
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        for node in nodes:
            n_id = node.get("node_id")
            if n_id in rendered_by_node:
                content = rendered_by_node[n_id]
                node["rendered_markdown"] = content
                if "```mermaid" in content or f"asset_{n_id}" not in content:
                    for asset_f in out_assets_dir.glob(f"asset_{n_id}.*"):
                        try:
                            asset_f.unlink()
                        except Exception:
                            pass
                    node["png_path"] = None
                    node["svg_path"] = None
                    node["sidecar_path"] = None
                else:
                    asset_f = meta.get("asset_file", f"asset_{n_id}.png")
                    node["sidecar_path"] = f"{assets_dir.relative_to(workspace_dir).as_posix()}/{asset_f}.txt"
                applied_count += 1

        target_file = out_dir / c_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

    print(f"[+] Applied {applied_count} graphic tasks to {out_dir}.")
    return applied_count


def main():
    parser = argparse.ArgumentParser(description="Stage 08: Transform graphic nodes into Mermaid/Obsidian embeds with RAG sidecars")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--prepare", action="store_true", help="Prepare graphic tasks for the Agent in workspace/tasks/graphics/")
    parser.add_argument("--apply", action="store_true", help="Apply Agent's edited graphics and sidecars back to chapters")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    if args.prepare:
        prepare_graphics_tasks(workspace_dir)
        return

    if args.apply:
        apply_graphics_tasks(workspace_dir)
        return

    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_graphics(workspace_dir, config)


if __name__ == "__main__":
    main()
