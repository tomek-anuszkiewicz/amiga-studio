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
    title = lines[0] if lines else f"Technical Diagram on Page {page_num}"
    body = "\n".join(lines[1:]) if len(lines) > 1 else raw_text

    return (
        f"Identifier: {node_id}\n"
        f"Title: {title}\n"
        f"Page: {page_num}\n"
        f"Extracted Labels & Details:\n{body}\n"
    )


def process_graphics(workspace_dir: Path, config: dict):
    input_candidates = [
        workspace_dir / "07_chapters_tables",
        workspace_dir / "06_chapters_continuations",
        workspace_dir / "05_chapters_raw",
        workspace_dir / "chapters"
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "08_chapters_graphics"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in out_dir.glob("*.json"):
        f.unlink()
    assets_dir = workspace_dir / "assets"
    assets_dir.mkdir(parents=True, exist_ok=True)

    gemini = GeminiClient(config) if GeminiClient else None
    if not gemini or not gemini.is_available():
        raise RuntimeError("GEMINI_API_KEY environment variable is required for Stage 08 graphics transformation.")

    mermaid_prompt_path = Path(__file__).resolve().parent / "prompt_mermaid.md"
    mermaid_prompt = mermaid_prompt_path.read_text(encoding="utf-8") if mermaid_prompt_path.exists() else ""

    sidecar_prompt_path = Path(__file__).resolve().parent / "prompt_rag_sidecar.md"
    sidecar_prompt = sidecar_prompt_path.read_text(encoding="utf-8") if sidecar_prompt_path.exists() else ""

    print(f"[*] Graphics Worker LLM active ({gemini.vision_model}). Transforming graphics...")

    chapter_files = sorted(list(input_dir.glob("*.json")))
    print(f"[*] Transforming graphics across {len(chapter_files)} chapter files...")

    transformed_count = 0
    sidecar_count = 0

    for c_file in chapter_files:
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
            png_path = workspace_dir / png_rel if png_rel else None

            # First, classify with Gemini if this is a flowchart/state machine or circuit schematic
            triage_prompt = (
                "Analyze this technical graphic. Is it a flowchart, state diagram, or structural block chart "
                "that should be converted to Mermaid code? Or is it a detailed circuit schematic, timing waveform, "
                "IC pinout, or photographic illustration that must be preserved as an image? "
                "Return a strict JSON object: {\"type\": \"mermaid\" | \"schematic\", \"caption\": \"Short descriptive title\"}"
            )
            triage = gemini.generate_json(triage_prompt, image_path=png_path) if png_path and png_path.exists() else {}
            graphic_type = triage.get("type", "schematic") if isinstance(triage, dict) else "schematic"
            caption = (triage.get("caption") if isinstance(triage, dict) else None) or (raw_text.splitlines()[0].strip() if raw_text.strip() else f"Figure on page {page_num}")
            caption = re.sub(r"[\[\]|]", "", caption)

            if graphic_type == "mermaid" and png_path and png_path.exists() and mermaid_prompt:
                mermaid_res = gemini.generate_vision(f"{mermaid_prompt}\n\nDiagram Labels:\n{raw_text}", png_path)
                if mermaid_res:
                    node["rendered_markdown"] = mermaid_res.strip() + "\n\n"
                    transformed_count += 1
                    continue

            # Fallback to Obsidian image embed + RAG sidecar
            node["rendered_markdown"] = f"![[{asset_file}|{caption}]]\n\n*{caption}*\n\n"
            transformed_count += 1

            # Generate technical engineering sidecar via Gemini Vision
            sidecar_name = f"{asset_file}.txt"
            sidecar_path = assets_dir / sidecar_name
            sidecar_text = None
            if png_path and png_path.exists() and sidecar_prompt:
                sidecar_text = gemini.generate_vision(f"{sidecar_prompt}\n\nExtracted Labels:\n{raw_text}", png_path)

            if not sidecar_text:
                sidecar_text = (
                    f"Title: {caption}\n"
                    f"Page: {page_num}\n"
                    f"Labels:\n{raw_text}\n"
                )

            with open(sidecar_path, "w", encoding="utf-8") as sf:
                sf.write(sidecar_text)
            node["sidecar_path"] = f"assets/{sidecar_name}"
            sidecar_count += 1

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
        workspace_dir / "07_chapters_tables",
        workspace_dir / "06_chapters_continuations",
        workspace_dir / "05_chapters_raw",
        workspace_dir / "chapters"
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

            caption = raw_text.splitlines()[0].strip() if raw_text.strip() else f"Figure on page {page_num}"
            caption = re.sub(r"[\[\]|]", "", caption)
            draft_md = f"![[{asset_file}|{caption}]]\n\n*{caption}*\n"
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
    updating workspace/08_chapters_graphics/ and workspace/assets/.
    """
    tasks_dir = workspace_dir / "tasks" / "graphics"
    assets_dir = workspace_dir / "assets"
    assets_dir.mkdir(parents=True, exist_ok=True)

    if not tasks_dir.exists():
        print(f"[!] No graphic tasks directory found at {tasks_dir}")
        return 0

    input_candidates = [
        workspace_dir / "07_chapters_tables",
        workspace_dir / "06_chapters_continuations",
        workspace_dir / "05_chapters_raw",
        workspace_dir / "chapters"
    ]
    input_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not input_dir:
        raise FileNotFoundError(f"Missing input chapters directory in {workspace_dir}")

    out_dir = workspace_dir / "08_chapters_graphics"
    out_dir.mkdir(parents=True, exist_ok=True)

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
                node["rendered_markdown"] = rendered_by_node[n_id]
                asset_f = meta.get("asset_file", f"asset_{n_id}.png")
                node["sidecar_path"] = f"assets/{asset_f}.txt"
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
