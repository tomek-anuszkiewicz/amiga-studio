"""Validate PDF predecessor completion and configuration before cleanup or reuse."""

from pathlib import Path
import hashlib
import json
import shutil

from .cache import CONTRACT_VERSION, digest
from .transport import SDK_VERSION


def file_hash(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_state(workspace):
    path = workspace / ".conversion-state.json"
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else {"stages": {}}


def write_state(workspace, state):
    path = workspace / ".conversion-state.json"
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, indent=2), encoding="utf-8")
    temporary.replace(path)


def stage_identity(stage, config, source, predecessor, skill_dir):
    directory = skill_dir / "stages" / stage["dir"]
    shared = Path(__file__).resolve().parent
    procedure = {p.relative_to(directory).as_posix(): file_hash(p) for p in sorted(directory.iterdir()) if p.suffix in (".py", ".md")}
    procedure["shared"] = digest({p.name: file_hash(p) for p in sorted(shared.glob("*.py"))})
    return {"contract": CONTRACT_VERSION, "sdk_runtime": SDK_VERSION, "stage": stage["dir"],
            "selection": config["llm"]["stages"].get(stage["dir"]),
            "settings": {key: value for key, value in config.items() if key != "llm"},
            "source": source, "predecessor": predecessor, "procedure": procedure}


def validate_prefix(state, registry, count, config, source, skill_dir, workspace, output):
    previous = None
    shared_outputs = {}
    for stage in registry[:count]:
        record = state.get("stages", {}).get(stage["id"])
        if not record or record.get("status") != "completed":
            raise ValueError(f"Missing validated predecessor: regenerate Stage {stage['id']}")
        if record.get("completion") != digest({key: value for key, value in record.items() if key != "completion"}):
            raise ValueError(f"Damaged Stage {stage['id']} completion record")
        expected = stage_identity(stage, config, source, previous, skill_dir)
        if record["identity"] != expected:
            raise ValueError(f"Incompatible conversion: regenerate Stage {stage['id']} and dependents")
        for relative, expected_hash in record["files"].items():
            anchor, name = relative.split(":", 1)
            base = workspace if anchor == "workspace" else output
            path = base / name if base is not None else None
            if path is None or not path.is_file() or file_hash(path) != expected_hash:
                raise ValueError(f"Changed/missing Stage {stage['id']} artifact: {name}")
        shared_outputs.update(record.get("shared_outputs", {}))
        previous = record["completion"]
    current_shared = {}
    for record in state.get("stages", {}).values():
        current_shared.update(record.get("shared_outputs", {}))
    for name, entry in current_shared.items():
        path = workspace / name
        if not path.is_file() or file_hash(path) != entry["hash"]:
            raise ValueError(f"Changed/missing shared conversion manifest: {name}")
    return previous


def restore_shared(state, registry, count, workspace):
    outputs = {}
    for stage in registry[:count]:
        outputs.update(state["stages"][stage["id"]].get("shared_outputs", {}))
    for name, entry in outputs.items():
        shutil.copyfile(workspace / entry["snapshot"], workspace / name)


def complete_stage(state, stage, identity, workspace, output):
    paths = []
    directory = workspace / stage["dir"]
    shared_outputs = {}
    for number, name in (("01", "pages_manifest.json"), ("05", "chapters_manifest.json"), ("10", "chapters_manifest.json")):
        if stage["id"] == number:
            snapshot = directory / ".manifests" / name
            snapshot.parent.mkdir(exist_ok=True)
            shutil.copyfile(workspace / name, snapshot)
            shared_outputs[name] = {"hash": file_hash(snapshot), "snapshot": snapshot.relative_to(workspace).as_posix()}
    paths.extend(("workspace", p) for p in sorted(directory.rglob("*")) if p.is_file())
    if stage["id"] == "14" and output is not None:
        paths.extend(("output", p) for p in sorted(output.rglob("*")) if p.is_file() and p.suffix != ".json")
    files = {}
    for anchor, path in paths:
        if path.suffix == ".json":
            json.loads(path.read_text(encoding="utf-8"))
        base = workspace if anchor == "workspace" else output
        files[f"{anchor}:{path.relative_to(base).as_posix()}"] = file_hash(path)
    if not files:
        raise ValueError(f"Stage {stage['id']} produced no artifacts")
    record = {"status": "completed", "identity": identity, "files": files, "shared_outputs": shared_outputs}
    record["completion"] = digest(record)
    state.setdefault("stages", {})[stage["id"]] = record
    write_state(workspace, state)
    return record["completion"]
