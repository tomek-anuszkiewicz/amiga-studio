#!/usr/bin/env python3
"""
stages/11_emit_markdown/emit_markdown.py:
Serializes partitioned chapter node streams into final Markdown documents:
1. Emits one file per section: <output_dir>/{index:02d}_{slug}.md.
2. Explicitly ignores and skips any segment of type 'toc_heading'.
3. Skips child continuation nodes whose content was rendered by the head node.
4. Copies all referenced assets from workspace/assets/ to <output_dir>/assets/.
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
from common.pdf_tables import ordered_markdown, converted


def emit_markdown(workspace_dir: Path, output_dir: Path, config: dict):
    chapters_dir = workspace_dir / "10_proofread_stream"
    chapters = [json.loads(path.read_text(encoding="utf-8"))
                for path in sorted(chapters_dir.iterdir()) if path.suffix == ".json"]

    output_dir.mkdir(parents=True, exist_ok=True)
    for old_md in output_dir.glob("*.md"):
        old_md.unlink()
    out_assets_dir = output_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)
    for old_asset in out_assets_dir.glob("*"):
        if old_asset.is_file():
            old_asset.unlink(missing_ok=True)

    # Collect explicitly referenced asset filenames from rendered markdown
    referenced_assets = set()
    for entry in chapters:
        for node in entry["nodes"]:
            rendered = node.get("rendered_markdown") or ""
            for match in re.finditer(r"asset_node_\d+\.[a-zA-Z0-9]+", rendered):
                referenced_assets.add(match.group(0))
            if node.get("table_format") == "html":
                referenced_assets.add(Path(node["png_path"]).name)

    # 1. Synchronize assets from latest stage (only active, referenced assets)
    src_assets_dir = chapters_dir / "assets"
    if src_assets_dir.exists():
        for asset_file in src_assets_dir.glob("*"):
            if asset_file.is_file():
                base_name = re.sub(r"\.txt$", "", asset_file.name)
                if asset_file.name in referenced_assets or base_name in referenced_assets:
                    shutil.copy2(asset_file, out_assets_dir / asset_file.name)

    if out_assets_dir.exists():
        if any(out_assets_dir.iterdir()):
            if src_assets_dir:
                print(f"[*] Synchronized active assets from {src_assets_dir} to {out_assets_dir}")
        else:
            out_assets_dir.rmdir()

    print(f"[*] Emitting {len(chapters)} Markdown files from {chapters_dir.name} to {output_dir}...")

    # Assert uniqueness of target Markdown filenames to prevent silent overwrites
    target_names = [e.get("target_md_file") for e in chapters if e.get("target_md_file")]
    if len(target_names) != len(set(target_names)):
        duplicates = [name for name in target_names if target_names.count(name) > 1]
        raise ValueError(f"Stage 11 Collision Error: Duplicate target Markdown filenames detected in chapters: {set(duplicates)}")

    for entry in chapters:
        idx = entry["index"]
        slug = entry["slug"]
        title = entry["title"]
        target_md_name = entry.get("target_md_file", f"{idx:02d}_{slug}.md")
        target_path = output_dir / target_md_name

        nodes = entry["nodes"]

        def render(node):
            n_type = node.get("type")

            # Ignore and skip toc_heading segments per design specification
            if n_type == "toc_heading":
                return ""

            # Skip child continuation nodes
            if node.get("continuation_status") == "continuation" and not converted(node):
                return ""

            if "rendered_markdown" in node:
                rendered = node.get("rendered_markdown")
                if rendered and rendered.strip():
                    return rendered.rstrip()
            elif node.get("raw_text"):
                return node["raw_text"].rstrip()
            return ""

        content = ordered_markdown(nodes, render, lambda n: f"assets/{Path(n['png_path']).name}")

        with open(target_path, "w", encoding="utf-8") as f:
            f.write(content.rstrip() + "\n")

        print(f"    Emitted: {target_md_name}")

    print(f"[+] Stage 11 complete. Markdown files written to {output_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 11: Emit per-section Markdown files")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--output-dir", type=str, default="output_markdown", help="Target output directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    output_dir = Path(args.output_dir)
    config_path = Path(args.config)

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)

    emit_markdown(workspace_dir, output_dir, config)


if __name__ == "__main__":
    main()
