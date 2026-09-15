#!/usr/bin/env python3
"""
stages/05_chapter_partition/partition_chapters.py:
Partitions the monolithic reduced_stream.json into chapter-level streams using numeric naming:
workspace/chapters/{index:02d}_{slug}.json.
Preamble Invariant: Any segments before the first detected chapter heading are prepended to Chapter 1.
Emits workspace/chapters_manifest.json.
"""

import argparse
import json
import re
import sys
from pathlib import Path
import yaml


def generate_slug(text: str) -> str:
    cleaned = text.lower()
    cleaned = re.sub(r"[^a-z0-9]+", "_", cleaned).strip("_")
    return cleaned[:40] if cleaned else "section"


def partition_chapters(workspace_dir: Path, config: dict):
    reduced_stream_path = workspace_dir / "reduced_stream.json"
    if not reduced_stream_path.exists():
        raise FileNotFoundError(f"Missing reduced_stream.json in {workspace_dir}")

    with open(reduced_stream_path, "r", encoding="utf-8") as f:
        nodes = json.load(f)

    chapters_dir = workspace_dir / "chapters"
    chapters_dir.mkdir(parents=True, exist_ok=True)

    print(f"[*] Partitioning {len(nodes)} nodes into chapter streams...")

    # Identify partition boundary points
    partitions = []
    current_nodes = []
    current_title = "preliminary"
    current_slug = "preliminary"
    has_found_first_heading = False

    for node in nodes:
        is_heading_1 = (node.get("type") == "heading" and node.get("heading_level") == 1)

        # Check for chapter boundary
        if is_heading_1:
            title_text = node.get("raw_text", "").splitlines()[0].strip()
            slug = generate_slug(title_text)

            if not has_found_first_heading:
                # First chapter encountered!
                # Preamble invariant: if current_nodes has content, keep them inside this first chapter
                has_found_first_heading = True
                current_title = title_text
                current_slug = slug
                current_nodes.append(node)
            else:
                # Save previous partition
                if current_nodes:
                    partitions.append({
                        "title": current_title,
                        "slug": current_slug,
                        "nodes": current_nodes
                    })
                current_title = title_text
                current_slug = slug
                current_nodes = [node]
        else:
            current_nodes.append(node)

    # Append trailing partition
    if current_nodes:
        partitions.append({
            "title": current_title,
            "slug": current_slug,
            "nodes": current_nodes
        })

    # If no major headings were detected, keep everything in 01_document
    if not partitions:
        partitions = [{
            "title": "Document",
            "slug": "document",
            "nodes": nodes
        }]

    # Write per-chapter JSON files with clean numeric prefixes
    manifest = []
    for idx, part in enumerate(partitions, start=1):
        file_slug = f"{idx:02d}_{part['slug']}"
        file_name = f"{file_slug}.json"
        target_path = chapters_dir / file_name

        with open(target_path, "w", encoding="utf-8") as f:
            json.dump(part["nodes"], f, indent=2)

        manifest.append({
            "index": idx,
            "slug": part["slug"],
            "title": part["title"],
            "json_file": f"chapters/{file_name}",
            "target_md_file": f"{file_slug}.md",
            "node_count": len(part["nodes"])
        })
        print(f"    Partition {idx:02d}: {part['title']} ({len(part['nodes'])} nodes) -> {file_name}")

    manifest_path = workspace_dir / "chapters_manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)

    print(f"[+] Stage 05 complete. {len(partitions)} chapters partitioned into {chapters_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 05: Partition stream into numbered section files")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    partition_chapters(workspace_dir, config)


if __name__ == "__main__":
    main()
