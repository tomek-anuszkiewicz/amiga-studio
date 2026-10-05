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
from typing import Optional
import yaml


# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def check_continuation_with_gemini(
    node_a: dict,
    node_b: dict,
    gemini: Optional[GeminiClient],
    prompt_template: str,
    caption_hint: Optional[str] = None,
) -> bool:
    """
    Uses Gemini LLM to analyze candidate adjacent table/graphic blocks across page boundaries.
    """
    if node_a.get("type") not in ("table", "graphic") or node_b.get("type") not in ("table", "graphic"):
        return False

    page_a = node_a.get("page", 0)
    page_b = node_b.get("page", 0)

    # Must be on consecutive pages
    if page_b != page_a + 1:
        return False

    text_a = node_a.get("raw_text", "").strip()
    text_b = node_b.get("raw_text", "").strip()
    if not text_a or not text_b:
        return False

    if prompt_template:
        sample_a = text_a if len(text_a) <= 800 else f"{text_a[:350]}\n...\n{text_a[-400:]}"
        sample_b = text_b if len(text_b) <= 800 else f"{text_b[:500]}\n...\n{text_b[-250:]}"
        prompt = (
            f"{prompt_template}\n\n"
            f"## Block A (Page {page_a}, Type: {node_a.get('type')}):\n"
            f"```text\n{sample_a}\n```\n\n"
            f"## Block B (Page {page_b}, Type: {node_b.get('type')}):\n"
            f"```text\n{sample_b}\n```\n"
        )
        if caption_hint:
            prompt += f"\n## Preceding Continuation Caption on Page {page_b}:\n```text\n{caption_hint}\n```\n"

        res = gemini.generate_json(prompt)
        if isinstance(res, dict) and res.get("is_continuation"):
            return True

    return False


def process_chapter_continuations(workspace_dir: Path, config: dict):
    input_dir = workspace_dir / "05_chapter_partition"
    if not input_dir.exists():
        raise FileNotFoundError(f"Input chapters directory missing: {input_dir}")

    out_dir = workspace_dir / "06_detect_continuations"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in out_dir.glob("*.json"):
        f.unlink()

    prompt_path = Path(__file__).resolve().parent / "prompt_continuation.md"
    prompt_template = prompt_path.read_text(encoding="utf-8") if prompt_path.exists() else ""

    gemini = GeminiClient(config)

    chapter_files = sorted(list(input_dir.glob("*.json")))
    from concurrent.futures import ThreadPoolExecutor

    concurrency = int(config.get("llm", {}).get("concurrency", 8))
    print(f"[*] Detecting continuations across {len(chapter_files)} chapter files using Gemini (concurrency={concurrency})...")

    def _process_chapter(c_file_and_idx):
        c_file, ch_idx = c_file_and_idx
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        ch_continuations = 0
        group_counter = 1

        i = 0
        while i < len(nodes) - 1:
            curr_node = nodes[i]
            if curr_node.get("type") not in ("table", "graphic"):
                i += 1
                continue

            # Forward scan for multi-page continuation chain
            head_node = curr_node
            prev_in_chain = curr_node
            j = i + 1
            while j < len(nodes):
                cap_node = None
                candidate = None
                advance_step = 1

                if (
                    nodes[j].get("type") == "caption_continuation"
                    and j + 1 < len(nodes)
                    and nodes[j + 1].get("type") == prev_in_chain.get("type")
                ):
                    cap_node = nodes[j]
                    candidate = nodes[j + 1]
                    advance_step = 2
                elif nodes[j].get("type") == prev_in_chain.get("type"):
                    candidate = nodes[j]
                    advance_step = 1
                else:
                    break

                caption_hint = cap_node.get("raw_text", "").strip() if cap_node else None
                if check_continuation_with_gemini(prev_in_chain, candidate, gemini, prompt_template, caption_hint=caption_hint):
                    if not head_node.get("continuation_status"):
                        group_id = f"table_group_{ch_idx:02d}_{group_counter:04d}"
                        group_counter += 1
                        head_node["continuation_status"] = "head"
                        head_node["is_head"] = True
                        head_node["continuation_group_id"] = group_id
                        head_node["merged_nodes"] = [head_node["node_id"]]
                        head_node["merged_assets"] = []
                        for p in ("svg_path", "png_path", "raw_text_path"):
                            if head_node.get(p):
                                head_node["merged_assets"].append(head_node[p])

                    if cap_node:
                        cap_node["continuation_status"] = "absorbed_caption"
                        cap_node["is_head"] = False
                        cap_node["continued_from"] = head_node["node_id"]
                        cap_node["continuation_group_id"] = head_node["continuation_group_id"]
                        head_node["merged_nodes"].append(cap_node["node_id"])

                    head_node["merged_nodes"].append(candidate["node_id"])
                    for p in ("svg_path", "png_path", "raw_text_path"):
                        if candidate.get(p):
                            head_node["merged_assets"].append(candidate[p])

                    candidate["continuation_status"] = "continuation"
                    candidate["is_head"] = False
                    candidate["continued_from"] = head_node["node_id"]
                    candidate["continuation_group_id"] = head_node["continuation_group_id"]

                    ch_continuations += 1
                    print(f"    Linked continuation: {candidate['node_id']} (Page {candidate['page']}) -> {head_node['node_id']} (Page {head_node['page']})")
                    prev_in_chain = candidate
                    j += advance_step
                else:
                    break

            i = j

        # Write immutable output to 06_detect_continuations
        target_file = out_dir / c_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

        return ch_continuations

    with ThreadPoolExecutor(max_workers=min(len(chapter_files), max(1, concurrency))) as executor:
        results = list(executor.map(_process_chapter, [(cf, idx + 1) for idx, cf in enumerate(chapter_files)]))

    total_continuations = sum(results)
    print(f"[+] Stage 06 complete. Emitted {len(chapter_files)} chapters to {out_dir} with {total_continuations} continuations.")


def prepare_continuation_tasks(workspace_dir: Path) -> int:
    """
    Extracts candidate multi-page table/graphic continuations into
    workspace/tasks/continuations/candidates.json for Agent review.
    """
    chapters_dir = workspace_dir / "05_chapter_partition"
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
            if curr_node.get("type") != "table":
                continue

            cap_node = None
            next_node = None
            if i + 1 < len(nodes) and nodes[i + 1].get("type") == "table":
                next_node = nodes[i + 1]
            elif (
                i + 2 < len(nodes)
                and nodes[i + 1].get("type") == "caption_continuation"
                and nodes[i + 2].get("type") == "table"
            ):
                cap_node = nodes[i + 1]
                next_node = nodes[i + 2]

            if next_node:
                page_a = curr_node.get("page", 0)
                page_b = next_node.get("page", 0)
                if page_b == page_a + 1:
                    is_cont = are_tables_likely_continuation(curr_node, next_node) or (cap_node is not None)
                    candidates.append({
                        "candidate_id": f"cont_{cand_counter:04d}",
                        "chapter_file": c_file.name,
                        "head_node_id": curr_node["node_id"],
                        "head_page": page_a,
                        "head_snippet": curr_node.get("raw_text", "")[:200],
                        "tail_node_id": next_node["node_id"],
                        "tail_page": page_b,
                        "tail_snippet": next_node.get("raw_text", "")[:200],
                        "cap_node_id": cap_node["node_id"] if cap_node else None,
                        "is_continuation": is_cont,
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
    into workspace/06_detect_continuations/ without mutating 05_chapter_partition.
    """
    cand_file = workspace_dir / "tasks" / "continuations" / "candidates.json"
    if not cand_file.exists():
        print(f"[!] No candidates.json found at {cand_file}")
        return 0

    with open(cand_file, "r", encoding="utf-8") as f:
        candidates = json.load(f)

    input_dir = workspace_dir / "05_chapter_partition"
    out_dir = workspace_dir / "06_detect_continuations"
    out_dir.mkdir(parents=True, exist_ok=True)

    applied_count = 0
    group_counter = 1

    # Group candidates by chapter file name
    by_chapter = {}
    for c in candidates:
        if c.get("is_continuation"):
            by_chapter.setdefault(c["chapter_file"], []).append(c)

    for c_file in sorted(list(input_dir.glob("*.json"))):
        with open(c_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        node_map = {n.get("node_id"): n for n in nodes}
        items = by_chapter.get(c_file.name, [])

        for item in items:
            head_id = item["head_node_id"]
            tail_id = item["tail_node_id"]
            cap_id = item.get("cap_node_id")

            if head_id in node_map and tail_id in node_map:
                group_id = f"table_group_{group_counter:04d}"
                group_counter += 1

                head_n = node_map[head_id]
                tail_n = node_map[tail_id]

                head_n["continuation_status"] = "head"
                head_n["is_head"] = True
                head_n["continuation_group_id"] = group_id
                merged = [head_id]
                if cap_id and cap_id in node_map:
                    cap_n = node_map[cap_id]
                    cap_n["continuation_status"] = "absorbed_caption"
                    cap_n["is_head"] = False
                    cap_n["continued_from"] = head_id
                    cap_n["continuation_group_id"] = group_id
                    merged.append(cap_id)
                merged.append(tail_id)
                head_n["merged_nodes"] = merged

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

        target_file = out_dir / c_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

    print(f"[+] Applied {applied_count} confirmed continuations to {out_dir}")
    return applied_count


def main():
    parser = argparse.ArgumentParser(description="Stage 06: Detect multi-page table and graphic continuations")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")
    parser.add_argument("--prepare", action="store_true", help="Prepare continuation candidates for Agent review")
    parser.add_argument("--apply", action="store_true", help="Apply confirmed continuations from candidates.json back to chapters")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 06: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 06: Config file is empty or invalid: {config_path}")

    if args.prepare:
        prepare_continuation_tasks(workspace_dir)
        return

    if args.apply:
        apply_continuation_tasks(workspace_dir)
        return

    process_chapter_continuations(workspace_dir, config)


if __name__ == "__main__":
    main()
