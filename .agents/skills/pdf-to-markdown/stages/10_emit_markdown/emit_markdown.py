#!/usr/bin/env python3
"""
stages/10_emit_markdown/emit_markdown.py:
Serializes partitioned chapter node streams into final Markdown documents:
1. Emits one file per section: <output_dir>/{index:02d}_{slug}.md.
2. Injects active Line 1 YAML frontmatter into each file.
3. Explicitly ignores and skips any segment of type 'toc_header'.
4. Skips child continuation nodes whose content was rendered by the head node.
5. Copies all referenced assets from workspace/assets/ to <output_dir>/assets/.
"""

import argparse
import json
import os
import shutil
import sys
from pathlib import Path
import yaml


def build_frontmatter(title: str, section_idx: int) -> str:
    return (
        f"---\n"
        f"title: \"{title}\"\n"
        f"section_index: {section_idx}\n"
        f"tags:\n"
        f"  - amiga\n"
        f"  - reference\n"
        f"  - hardware\n"
        f"properties:\n"
        f"  section_index: {section_idx}\n"
        f"---\n\n"
    )


def emit_markdown(workspace_dir: Path, output_dir: Path, config: dict):
    manifest_path = workspace_dir / "chapters_manifest.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"Missing chapters_manifest.json in {workspace_dir}")

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    output_dir.mkdir(parents=True, exist_ok=True)
    out_assets_dir = output_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)

    # 1. Synchronize assets
    src_assets_dir = workspace_dir / "assets"
    if src_assets_dir.exists():
        for asset_file in src_assets_dir.glob("*"):
            if asset_file.is_file():
                shutil.copy2(asset_file, out_assets_dir / asset_file.name)
        print(f"[*] Synchronized assets to {out_assets_dir}")

    print(f"[*] Emitting {len(manifest)} Markdown files to {output_dir}...")

    for entry in manifest:
        idx = entry["index"]
        slug = entry["slug"]
        title = entry["title"]
        target_md_name = entry.get("target_md_file", f"{idx:02d}_{slug}.md")
        target_path = output_dir / target_md_name

        json_file = workspace_dir / entry["json_file"]
        if not json_file.exists():
            continue

        with open(json_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        content_parts = [build_frontmatter(title, idx)]

        for node in nodes:
            n_type = node.get("type")

            # Ignore and skip toc_header segments per design specification
            if n_type == "toc_header":
                continue

            # Skip child continuation nodes
            if node.get("continuation_status") == "continuation":
                continue

            rendered = node.get("rendered_markdown")
            if rendered is not None and rendered.strip():
                content_parts.append(rendered.rstrip() + "\n\n")
            elif node.get("raw_text"):
                content_parts.append(node["raw_text"].rstrip() + "\n\n")

        with open(target_path, "w", encoding="utf-8") as f:
            f.write("".join(content_parts).rstrip() + "\n")

        print(f"    Emitted: {target_md_name}")

    print(f"[+] Stage 10 complete. Markdown files written to {output_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 10: Emit per-section Markdown files with Line 1 YAML")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--output-dir", type=str, default="output_markdown", help="Target output directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    output_dir = Path(args.output_dir)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    emit_markdown(workspace_dir, output_dir, config)


if __name__ == "__main__":
    main()
