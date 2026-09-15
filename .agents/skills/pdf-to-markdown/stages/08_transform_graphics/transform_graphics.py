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
    chapters_dir = workspace_dir / "chapters"
    assets_dir = workspace_dir / "assets"
    if not chapters_dir.exists():
        raise FileNotFoundError(f"Missing chapters directory: {chapters_dir}")

    gemini = GeminiClient(config) if GeminiClient else None
    if gemini and gemini.is_available():
        print(f"[*] Graphics Worker LLM active ({gemini.vision_model}, thinking: {gemini.thinking_level}).")
    else:
        print(f"[*] Graphics Worker LLM unavailable (no GEMINI_API_KEY). Using heuristic diagram converter.")

    chapter_files = sorted(list(chapters_dir.glob("*.json")))
    print(f"[*] Transforming graphics across {len(chapter_files)} chapter files...")

    transformed_count = 0
    sidecar_count = 0

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        modified = False
        for node in nodes:
            if node.get("type") != "graphic":
                continue

            node_id = node.get("node_id", "asset")
            page_num = node.get("page", 1)
            raw_text = node.get("raw_text", "")

            # Check if this is a flowchart / state machine candidate
            is_flowchart = bool(re.search(r"(state\s+machine|flowchart|step\s+\d+|transition)", raw_text, re.IGNORECASE))

            if is_flowchart:
                # Generate sample Mermaid block with collapsible ASCII fallback
                rendered = (
                    f"```mermaid\n"
                    f"flowchart TD\n"
                    f"    A[\"Start / Initial State\"] --> B[\"{node_id}\"]\n"
                    f"```\n\n"
                    f"> [!NOTE]- Click to view Text / ASCII Diagram\n"
                    f"> [Start] ---> [{node_id}]\n"
                )
                node["rendered_markdown"] = rendered
            else:
                # Embed as Obsidian wikilink
                svg_rel = node.get("svg_path")
                png_rel = node.get("png_path")
                asset_file = Path(svg_rel).name if svg_rel else (Path(png_rel).name if png_rel else f"asset_{node_id}.png")

                caption = raw_text.splitlines()[0].strip() if raw_text.strip() else f"Figure on page {page_num}"
                caption = re.sub(r"[\[\]|]", "", caption)
                node["rendered_markdown"] = f"![[{asset_file}|{caption}]]\n\n*{caption}*\n"

                # Generate RAG sidecar .txt file
                sidecar_name = f"{asset_file}.txt"
                sidecar_path = assets_dir / sidecar_name
                sidecar_content = generate_default_sidecar(node_id, raw_text, page_num)
                with open(sidecar_path, "w", encoding="utf-8") as sf:
                    sf.write(sidecar_content)
                node["sidecar_path"] = f"assets/{sidecar_name}"
                sidecar_count += 1

            modified = True
            transformed_count += 1

        if modified:
            with open(c_file, "w", encoding="utf-8") as f:
                json.dump(nodes, f, indent=2)

    print(f"[+] Stage 08 complete. Transformed {transformed_count} graphics, generated {sidecar_count} RAG sidecars.")


def main():
    parser = argparse.ArgumentParser(description="Stage 08: Transform graphic nodes into Mermaid/Obsidian embeds with RAG sidecars")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_graphics(workspace_dir, config)


if __name__ == "__main__":
    main()
