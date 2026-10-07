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

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_runtime import LLM_CONCURRENCY

# Import CodexClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

from conversion import CodexClient
from common import pdf_schemas


def process_graphics(workspace_dir: Path, config: dict):
    input_dir = workspace_dir / "07_transform_tables"

    out_dir = workspace_dir / "08_transform_graphics"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in (path for path in out_dir.iterdir() if path.suffix == ".json"):
        f.unlink()

    out_assets_dir = out_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)
    for f in out_assets_dir.glob("*"):
        f.unlink()

    src_assets = workspace_dir / "07_transform_tables" / "assets"
    if src_assets.exists():
        for f in src_assets.glob("*"):
            if f.is_file():
                shutil.copy2(f, out_assets_dir / f.name)
    assets_dir = out_assets_dir

    with CodexClient(config, stage="08_transform_graphics", images=True) as codex:

        triage_prompt_path = Path(__file__).resolve().parent / "prompt_triage.md"
        triage_prompt = triage_prompt_path.read_text(encoding="utf-8")

        mermaid_prompt_path = Path(__file__).resolve().parent / "prompt_mermaid.md"
        mermaid_prompt = mermaid_prompt_path.read_text(encoding="utf-8")

        ascii_prompt_path = Path(__file__).resolve().parent / "prompt_ascii_art.md"
        ascii_prompt = ascii_prompt_path.read_text(encoding="utf-8")

        sidecar_prompt_path = Path(__file__).resolve().parent / "prompt_rag_sidecar.md"
        sidecar_prompt = sidecar_prompt_path.read_text(encoding="utf-8")

        from concurrent.futures import ThreadPoolExecutor

        concurrency = LLM_CONCURRENCY
        print(f"[*] Graphics Worker LLM active ({codex.selected.model}). Transforming graphics (concurrency={concurrency})...")

        chapter_files = sorted(list((path for path in input_dir.iterdir() if path.suffix == ".json")))
        print(f"[*] Transforming graphics across {len(chapter_files)} chapter files...")

        transformed_count = 0
        sidecar_count = 0

        def _transform_graphic_task(task):
            c_file, idx, node, has_dedicated_caption = task
            node_id = node.get("node_id", "asset")
            page_num = node.get("page", 1)
            raw_text = node.get("raw_text", "")

            svg_rel = node.get("svg_path")
            png_rel = node.get("png_path")
            asset_file = Path(svg_rel).name if svg_rel else (Path(png_rel).name if png_rel else f"asset_{node_id}.png")
            png_path = workspace_dir / png_rel if png_rel else None

            # First, classify with Codex if this is a flowchart, ascii_art (register/bitfield), or circuit schematic
            image_only = node.get("metadata", {}).get("image_only", False)
            triage = codex.generate_json(triage_prompt, image_path=png_path, schema=pdf_schemas.GRAPHIC_TRIAGE) if png_path and triage_prompt and not image_only else {}
            graphic_type = triage.get("type", "schematic")

            # Check for genuine figure caption from raw_text or separate caption nodes
            fig_match = re.search(r"(Figure\s+[A-Z0-9]+(?:[\-\.][A-Z0-9]+)?[:\s][^\n\r]+)", raw_text, re.IGNORECASE)
            genuine_caption = fig_match.group(1).strip() if fig_match else None
            if not genuine_caption and raw_text.strip():
                fig_lines = [l.strip() for l in raw_text.splitlines() if re.match(r"^Figure\s+[A-Z0-9]", l.strip(), re.IGNORECASE)]
                if fig_lines:
                    genuine_caption = fig_lines[0]
            if genuine_caption:
                genuine_caption = re.sub(r"[\[\]|]", "", genuine_caption)

            # Metadata title strictly for RAG sidecar (never injected as visible body text if absent from book)
            node_meta_caption = node.get("metadata", {}).get("caption")
            sidecar_title = genuine_caption or node_meta_caption or triage.get("caption") or f"Figure on page {page_num}"
            sidecar_title = re.sub(r"[\[\]|]", "", sidecar_title)

            if graphic_type == "mermaid" and png_path and mermaid_prompt:
                mermaid_res = codex.generate_vision(f"{mermaid_prompt}\n\nDiagram Labels:\n{raw_text}", png_path)
                if mermaid_res:
                    return c_file, idx, {
                        "rendered_markdown": mermaid_res.strip() + "\n\n",
                        "prune_image": True,
                        "sidecar_name": None,
                        "sidecar_text": None,
                    }

            if graphic_type == "ascii_art" and png_path and ascii_prompt:
                caption_hint = (
                    "A dedicated caption node already exists in the document text, do not output any caption line."
                    if has_dedicated_caption else
                    (f"Include the genuine figure caption below the ASCII diagram: *{genuine_caption}*" if genuine_caption else "No caption line was printed in the book, do not output any caption.")
                )
                full_ascii_prompt = f"{ascii_prompt}\n\nCaption Guideline: {caption_hint}\n\nExtracted Labels:\n{raw_text}"
                ascii_res = codex.generate_vision(full_ascii_prompt, png_path)
                if ascii_res:
                    return c_file, idx, {
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

            # Generate technical engineering sidecar via Codex Vision
            sidecar_name = f"{asset_file}.txt"
            sidecar_text = None
            if png_path and sidecar_prompt:
                sidecar_text = codex.generate_vision(f"{sidecar_prompt}\n\nExtracted Labels:\n{raw_text}", png_path)

            if not sidecar_text:
                sidecar_text = (
                    f"Title: {sidecar_title}\n"
                    f"Page: {page_num}\n"
                    f"Labels:\n{raw_text}\n"
                )

            return c_file, idx, {
                "rendered_markdown": rendered_md,
                "prune_image": False,
                "sidecar_name": sidecar_name,
                "sidecar_text": sidecar_text,
            }

        chapters = {}
        metadata = {}
        all_tasks = []

        for c_file in chapter_files:
            with open(c_file, "r", encoding="utf-8") as f:
                chapter = json.load(f)
                nodes = chapter["nodes"]
            chapters[c_file] = nodes
            metadata[c_file] = chapter

            for idx, node in enumerate(nodes):
                if node.get("type") != "graphic":
                    continue
                has_dedicated_caption = any(n.get("type") == "caption" and n.get("page") == node.get("page") for n in nodes)
                all_tasks.append((c_file, idx, node, has_dedicated_caption))

        if all_tasks:
            print(f"[*] Transforming {len(all_tasks)} graphics across {len(chapters)} chapters (concurrency={concurrency})...")
            with ThreadPoolExecutor(max_workers=min(len(all_tasks), concurrency)) as executor:
                results = list(executor.map(_transform_graphic_task, all_tasks))

            for c_file, idx, res in results:
                node = chapters[c_file][idx]
                node_id = node.get("node_id", "asset")
                node["rendered_markdown"] = res["rendered_markdown"]
                if res["prune_image"]:
                    for asset_f in out_assets_dir.glob(f"asset_{node_id}.*"):
                        asset_f.unlink()
                    node["png_path"] = None
                    node["svg_path"] = None
                    node["sidecar_path"] = None
                else:
                    node["sidecar_path"] = None
                transformed_count += 1

        for c_file, nodes in chapters.items():
            target_file = out_dir / c_file.name
            with open(target_file, "w", encoding="utf-8") as f:
                json.dump({**metadata[c_file], "nodes": nodes}, f, indent=2)

        print(f"[+] Stage 08 complete. Transformed {transformed_count} graphics, generated {sidecar_count} RAG sidecars in {out_dir}.")


def main():
    parser = argparse.ArgumentParser(description="Stage 08: Transform graphic nodes into Mermaid/Obsidian embeds with RAG sidecars")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)


    process_graphics(workspace_dir, config)


if __name__ == "__main__":
    main()
