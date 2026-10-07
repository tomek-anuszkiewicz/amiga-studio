#!/usr/bin/env python3
"""
stages/06_detect_continuations/detect_continuations.py:
Scans candidate adjacent table (and graphic) blocks across page transitions in chapter streams.
Tags nodes with continuation metadata (group_id, continuation_status, continued_from)
so Stage 07 fuses them into a single coherent table block.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Optional
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))


# Import CodexClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

from conversion import CodexClient
from common import pdf_schemas


def check_continuation_with_codex(
    node_a: dict,
    node_b: dict,
    codex: Optional[CodexClient],
    prompt_template: str,
    caption_hint: Optional[str] = None,
) -> bool:
    """
    Uses Codex LLM to analyze candidate adjacent table/graphic blocks across page boundaries.
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

        res = codex.generate_json(prompt, schema=pdf_schemas.CONTINUATION)
        if res["is_continuation"]:
            return True

    return False


def process_chapter_continuations(workspace_dir: Path, config: dict):
    input_dir = workspace_dir / "05_chapter_partition"

    out_dir = workspace_dir / "06_detect_continuations"
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in (path for path in out_dir.iterdir() if path.suffix == ".json"):
        f.unlink()

    prompt_path = Path(__file__).resolve().parent / "prompt_continuation.md"
    prompt_template = prompt_path.read_text(encoding="utf-8")

    with CodexClient(config, stage="06_detect_continuations", images=False) as codex:

        chapter_files = sorted(list((path for path in input_dir.iterdir() if path.suffix == ".json")))
        from concurrent.futures import ThreadPoolExecutor

        concurrency = int(config.get("llm", {}).get("concurrency", 1))
        print(f"[*] Detecting continuations across {len(chapter_files)} chapter files using Codex (concurrency={concurrency})...")

        def _process_chapter(c_file_and_idx):
            c_file, ch_idx = c_file_and_idx
            with open(c_file, "r", encoding="utf-8") as f:
                chapter = json.load(f)
                nodes = chapter["nodes"]

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
                    if check_continuation_with_codex(prev_in_chain, candidate, codex, prompt_template, caption_hint=caption_hint):
                        if not head_node.get("continuation_status"):
                            group_id = f"table_group_{ch_idx:02d}_{group_counter:04d}"
                            group_counter += 1
                            head_node["continuation_status"] = "head"
                            head_node["is_head"] = True
                            head_node["continuation_group_id"] = group_id
                            head_node["merged_nodes"] = [head_node["node_id"]]
                            head_node["merged_assets"] = []
                            for p in ("svg_path", "png_path"):
                                if head_node.get(p):
                                    head_node["merged_assets"].append(head_node[p])

                        if cap_node:
                            cap_node["continuation_status"] = "absorbed_caption"
                            cap_node["is_head"] = False
                            cap_node["continued_from"] = head_node["node_id"]
                            cap_node["continuation_group_id"] = head_node["continuation_group_id"]
                            head_node["merged_nodes"].append(cap_node["node_id"])

                        head_node["merged_nodes"].append(candidate["node_id"])
                        for p in ("svg_path", "png_path"):
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
                json.dump({**chapter, "nodes": nodes}, f, indent=2)

            return ch_continuations

        with ThreadPoolExecutor(max_workers=min(len(chapter_files), max(1, concurrency))) as executor:
            results = list(executor.map(_process_chapter, [(cf, idx + 1) for idx, cf in enumerate(chapter_files)]))

        total_continuations = sum(results)
        print(f"[+] Stage 06 complete. Emitted {len(chapter_files)} chapters to {out_dir} with {total_continuations} continuations.")


def main():
    parser = argparse.ArgumentParser(description="Stage 06: Detect multi-page table and graphic continuations")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)


    process_chapter_continuations(workspace_dir, config)


if __name__ == "__main__":
    main()
