#!/usr/bin/env python3
"""
stages/06_detect_continuations/detect_continuations.py:
Scans candidate adjacent table (and graphic) blocks across page transitions in chapter streams.
Tags nodes with continuation metadata (group_id, continuation_status, continued_from)
so Stage 07 fuses them into a single coherent table block.
"""

import argparse
import json
import re
import sys
from pathlib import Path
import yaml


def are_tables_likely_continuation(node_a: dict, node_b: dict) -> bool:
    """
    Deterministic heuristic checking if table B continues table A across a page break.
    """
    if node_a.get("type") != "table" or node_b.get("type") != "table":
        return False

    page_a = node_a.get("page", 0)
    page_b = node_b.get("page", 0)

    # Must be on consecutive pages
    if page_b != page_a + 1:
        return False

    # Check text characteristics
    text_a = node_a.get("raw_text", "").strip()
    text_b = node_b.get("raw_text", "").strip()
    if not text_a or not text_b:
        return False

    lines_a = [l.strip() for l in text_a.splitlines() if l.strip()]
    lines_b = [l.strip() for l in text_b.splitlines() if l.strip()]

    # If first line of B says "continued" or matches header of A
    if lines_b and ("continued" in lines_b[0].lower() or (lines_a and lines_a[0].lower() == lines_b[0].lower())):
        return True

    # Count tab / column structure similarity
    tab_count_a = sum(l.count("\t") for l in lines_a) / max(1, len(lines_a))
    tab_count_b = sum(l.count("\t") for l in lines_b) / max(1, len(lines_b))
    if abs(tab_count_a - tab_count_b) <= 0.5 and tab_count_a > 1.0:
        return True

    return False


def process_chapter_continuations(workspace_dir: Path, config: dict):
    chapters_dir = workspace_dir / "chapters"
    if not chapters_dir.exists():
        raise FileNotFoundError(f"Chapters directory missing: {chapters_dir}")

    chapter_files = sorted(list(chapters_dir.glob("*.json")))
    print(f"[*] Detecting continuations across {len(chapter_files)} chapter files...")

    total_continuations = 0
    group_counter = 1

    for c_file in chapter_files:
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        modified = False
        i = 0
        while i < len(nodes) - 1:
            curr_node = nodes[i]
            next_node = nodes[i + 1]

            # Allow skipping whitespace / minor elements if right next
            if are_tables_likely_continuation(curr_node, next_node):
                group_id = f"table_group_{group_counter:04d}"
                group_counter += 1
                total_continuations += 1
                modified = True

                # Mark head node
                curr_node["continuation_status"] = "head"
                curr_node["is_head"] = True
                curr_node["continuation_group_id"] = group_id
                curr_node["merged_nodes"] = [curr_node["node_id"], next_node["node_id"]]

                # Collect merged asset paths
                merged_assets = []
                for p in ("svg_path", "png_path", "raw_text_path"):
                    if curr_node.get(p):
                        merged_assets.append(curr_node[p])
                    if next_node.get(p):
                        merged_assets.append(next_node[p])
                curr_node["merged_assets"] = merged_assets

                # Mark child continuation node
                next_node["continuation_status"] = "continuation"
                next_node["is_head"] = False
                next_node["continued_from"] = curr_node["node_id"]
                next_node["continuation_group_id"] = group_id

                print(f"    Linked continuation: {next_node['node_id']} (Page {next_node['page']}) -> {curr_node['node_id']} (Page {curr_node['page']})")
                i += 2
            else:
                i += 1

        if modified:
            with open(c_file, "w", encoding="utf-8") as f:
                json.dump(nodes, f, indent=2)

    print(f"[+] Stage 06 complete. Detected and linked {total_continuations} multi-page continuations.")


def prepare_continuation_tasks(workspace_dir: Path) -> int:
    """
    Extracts candidate multi-page table/graphic continuations into
    workspace/tasks/continuations/candidates.json for Agent review.
    """
    chapters_dir = workspace_dir / "chapters"
    if not chapters_dir.exists():
        raise FileNotFoundError(f"Chapters directory missing: {chapters_dir}")

    tasks_dir = workspace_dir / "tasks" / "continuations"
    tasks_dir.mkdir(parents=True, exist_ok=True)

    candidates = []
    cand_counter = 1

    for c_file in sorted(list(chapters_dir.glob("*.json"))):
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        for i in range(len(nodes) - 1):
            curr_node = nodes[i]
            next_node = nodes[i + 1]

            if curr_node.get("type") == "table" and next_node.get("type") == "table":
                page_a = curr_node.get("page", 0)
                page_b = next_node.get("page", 0)
                if page_b == page_a + 1:
                    is_cont = are_tables_likely_continuation(curr_node, next_node)
                    candidates.append({
                        "candidate_id": f"cont_{cand_counter:04d}",
                        "chapter_file": str(c_file.relative_to(workspace_dir)),
                        "head_node_id": curr_node["node_id"],
                        "head_page": page_a,
                        "head_snippet": curr_node.get("raw_text", "")[:200],
                        "tail_node_id": next_node["node_id"],
                        "tail_page": page_b,
                        "tail_snippet": next_node.get("raw_text", "")[:200],
                        "is_continuation": is_cont
                    })
                    cand_counter += 1

    cand_file = tasks_dir / "candidates.json"
    with open(cand_file, "w", encoding="utf-8") as f:
        json.dump(candidates, f, indent=2)

    print(f"[+] Prepared {len(candidates)} continuation candidates in {cand_file}")
    return len(candidates)


def apply_continuation_tasks(workspace_dir: Path) -> int:
    """
    Applies confirmed continuations from workspace/tasks/continuations/candidates.json
    into chapter streams.
    """
    cand_file = workspace_dir / "tasks" / "continuations" / "candidates.json"
    if not cand_file.exists():
        print(f"[!] No candidates.json found at {cand_file}")
        return 0

    with open(cand_file, "r", encoding="utf-8") as f:
        candidates = json.load(f)

    applied_count = 0
    group_counter = 1

    # Group candidates by chapter
    by_chapter = {}
    for c in candidates:
        if c.get("is_continuation"):
            by_chapter.setdefault(c["chapter_file"], []).append(c)

    for chapter_rel, items in by_chapter.items():
        chapter_path = workspace_dir / chapter_rel
        if not chapter_path.exists():
            continue

        with open(chapter_path, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        node_map = {n.get("node_id"): n for n in nodes}

        for item in items:
            head_id = item["head_node_id"]
            tail_id = item["tail_node_id"]

            if head_id in node_map and tail_id in node_map:
                group_id = f"table_group_{group_counter:04d}"
                group_counter += 1

                head_n = node_map[head_id]
                tail_n = node_map[tail_id]

                head_n["continuation_status"] = "head"
                head_n["is_head"] = True
                head_n["continuation_group_id"] = group_id
                head_n["merged_nodes"] = [head_id, tail_id]

                merged_assets = []
                for p in ("svg_path", "png_path", "raw_text_path"):
                    if head_n.get(p):
                        merged_assets.append(head_n[p])
                    if tail_n.get(p):
                        merged_assets.append(tail_n[p])
                head_n["merged_assets"] = merged_assets

                tail_n["continuation_status"] = "continuation"
                tail_n["is_head"] = False
                tail_n["continued_from"] = head_id
                tail_n["continuation_group_id"] = group_id

                applied_count += 1

        with open(chapter_path, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

    print(f"[+] Applied {applied_count} confirmed continuations to chapter streams.")
    return applied_count


def main():
    parser = argparse.ArgumentParser(description="Stage 06: Detect multi-page table and graphic continuations")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--prepare", action="store_true", help="Prepare continuation candidates for Agent review")
    parser.add_argument("--apply", action="store_true", help="Apply confirmed continuations from candidates.json back to chapters")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    if args.prepare:
        prepare_continuation_tasks(workspace_dir)
        return

    if args.apply:
        apply_continuation_tasks(workspace_dir)
        return

    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_chapter_continuations(workspace_dir, config)


if __name__ == "__main__":
    main()
