#!/usr/bin/env python3
"""
stages/05_chapter_partition/partition_chapters.py:
Partitions the monolithic reduced_stream.json into clean chapter-level streams:
1. Preamble & Front Matter: all nodes prior to Table of Contents -> 00_preface.json.
2. Table of Contents & Lists: all TOC, figures, and tables lists -> 00_toc.json.
3. Real Chapters: welds chapter numbers and titles (e.g. "Chapter 1" + "INTRODUCTION")
   into unified chapter streams (e.g. 01_chapter_1_introduction.json).
4. Subsections within chapters remain inside their respective chapter stream.
5. Emits workspace/05_chapter_partition/*.json and workspace/chapters_manifest.json.
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
    reduced_stream_path = workspace_dir / "04_stream_reduction" / "reduced_stream.json"

    if not reduced_stream_path.exists():
        raise FileNotFoundError(f"Missing reduced_stream.json in {reduced_stream_path.parent}")

    with open(reduced_stream_path, "r", encoding="utf-8") as f:
        nodes = json.load(f)

    chapters_raw_dir = workspace_dir / "05_chapter_partition"
    chapters_raw_dir.mkdir(parents=True, exist_ok=True)

    print(f"[*] Partitioning {len(nodes)} nodes into chapter streams...")

    # Clean prior chapter json files
    for f in chapters_raw_dir.glob("*.json"):
        f.unlink()

    # Find boundaries for Front Matter and Table of Contents
    first_toc_idx = None
    first_chapter_idx = None

    for idx, node in enumerate(nodes):
        n_type = node.get("type")
        raw = node.get("raw_text", "").strip()

        # Detect TOC start
        if first_toc_idx is None:
            if n_type == "toc_header" or n_type == "toc" or (n_type == "heading" and "table of contents" in raw.lower()):
                first_toc_idx = idx

        # Detect First Chapter start (must be after TOC or standalone)
        if first_chapter_idx is None and first_toc_idx is not None and idx > first_toc_idx:
            if n_type == "chapter":
                first_chapter_idx = idx

    # Fallback if TOC markers were not present
    if first_chapter_idx is None:
        for idx, node in enumerate(nodes):
            if node.get("type") == "chapter":
                first_chapter_idx = idx
                break

    partitions = []

    # 1. Front Matter / Preface (All nodes prior to TOC)
    if first_toc_idx is not None and first_toc_idx > 0:
        front_matter_nodes = nodes[:first_toc_idx]
        partitions.append({
            "index": 0,
            "title": "Preface & Front Matter",
            "slug": "preface",
            "nodes": front_matter_nodes
        })

    # 2. Table of Contents & Lists (From first TOC node up to first Chapter)
    if first_toc_idx is not None:
        toc_end_idx = first_chapter_idx if first_chapter_idx is not None else len(nodes)
        toc_nodes = nodes[first_toc_idx:toc_end_idx]
        if toc_nodes:
            partitions.append({
                "index": 0,
                "title": "Table of Contents",
                "slug": "toc",
                "nodes": toc_nodes
            })

    # 3. Chapters (Starting at first_chapter_idx)
    chapter_nodes = nodes[first_chapter_idx:] if first_chapter_idx is not None else (nodes if first_toc_idx is None else [])

    chapter_counter = 1
    current_chapter_nodes = []
    current_chapter_title = ""

    i = 0
    while i < len(chapter_nodes):
        node = chapter_nodes[i]

        if node.get("type") == "chapter":
            # Save previous chapter if active
            if current_chapter_nodes:
                partitions.append({
                    "index": chapter_counter,
                    "title": current_chapter_title,
                    "slug": generate_slug(current_chapter_title),
                    "nodes": current_chapter_nodes
                })
                chapter_counter += 1

            # Check if next node on the same page is a heading/subtitle (e.g. "INTRODUCTION")
            raw_title = node.get("raw_text", "").strip()
            current_chapter_nodes = [node]

            if i + 1 < len(chapter_nodes):
                next_node = chapter_nodes[i + 1]
                if next_node.get("page") == node.get("page") and next_node.get("type") in ("heading", "chapter"):
                    subtitle = next_node.get("raw_text", "").strip()
                    current_chapter_title = f"{raw_title}: {subtitle}"
                    current_chapter_nodes.append(next_node)
                    i += 1
                else:
                    current_chapter_title = raw_title
            else:
                current_chapter_title = raw_title
        else:
            if current_chapter_nodes:
                current_chapter_nodes.append(node)
            else:
                # Trailing or orphaned node before first chapter
                current_chapter_nodes = [node]
                first_line = node.get("raw_text", "").strip().splitlines()[0][:40] if node.get("raw_text") else ""
                current_chapter_title = first_line if node.get("type") == "heading" and first_line else "Section"

        i += 1

    # Save final chapter
    if current_chapter_nodes:
        partitions.append({
            "index": chapter_counter,
            "title": current_chapter_title,
            "slug": generate_slug(current_chapter_title),
            "nodes": current_chapter_nodes
        })

    # If no partitions formed, default fallback
    if not partitions:
        partitions = [{
            "index": 1,
            "title": "Document",
            "slug": "document",
            "nodes": nodes
        }]

    manifest = []
    for part in partitions:
        idx = part["index"]
        file_slug = f"{idx:02d}_{part['slug']}"
        file_name = f"{file_slug}.json"
        target_path = chapters_raw_dir / file_name
        with open(target_path, "w", encoding="utf-8") as f:
            json.dump(part["nodes"], f, indent=2)

        clean_title_name = re.sub(r'[:/\\|]', ' - ', part['title'])
        clean_title_name = re.sub(r'[*?"<>]', '', clean_title_name)
        clean_title_name = re.sub(r'\s+', ' ', clean_title_name).strip(' -.')
        target_md_name = f"{idx:02d} - {clean_title_name}.md" if clean_title_name else f"{file_slug}.md"

        manifest.append({
            "index": idx,
            "slug": part["slug"],
            "title": part["title"],
            "json_file": f"05_chapter_partition/{file_name}",
            "target_md_file": target_md_name,
            "node_count": len(part["nodes"])
        })
        print(f"    Partition {idx:02d}: {part['title']} ({len(part['nodes'])} nodes) -> {file_name}")

    manifest_path = workspace_dir / "chapters_manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)

    print(f"[+] Stage 05 complete. {len(partitions)} clean chapter streams emitted to {chapters_raw_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 05: Partition stream into numbered section files")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 05: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 05: Config file is empty or invalid: {config_path}")

    partition_chapters(workspace_dir, config)


if __name__ == "__main__":
    main()
