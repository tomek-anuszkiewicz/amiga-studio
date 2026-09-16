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
import re
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
    input_candidates = [
        workspace_dir / "10_proofread_stream",
        workspace_dir / "09_transform_prose",
        workspace_dir / "09_chapters_formatted",
        workspace_dir / "08_transform_graphics",
        workspace_dir / "08_chapters_graphics",
        workspace_dir / "07_transform_tables",
        workspace_dir / "07_chapters_tables",
        workspace_dir / "06_detect_continuations",
        workspace_dir / "06_chapters_continuations",
        workspace_dir / "05_chapter_partition",
        workspace_dir / "05_chapters_raw",
    ]
    chapters_dir = next((p for p in input_candidates if p.exists() and list(p.glob("*.json"))), None)
    if not chapters_dir:
        raise FileNotFoundError(f"Missing formatted chapters in {workspace_dir}")

    manifest_path = workspace_dir / "chapters_manifest.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"Missing chapters_manifest.json in {workspace_dir}")

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

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
    for entry in manifest:
        idx = entry["index"]
        slug = entry["slug"]
        c_path = chapters_dir / f"{idx:02d}_{slug}.json"
        if not c_path.exists():
            c_path = workspace_dir / entry.get("json_file", "")
        if c_path.exists():
            try:
                with open(c_path, "r", encoding="utf-8") as f:
                    c_nodes = json.load(f)
                for cn in c_nodes:
                    rendered = cn.get("rendered_markdown", "")
                    for m in re.finditer(r"asset_node_\d+\.[a-zA-Z0-9]+", rendered):
                        referenced_assets.add(m.group(0))
            except Exception:
                pass

    # 1. Synchronize assets from latest stage (only active, referenced assets)
    asset_candidates = [
        workspace_dir / "10_proofread_stream" / "assets",
        workspace_dir / "08_transform_graphics" / "assets",
        workspace_dir / "08_chapters_graphics" / "assets",
        workspace_dir / "07_transform_tables" / "assets",
        workspace_dir / "07_chapters_tables" / "assets",
        workspace_dir / "06_detect_continuations" / "assets",
        workspace_dir / "06_chapters_continuations" / "assets",
        workspace_dir / "05_chapter_partition" / "assets",
        workspace_dir / "05_chapters_raw" / "assets",
        workspace_dir / "04_stream_reduction" / "assets",
        workspace_dir / "04_reduced_stream" / "assets",
        workspace_dir / "03_build_raw_stream" / "assets",
        workspace_dir / "03_raw_stream" / "assets",
        workspace_dir / "assets",
    ]
    src_assets_dir = next((p for p in asset_candidates if p.exists()), None)
    if src_assets_dir and src_assets_dir.exists():
        for asset_file in src_assets_dir.glob("*"):
            if asset_file.is_file():
                base_name = re.sub(r"\.txt$", "", asset_file.name)
                if asset_file.name in referenced_assets or base_name in referenced_assets:
                    shutil.copy2(asset_file, out_assets_dir / asset_file.name)
        if any(out_assets_dir.iterdir()):
            print(f"[*] Synchronized active assets from {src_assets_dir} to {out_assets_dir}")
        else:
            try:
                out_assets_dir.rmdir()
            except Exception:
                pass

    print(f"[*] Emitting {len(manifest)} Markdown files from {chapters_dir.name} to {output_dir}...")

    for entry in manifest:
        idx = entry["index"]
        slug = entry["slug"]
        title = entry["title"]
        target_md_name = entry.get("target_md_file", f"{idx:02d}_{slug}.md")
        target_path = output_dir / target_md_name

        file_slug = f"{idx:02d}_{slug}.json"
        json_file = chapters_dir / file_slug
        if not json_file.exists():
            # Try path from manifest
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

            if "rendered_markdown" in node:
                rendered = node.get("rendered_markdown")
                if rendered and rendered.strip():
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
