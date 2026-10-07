#!/usr/bin/env python3
"""
stages/10_proofread_stream/proofread_stream.py:
Proofreads self-contained chapter titles and nodes before Markdown serialization:
1. Proofreads and corrects chapter titles using Codex LLM.
2. Harmonizes chapter titles with primary heading nodes in the stream.
3. Derives clean slugs and target filenames directly.
4. Emits normalized chapter JSON files to workspace/10_proofread_stream/.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import re
import shutil
import sys
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_runtime import LLM_CONCURRENCY

# Import CodexClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

from conversion import CodexClient


def generate_slug(text: str) -> str:
    cleaned = text.lower()
    cleaned = re.sub(r"[^a-z0-9]+", "_", cleaned).strip("_")
    return cleaned[:40] if cleaned else "section"


def proofread_title_llm(raw_title: str, codex: CodexClient) -> str:
    prompt = (
        "You are an expert technical editor. Correct any OCR typos, accidental split words, or weird spacing "
        "in this chapter/section title. Do not change the wording or meaning; only fix OCR errors and broken words "
        "(e.g. 'HARDW ARE' -> 'HARDWARE', 'COPROC ESSOR' -> 'COPROCESSOR', 'PLAYF IELD' -> 'PLAYFIELD'). "
        "Motorola processor model numbers end in numeric zeros, NEVER the letter 'O' "
        "(e.g. 'MC68000' not 'MC68OOO', 'MC68HC000' not 'MC68HCOOO', 'MC68EC000' not 'MC68ECOOO'). "
        "Return ONLY the clean corrected title text without quotes, markdown formatting, or explanation:\n\n"
        f"{raw_title}"
    )
    try:
        c_title = codex.generate_text(prompt).strip().strip('"').strip("'")
        return c_title
    except Exception:
        raise


def determine_section_title_llm(raw_title: str, raw_slug: str, sample_text: str, codex: CodexClient) -> str:
    """Queries Codex to dynamically determine the canonical, publication-grade title based on section content."""
    prompt = (
        "You are an expert technical book editor. Determine the canonical, professional title for this section "
        "of a technical manual based on its actual content.\n\n"
        f"Preliminary Title: {raw_title}\n"
        f"Section Slug: {raw_slug}\n"
        f"Content Excerpt:\n```markdown\n{sample_text[:1500]}\n```\n\n"
        "Return ONLY the clean canonical title string (e.g. 'Preface and Front Matter', 'Table of Contents', 'Publication Colophon') "
        "without quotes, markdown formatting, or explanation:"
    )
    try:
        c_title = codex.generate_text(prompt).strip().strip('"').strip("'")
        if raw_slug == "preface" and "table of contents" in c_title.lower():
            return "Front Matter"
        return c_title
    except Exception:
        raise


def process_proofread_stream(
    workspace_dir: Path,
    input_dir: Path,
    output_dir: Path,
    config: dict
):
    output_dir.mkdir(parents=True, exist_ok=True)
    out_assets = output_dir / "assets"
    out_assets.mkdir(parents=True, exist_ok=True)

    # Clean previous outputs in output_dir
    for old_json in (path for path in output_dir.iterdir() if path.suffix == ".json"):
        old_json.unlink()
    if out_assets.exists():
        for old_asset in out_assets.glob("*"):
            if old_asset.is_file():
                old_asset.unlink()

    chapters = [json.loads(path.read_text(encoding="utf-8"))
                for path in sorted(input_dir.iterdir()) if path.suffix == ".json"]

    # Synchronize assets forward
    src_assets = input_dir / "assets"
    if src_assets and src_assets.exists():
        for asset_file in src_assets.glob("*"):
            if asset_file.is_file():
                shutil.copy2(asset_file, out_assets / asset_file.name)

    with CodexClient(config, stage="10_proofread_stream", images=False) as codex:
        prompt_file = Path(__file__).resolve().parent / "prompt.md"
        base_prompt = prompt_file.read_text(encoding="utf-8")

        concurrency = LLM_CONCURRENCY
        print(f"[*] Stream Proofreading LLM active ({codex.selected.model}). Processing {len(chapters)} partitions (concurrency={concurrency})...")

        title_map = {}
        if chapters:
            from concurrent.futures import ThreadPoolExecutor

            def _proofread_single_title(entry):
                raw_title = entry.get("title", "")
                raw_slug = entry.get("slug", "")
                idx = entry.get("index", 0)
                key = (idx, raw_slug)

                sample_nodes = entry["nodes"]
                informative_nodes = [n for n in sample_nodes if n.get("type") not in ("thumb_index", "header", "footer")]
                if not informative_nodes:
                    informative_nodes = sample_nodes
                sample_text = " ".join(n.get("raw_text", "") or n.get("rendered_markdown", "") for n in informative_nodes[:10])

                if idx == 0 or raw_slug in ("preface", "toc"):
                    return key, determine_section_title_llm(raw_title, raw_slug, sample_text, codex)
                elif raw_title:
                    return key, proofread_title_llm(raw_title, codex)
                return key, raw_title

            with ThreadPoolExecutor(max_workers=min(len(chapters), concurrency)) as executor:
                title_results = list(executor.map(_proofread_single_title, chapters))
            title_map = dict(title_results)

        outputs = []

        for entry in chapters:
            idx = entry["index"]
            raw_slug = entry["slug"]
            raw_title = entry["title"]

            nodes = entry["nodes"]

            # 1. Proofread and correct chapter title using composite key
            corrected_title = title_map.get((idx, raw_slug), raw_title)

            # 2. Harmonize with primary heading node and update node rendered_markdown
            ch_sub = re.match(r"^chapter\s+\d+[:\s]+(.*)$", corrected_title, re.IGNORECASE)
            expected_sub = ch_sub.group(1).strip() if ch_sub else ""

            for node in nodes:
                if node.get("type") in ("chapter", "heading"):
                    rendered = node.get("rendered_markdown", "").strip()
                    if rendered:
                        h_m = re.search(r"^#+\s+(Chapter\s+\d+)\s*\n+#+\s+([^\n]+)", rendered, re.IGNORECASE | re.MULTILINE)
                        if h_m:
                            expected = f"{h_m.group(1).title()}: {h_m.group(2).strip()}"
                            if expected.lower().replace(" ", "") == corrected_title.lower().replace(" ", ""):
                                corrected_title = expected
                        elif expected_sub:
                            r_clean = rendered.lstrip("#").strip()
                            if r_clean.lower().replace(" ", "") == expected_sub.lower().replace(" ", ""):
                                node["rendered_markdown"] = f"# {expected_sub}\n\n"

            # 3. Derive clean canonical slug and filename
            if idx > 0:
                clean_t = corrected_title.lower()
                ch_sub = re.match(r"^chapter\s+\d+[:\s]+(.*)$", clean_t)
                rest = ch_sub.group(1) if ch_sub else clean_t
                rest_slug = generate_slug(rest)
                new_slug = f"chapter_{idx}_{rest_slug}" if rest_slug else f"chapter_{idx}"
            else:
                new_slug = raw_slug if raw_slug in ("toc", "preface") else generate_slug(corrected_title)

            target_file_slug = f"{idx:02d}_{new_slug}"
            out_json_name = f"{target_file_slug}.json"

            # Format canonical publication-grade markdown filename matching {idx:02d} - {Title}.md
            clean_title_name = re.sub(r'[:/\\|]', ' - ', corrected_title)
            clean_title_name = re.sub(r'[*?"<>]', '', clean_title_name)
            clean_title_name = re.sub(r'\s+', ' ', clean_title_name).strip(' -.')
            target_md_name = f"{idx:02d} - {clean_title_name}.md" if clean_title_name else f"{target_file_slug}.md"

            # Prevent duplicate target filenames across partitions in the chapters
            existing_targets = [e.get("target_md_file") for e in outputs]
            if target_md_name in existing_targets:
                target_md_name = f"{idx:02d} - {new_slug.replace('_', ' ').title()}.md"

            # 4. Save proofread nodes to output_dir
            out_json_path = output_dir / out_json_name

            updated_entry = {
                "index": idx,
                "slug": new_slug,
                "title": corrected_title,
                "target_md_file": target_md_name,
                "nodes": nodes,
            }
            with open(out_json_path, "w", encoding="utf-8") as f:
                json.dump(updated_entry, f, indent=2)
            outputs.append(updated_entry)
            print(f"    Proofread Partition {idx:02d}: {corrected_title} -> {out_json_name}")

        # Clean empty assets directory if present
        if out_assets.exists() and not any(out_assets.iterdir()):
            out_assets.rmdir()

        print(f"[+] Stage 10 complete. Proofread streams and chapters written to {output_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 10: Proofread streams & chapters with LLM")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--input-dir", type=str, default=None, help="Input directory (defaults to workspace/09_transform_prose)")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory (defaults to workspace/10_proofread_stream)")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")
    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    input_dir = Path(args.input_dir) if args.input_dir else (workspace_dir / "09_transform_prose")
    output_dir = Path(args.output_dir) if args.output_dir else (workspace_dir / "10_proofread_stream")

    config_path = Path(args.config)

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)

    process_proofread_stream(
        workspace_dir,
        input_dir,
        output_dir,
        config
    )


if __name__ == "__main__":
    main()
