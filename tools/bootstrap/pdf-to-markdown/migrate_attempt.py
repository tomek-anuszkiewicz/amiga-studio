#!/usr/bin/env python3
"""One-time transfer of explicitly selected legacy attempts; never run by pipeline.py."""

import argparse
import os
from pathlib import Path
import shutil
import yaml

from common.pdf_artifacts import read_json, write_json

CHAPTER_STAGES = ("05_chapter_partition", "06_detect_continuations", "07_transform_tables",
                  "08_transform_graphics", "09_transform_prose", "10_proofread_stream")


def migrate_attempt(workspace, pdf=None):
    workspace = Path(workspace).resolve()
    state_path = workspace / ".conversion-state.json"
    state = read_json(state_path)
    config_path = workspace / "config.yaml"
    config = yaml.safe_load(config_path.read_text(encoding="utf-8"))
    preparation_path = workspace / "00_text_layer/text_layer_manifest.json"
    preparation = read_json(preparation_path) if preparation_path.exists() else {}
    source = Path(pdf).resolve() if pdf else (workspace / (
        preparation["source_file"] if "source_file" in preparation else config["input"]["source_pdf"])).resolve()
    config["input"] = {"source_pdf": Path(os.path.relpath(source, workspace)).as_posix(),
                       "pages": state["source"].get("pages")}

    def metadata(stage_id, directory):
        snapshot = state.get("stages", {}).get(stage_id, {}).get("shared_outputs", {}).get("chapters_manifest.json", {}).get("snapshot")
        path = workspace / snapshot if snapshot else workspace / directory / ".manifests/chapters_manifest.json"
        if not path.exists() and (stage_id == "10" or "10" not in state.get("stages", {})):
            path = workspace / "chapters_manifest.json"
        return {(entry["index"], entry["slug"]): entry for entry in read_json(path)}

    # Resolve every required metadata source before writing or removing any file.
    transfers = []
    metadata_by_owner = {}
    for directory in CHAPTER_STAGES:
        stage_path = workspace / directory
        if not stage_path.exists():
            continue
        for path in sorted(stage_path.glob("*.json")):
            nodes = read_json(path)
            if isinstance(nodes, dict):
                continue  # Already transferred in an interrupted migration.
            owner = "10" if directory.startswith("10") else "05"
            if owner not in metadata_by_owner:
                metadata_by_owner[owner] = metadata(owner, "10_proofread_stream" if owner == "10" else "05_chapter_partition")
            index, slug = path.stem.split("_", 1)
            entry = metadata_by_owner[owner][(int(index), slug)]
            transfers.append((path, {key: entry[key] for key in ("index", "slug", "title", "target_md_file")} | {"nodes": nodes}))

    status_path = workspace / "stage_status.json"
    statuses = read_json(status_path) if status_path.exists() else {}
    original_statuses = dict(statuses)
    for stage_id, entry in state.get("stages", {}).items():
        if stage_id not in statuses:
            statuses[stage_id] = {"status": "success" if entry["status"] == "completed" else entry["status"]}
    for path, chapter in transfers:
        write_json(path, chapter)
    candidate = config_path.with_suffix(".yaml.tmp")
    candidate.write_text(yaml.safe_dump(config, sort_keys=False, allow_unicode=True), encoding="utf-8")
    candidate.replace(config_path)
    if not status_path.exists() or statuses != original_statuses:
        write_json(status_path, statuses)

    obsolete = (workspace / "pages_manifest.json", workspace / "chapters_manifest.json", preparation_path,
                workspace / ".pages_manifest.candidate.json")
    for path in obsolete:
        path.unlink(missing_ok=True)
    # Only this explicit attempt is traversed; stage outputs and OCR recovery stay.
    for path in workspace.glob("*/.manifests"):
        resolved = path.resolve()
        if not resolved.is_relative_to(workspace) or resolved == workspace:
            raise ValueError("Migration cleanup escapes its selected attempt")
        shutil.rmtree(path)
    state_path.unlink()
    print(f"[migration] {workspace.name}: transferred inputs and {len(transfers)} chapters; retained outputs and metrics")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, required=True, help="One explicitly selected legacy attempt")
    parser.add_argument("--pdf", type=Path, help="Source PDF when legacy preparation metadata has no source path")
    args = parser.parse_args()
    migrate_attempt(args.workspace, args.pdf)


if __name__ == "__main__":
    main()
