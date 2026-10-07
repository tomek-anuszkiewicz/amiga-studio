"""Track completed PDF stages and shared manifest snapshots."""

from pathlib import Path
import hashlib
import json
import shutil

SHARED_MANIFESTS = {"01": ("pages_manifest.json",), "05": ("chapters_manifest.json",), "10": ("chapters_manifest.json",)}


def source_selection(source):
    """Keep workspace ownership while ignoring old source checksums."""
    return {"name": source["name"], "pages": source.get("pages")}


def file_hash(path):
    """Identify OCR recovery requests; never used to validate stage completion."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_state(workspace):
    path = workspace / ".conversion-state.json"
    state = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {"stages": {}}
    if state.get("source"):
        state["source"] = source_selection(state["source"])
    # Existing user workspaces retain completion statuses and manifest snapshots.
    for record in state.get("stages", {}).values():
        for key in ("identity", "completion", "files"):
            record.pop(key, None)
        record["shared_outputs"] = {name: {"snapshot": entry["snapshot"]}
                                    for name, entry in record.get("shared_outputs", {}).items()}
    return state


def write_state(workspace, state):
    path = workspace / ".conversion-state.json"
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, indent=2), encoding="utf-8")
    temporary.replace(path)


def validate_completed_stages(state, registry):
    """Check completion status; execution validates artifact contents."""
    for stage in registry:
        record = state.get("stages", {}).get(stage["id"], {})
        if record.get("status") != "completed":
            raise ValueError(f"Missing completed predecessor: regenerate Stage {stage['id']}")


def restore_shared(state, registry, workspace):
    from .pdf_artifacts import artifact_path
    outputs = {}
    for stage in registry:
        outputs.update(state["stages"][stage["id"]].get("shared_outputs", {}))
    for name in {name for names in SHARED_MANIFESTS.values() for name in names} - outputs.keys():
        (workspace / name).unlink(missing_ok=True)
    for name, entry in outputs.items():
        shutil.copyfile(artifact_path(workspace, entry["snapshot"]), artifact_path(workspace, name))


def complete_stage(state, stage, source, workspace):
    from .pdf_artifacts import validate_stage_artifacts
    validate_stage_artifacts(stage, workspace, source)
    directory = workspace / stage["dir"]
    shared_outputs = {}
    for name in SHARED_MANIFESTS.get(stage["id"], ()):
        snapshot = directory / ".manifests" / name
        snapshot.parent.mkdir(exist_ok=True)
        shutil.copyfile(workspace / name, snapshot)
        shared_outputs[name] = {"snapshot": snapshot.relative_to(workspace).as_posix()}
    state.setdefault("stages", {})[stage["id"]] = {"status": "completed", "shared_outputs": shared_outputs}
    write_state(workspace, state)
