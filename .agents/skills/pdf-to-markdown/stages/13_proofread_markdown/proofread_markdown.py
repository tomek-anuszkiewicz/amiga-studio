#!/usr/bin/env python3
"""
stages/13_proofread_markdown/proofread_markdown.py:
Final proofreading and OCR glitch correction pass for assembled Markdown documents:
1. Reads canonical Markdown files from workspace/12_canonical_markdown/*.md.
2. Preserves Line 1 YAML frontmatter intact.
3. Chunks document sections by headings (## / ###) to prevent LLM truncation.
4. Corrects OCR typos, misread punctuation, and register names using Gemini LLM.
5. Emits proofread Markdown files and synchronized assets directly to workspace/13_proofread_markdown/*.md.
"""

import argparse
import json
import os
import re
import shutil
import sys
from pathlib import Path
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def split_into_sections(body: str) -> list:
    """
    Splits markdown body into cohesive sections based on headings (## or ###)
    or size threshold (~2000 chars) while keeping headers attached.
    """
    lines = body.splitlines(keepends=True)
    sections = []
    current = []
    curr_len = 0

    for line in lines:
        is_heading = bool(re.match(r"^#{1,3}\s+", line))
        if is_heading and current and curr_len > 1200:
            sections.append("".join(current))
            current = [line]
            curr_len = len(line)
        else:
            current.append(line)
            curr_len += len(line)

    if current:
        sections.append("".join(current))

    return sections if sections else [body]


def proofread_section(section_text: str, gemini: GeminiClient, base_prompt: str) -> str:
    """
    Sends section to Gemini LLM for OCR proofreading.
    Falls back to original text if response is empty or invalid.
    """
    if not section_text.strip() or not gemini or not gemini.is_available():
        return section_text

    full_prompt = (
        f"{base_prompt}\n\n"
        f"## Section Text to Proofread:\n"
        f"{section_text}\n"
    )

    try:
        res = gemini.generate_text(full_prompt)
        if res and res.strip():
            cleaned = res.strip()
            # Strip accidental outer markdown code fence wrapping if model added it
            if cleaned.startswith("```markdown") and cleaned.endswith("```"):
                cleaned = cleaned[len("```markdown"): -3].strip()
            elif cleaned.startswith("```") and cleaned.endswith("```"):
                cleaned = cleaned[3:-3].strip()
            return cleaned + "\n\n"
    except Exception as e:
        print(f"    [!] Proofread warning on section: {e}", file=sys.stderr)

    return section_text


def process_proofreading(
    workspace_dir: Path,
    input_dir: Path,
    output_dir: Path,
    config: dict,
    skip_llm: bool = False
):
    output_dir.mkdir(parents=True, exist_ok=True)
    out_assets = output_dir / "assets"
    out_assets.mkdir(parents=True, exist_ok=True)

    # 1. Clean previous stage outputs
    for old_f in output_dir.glob("*.md"):
        try:
            old_f.unlink()
        except Exception:
            pass

    # 2. Locate Markdown files
    if not input_dir.exists():
        candidates = [
            workspace_dir / "12_canonical_markdown",
            workspace_dir / "11_markdown_linked",
            workspace_dir / "10_markdown_raw",
        ]
        input_dir = next((p for p in candidates if p.exists() and list(p.glob("*.md"))), None)

    if not input_dir or not input_dir.exists():
        raise FileNotFoundError(f"Missing input Markdown files in {workspace_dir}")

    md_files = sorted(list(input_dir.glob("*.md")))
    if not md_files:
        print("[!] No Markdown files found to proofread.")
        return

    # 3. Synchronize assets from input_dir to output_dir
    src_assets = input_dir / "assets"
    if not src_assets.exists():
        for cand in [
            workspace_dir / "11_markdown_linked" / "assets",
            workspace_dir / "10_markdown_raw" / "assets",
            workspace_dir / "04_reduced_stream" / "assets"
        ]:
            if cand.exists():
                src_assets = cand
                break

    if src_assets and src_assets.exists():
        for asset_f in src_assets.glob("*"):
            if asset_f.is_file():
                shutil.copy2(asset_f, out_assets / asset_f.name)

    # 4. Initialize Gemini Client
    gemini = None
    base_prompt = ""
    if not skip_llm:
        gemini = GeminiClient(config) if GeminiClient else None
        prompt_path = Path(__file__).parent / "prompt_proofread.md"
        if prompt_path.exists():
            base_prompt = prompt_path.read_text(encoding="utf-8")

    if gemini and gemini.is_available():
        print(f"[*] Proofreading LLM active ({gemini.default_model}). Processing {len(md_files)} files...")
    else:
        print(f"[*] LLM offline or skipped. Copying {len(md_files)} files directly...")

    # 5. Process each file
    for md_file in md_files:
        with open(md_file, "r", encoding="utf-8") as f:
            content = f.read()

        frontmatter = ""
        body = content
        if content.startswith("---"):
            parts = content.split("---", 2)
            if len(parts) >= 3:
                frontmatter = f"---{parts[1]}---\n\n"
                body = parts[2].lstrip()

        if gemini and gemini.is_available() and base_prompt and not skip_llm:
            sections = split_into_sections(body)
            proofread_parts = []
            for sec in sections:
                p_sec = proofread_section(sec, gemini, base_prompt)
                proofread_parts.append(p_sec.rstrip())
            final_body = "\n\n".join(proofread_parts).strip() + "\n"
        else:
            final_body = body.strip() + "\n"

        full_doc = f"{frontmatter}{final_body}"

        # Write to output_dir
        target_file = output_dir / md_file.name
        with open(target_file, "w", encoding="utf-8") as f:
            f.write(full_doc)

        print(f"    Proofread: {md_file.name}")

    print(f"[+] Stage 13 complete. Proofread documents saved in {output_dir}.")


def main():
    parser = argparse.ArgumentParser(description="Stage 13: Final OCR proofreading pass")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--input-dir", type=str, default=None, help="Input directory (defaults to workspace/12_canonical_markdown)")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory (defaults to workspace/13_proofread_markdown)")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--skip-llm", action="store_true", help="Skip LLM proofreading and copy directly")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    input_dir = Path(args.input_dir) if args.input_dir else (workspace_dir / "12_canonical_markdown")
    output_dir = Path(args.output_dir) if args.output_dir else (workspace_dir / "13_proofread_markdown")

    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        try:
            with open(config_path, "r", encoding="utf-8") as f:
                config = yaml.safe_load(f) or {}
        except Exception:
            config = {}

    process_proofreading(
        workspace_dir,
        input_dir,
        output_dir,
        config,
        skip_llm=args.skip_llm
    )


if __name__ == "__main__":
    main()
