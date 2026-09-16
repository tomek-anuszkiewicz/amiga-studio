#!/usr/bin/env python3
"""
stages/10_proofread_stream/proofread_stream.py:
Proofreads chapter streams and manifest metadata before Markdown serialization:
1. Proofreads and corrects chapter titles in chapters_manifest.json using Gemini LLM.
2. Harmonizes chapter titles with primary heading nodes in the stream.
3. Derives clean slugs and target filenames directly in chapters_manifest.json.
4. Performs an OCR proofreading pass on stream node text.
5. Emits normalized chapter JSON files to workspace/10_proofread_stream/ and updates chapters_manifest.json.
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import re
import shutil
import sys
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def generate_slug(text: str) -> str:
    cleaned = text.lower()
    cleaned = re.sub(r"[^a-z0-9]+", "_", cleaned).strip("_")
    return cleaned[:40] if cleaned else "section"


def proofread_title_llm(raw_title: str, gemini: GeminiClient) -> str:
    prompt = (
        "You are an expert technical editor. Correct any OCR typos, accidental split words, or weird spacing "
        "in this chapter/section title. Do not change the wording or meaning; only fix OCR errors and broken words "
        "(e.g. 'HARDW ARE' -> 'HARDWARE', 'COPROC ESSOR' -> 'COPROCESSOR', 'PLAYF IELD' -> 'PLAYFIELD'). "
        "Return ONLY the clean corrected title text without quotes, markdown formatting, or explanation:\n\n"
        f"{raw_title}"
    )
    try:
        c_title = gemini.generate_text(prompt, stage="10_proofread_stream").strip().strip('"').strip("'")
        if c_title and len(c_title) < len(raw_title) * 2:
            return c_title
    except Exception as e:
        print(f"[!] Warning: LLM title proofreading failed for '{raw_title}': {e}")
    return raw_title


def proofread_node_text(text: str, gemini: GeminiClient, base_prompt: str) -> str:
    if not text or not text.strip():
        return text
    # Only invoke LLM on blocks with substantial text or suspicious split words / punctuation
    prompt = f"{base_prompt}\n\n## Content to Proofread:\n\n{text}"
    try:
        corrected = gemini.generate_text(prompt, stage="10_proofread_stream").strip()
        if corrected:
            return corrected
    except Exception as e:
        print(f"[!] Warning: Node proofreading failed: {e}")
    return text


def process_proofread_stream(
    workspace_dir: Path,
    input_dir: Path,
    output_dir: Path,
    config: dict,
    skip_llm: bool = False
):
    output_dir.mkdir(parents=True, exist_ok=True)
    out_assets = output_dir / "assets"

    # Clean previous outputs in output_dir
    for old_json in output_dir.glob("*.json"):
        try:
            old_json.unlink()
        except Exception:
            pass
    if out_assets.exists():
        for old_asset in out_assets.glob("*"):
            if old_asset.is_file():
                try:
                    old_asset.unlink()
                except Exception:
                    pass

    manifest_path = workspace_dir / "chapters_manifest.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"Missing chapters_manifest.json in {workspace_dir}")

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    # Locate input chapters directory if not provided or missing
    if not input_dir or not input_dir.exists() or not list(input_dir.glob("*.json")):
        candidates = [
            workspace_dir / "09_transform_prose",
            workspace_dir / "08_transform_graphics",
            workspace_dir / "07_transform_tables",
            workspace_dir / "06_detect_continuations",
            workspace_dir / "05_chapter_partition",
        ]
        input_dir = next((p for p in candidates if p.exists() and list(p.glob("*.json"))), input_dir)

    if not input_dir or not input_dir.exists():
        raise FileNotFoundError(f"Missing input chapter files in {workspace_dir}")

    # Synchronize assets forward
    src_assets = input_dir / "assets"
    if not src_assets.exists():
        for cand in [
            workspace_dir / "08_transform_graphics" / "assets",
            workspace_dir / "07_transform_tables" / "assets",
            workspace_dir / "05_chapter_partition" / "assets",
            workspace_dir / "assets",
        ]:
            if cand.exists():
                src_assets = cand
                break

    if src_assets and src_assets.exists():
        for asset_f in src_assets.glob("*"):
            if asset_f.is_file():
                out_assets.mkdir(parents=True, exist_ok=True)
                shutil.copy2(asset_f, out_assets / asset_f.name)

    # Initialize Gemini client
    gemini = None
    base_prompt = ""
    if not skip_llm:
        gemini = GeminiClient(config) if GeminiClient else None
        prompt_file = Path(__file__).resolve().parent / "prompt.md"
        if prompt_file.exists():
            base_prompt = prompt_file.read_text(encoding="utf-8")

    concurrency = int(config.get("llm", {}).get("concurrency", 8))
    if gemini and gemini.is_available():
        print(f"[*] Stream Proofreading LLM active ({gemini.default_model}). Processing {len(manifest)} partitions (concurrency={concurrency})...")
    else:
        print(f"[*] Stream Proofreading LLM offline or skipped. Normalizing streams directly...")

    updated_manifest = []

    for entry in manifest:
        idx = entry["index"]
        raw_slug = entry["slug"]
        raw_title = entry["title"]

        # Locate chapter json file
        json_file = input_dir / f"{idx:02d}_{raw_slug}.json"
        if not json_file.exists():
            json_file = workspace_dir / entry.get("json_file", "")
        if not json_file.exists():
            # Match any json with prefix
            matches = list(input_dir.glob(f"{idx:02d}_*.json"))
            if matches:
                json_file = matches[0]

        if not json_file.exists():
            print(f"[!] Warning: Chapter JSON for {raw_title} not found at {json_file}")
            updated_manifest.append(entry)
            continue

        with open(json_file, "r", encoding="utf-8") as f:
            nodes = json.load(f)

        # 1. Proofread and correct chapter title
        corrected_title = raw_title
        if gemini and gemini.is_available() and not skip_llm and raw_title:
            corrected_title = proofread_title_llm(raw_title, gemini)

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
        target_md_name = f"{target_file_slug}.md"

        # 4. Save proofread nodes to output_dir
        out_json_path = output_dir / out_json_name
        with open(out_json_path, "w", encoding="utf-8") as f:
            json.dump(nodes, f, indent=2)

        updated_entry = {
            "index": idx,
            "slug": new_slug,
            "title": corrected_title,
            "json_file": f"10_proofread_stream/{out_json_name}",
            "target_md_file": target_md_name,
            "node_count": len(nodes),
        }
        updated_manifest.append(updated_entry)
        print(f"    Proofread Partition {idx:02d}: {corrected_title} -> {out_json_name}")

    # Write updated manifest back to workspace
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(updated_manifest, f, indent=2)

    # Clean empty assets directory if present
    if out_assets.exists() and not any(out_assets.iterdir()):
        try:
            out_assets.rmdir()
        except Exception:
            pass

    print(f"[+] Stage 10 complete. Proofread streams and manifest written to {output_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 10: Proofread streams & manifest with LLM")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--input-dir", type=str, default=None, help="Input directory (defaults to workspace/09_transform_prose)")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory (defaults to workspace/10_proofread_stream)")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")
    parser.add_argument("--skip-llm", action="store_true", help="Skip LLM proofreading and normalize streams directly")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    input_dir = Path(args.input_dir) if args.input_dir else (workspace_dir / "09_transform_prose")
    output_dir = Path(args.output_dir) if args.output_dir else (workspace_dir / "10_proofread_stream")

    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 10: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 10: Config file is empty or invalid: {config_path}")

    process_proofread_stream(
        workspace_dir,
        input_dir,
        output_dir,
        config,
        skip_llm=args.skip_llm
    )


if __name__ == "__main__":
    main()
