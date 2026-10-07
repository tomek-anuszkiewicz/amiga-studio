"""Track completed PDF stages and required files without content fingerprints."""

from pathlib import Path
import hashlib
import json
import shutil

SHARED_MANIFESTS = {"01": ("pages_manifest.json",), "05": ("chapters_manifest.json",), "10": ("chapters_manifest.json",)}


def source_selection(source):
    """Keep workspace ownership while ignoring old source checksums."""
    return {"name": source["name"], "pages": source.get("pages")}


def record_path(workspace, output, relative):
    from .pdf_artifacts import artifact_path, prepared_pdf_path, read_json
    anchor, name = relative.split(":", 1)
    if anchor == "prepared":
        pdf = prepared_pdf_path(workspace, read_json(workspace / "00_text_layer/text_layer_manifest.json"))
        if name != pdf.name:
            raise ValueError("Unexpected shared prepared artifact")
        return pdf
    base = {"workspace": workspace, "output": output}.get(anchor)
    if base is None:
        raise ValueError(f"Invalid conversion artifact anchor: {anchor}")
    return artifact_path(base, name)


def file_hash(path):
    """Identify OCR recovery requests; never used to validate stage completion."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_state(workspace):
    path = workspace / ".conversion-state.json"
    state = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {"stages": {}}
    if state.get("source"):
        state["source"] = source_selection(state["source"])
    # Existing user workspaces retain their completion statuses and file paths.
    for record in state.get("stages", {}).values():
        for key in ("identity", "completion"):
            record.pop(key, None)
        record["files"] = [name for name in record.get("files", [])
                           if not name.endswith(("/.metrics", "/.metrics.json")) and "/recovery/" not in name]
        record["shared_outputs"] = {name: {"snapshot": entry["snapshot"]}
                                    for name, entry in record.get("shared_outputs", {}).items()}
    return state


def write_state(workspace, state):
    path = workspace / ".conversion-state.json"
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, indent=2), encoding="utf-8")
    temporary.replace(path)


def validate_completed_stages(state, registry, workspace, output):
    """Check status and file existence; execution validates artifact contents."""
    from .pdf_artifacts import artifact_path
    for stage in registry:
        record = state.get("stages", {}).get(stage["id"], {})
        if record.get("status") != "completed" or not record.get("files"):
            raise ValueError(f"Missing completed predecessor: regenerate Stage {stage['id']}")
        for relative in record["files"]:
            if not record_path(workspace, output, relative).is_file():
                raise ValueError(f"Missing Stage {stage['id']} artifact: {relative}")
        for entry in record.get("shared_outputs", {}).values():
            if not artifact_path(workspace, entry["snapshot"]).is_file():
                raise ValueError(f"Missing Stage {stage['id']} manifest snapshot")


def restore_shared(state, registry, workspace):
    from .pdf_artifacts import artifact_path
    outputs = {}
    for stage in registry:
        outputs.update(state["stages"][stage["id"]].get("shared_outputs", {}))
    for name in {name for names in SHARED_MANIFESTS.values() for name in names} - outputs.keys():
        (workspace / name).unlink(missing_ok=True)
    for name, entry in outputs.items():
        shutil.copyfile(artifact_path(workspace, entry["snapshot"]), artifact_path(workspace, name))


def complete_stage(state, stage, source, workspace, output):
    from .pdf_artifacts import validate_stage_artifacts
    validate_stage_artifacts(stage, workspace, source)
    directory = workspace / stage["dir"]
    shared_outputs = {}
    for name in SHARED_MANIFESTS.get(stage["id"], ()):
        snapshot = directory / ".manifests" / name
        snapshot.parent.mkdir(exist_ok=True)
        shutil.copyfile(workspace / name, snapshot)
        shared_outputs[name] = {"snapshot": snapshot.relative_to(workspace).as_posix()}
    paths = [("workspace", p) for p in sorted(directory.rglob("*"))
             if p.is_file() and p.name not in (".metrics", ".metrics.json") and "recovery" not in p.relative_to(directory).parts]
    if stage.get("artifact_contract") == "pdf_text_layer":
        from .pdf_artifacts import validate_text_layer
        _, pdf = validate_text_layer(workspace, source)
        paths.append(("prepared", pdf))
    if stage["id"] == "14" and output is not None:
        paths.extend(("output", p) for p in sorted(output.rglob("*")) if p.is_file() and p.suffix != ".json")
    files = []
    for anchor, path in paths:
        if path.suffix == ".json":
            json.loads(path.read_text(encoding="utf-8"))
        base = workspace if anchor == "workspace" else (path.parent if anchor == "prepared" else output)
        files.append(f"{anchor}:{path.relative_to(base).as_posix()}")
    if not files:
        raise ValueError(f"Stage {stage['id']} produced no artifacts")
    state.setdefault("stages", {})[stage["id"]] = {"status": "completed", "files": files, "shared_outputs": shared_outputs}
    write_state(workspace, state)
