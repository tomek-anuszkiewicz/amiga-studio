"""Validate PDF predecessor completion and configuration before cleanup or reuse."""

from pathlib import Path
import hashlib
import json
import shutil

from conversion.cache import CONTRACT_VERSION, digest
from conversion.transport import SDK_VERSION

SHARED_MANIFESTS = {"01": ("pages_manifest.json",), "05": ("chapters_manifest.json",), "10": ("chapters_manifest.json",)}


def record_path(workspace, output, relative):
    """Resolve workspace/output artifacts and the single shared prepared PDF."""
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


def first_incomplete_stage(state, registry, workspace, output):
    """Resume at the first missing result, without repairing modified artifacts."""
    for index, stage in enumerate(registry):
        record = state.get("stages", {}).get(stage["id"])
        if not record or record.get("status") != "completed":
            return index
        for relative in record["files"]:
            try:
                path = record_path(workspace, output, relative)
            except FileNotFoundError:
                return index
            if not path.is_file():
                return index
    return len(registry)


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
    shared = Path(__file__).resolve().parents[2] / "conversion"
    pdf_common = Path(__file__).resolve().parent
    procedure = {p.relative_to(directory).as_posix(): file_hash(p) for p in sorted(directory.iterdir()) if p.suffix in (".py", ".md")}
    hashes = {f"{directory.name}/{p.name}": file_hash(p)
              for directory in (shared, pdf_common)
              for p in sorted(directory.glob("*.py"))}
    procedure["shared"] = digest(hashes)
    return {"contract": CONTRACT_VERSION, "sdk_runtime": SDK_VERSION, "stage": stage["dir"],
            "selection": config["llm"]["stages"].get(stage["dir"]),
            "settings": {key: value for key, value in config.items() if key != "llm"},
            "source": source, "predecessor": predecessor, "procedure": procedure}


def artifact_predecessor(state, registry, stage):
    position = next(i for i, item in enumerate(registry) if item["id"] == stage["id"])
    inputs = stage.get("inputs", [registry[position-1]["id"]] if position else [])
    completions = {key: state.get("stages", {}).get(key, {}).get("completion") for key in inputs}
    if any(value is None for value in completions.values()):
        raise ValueError(f"Missing validated artifact inputs for Stage {stage['id']}")
    return next(iter(completions.values())) if len(completions) == 1 else (completions or None)


def validate_prefix(state, registry, count, config, source, skill_dir, workspace, output):
    previous = None
    shared_outputs = {}
    for stage in registry[:count]:
        record = state.get("stages", {}).get(stage["id"])
        if not record or record.get("status") != "completed":
            raise ValueError(f"Missing validated predecessor: regenerate Stage {stage['id']}")
        if record.get("completion") != digest({key: value for key, value in record.items() if key != "completion"}):
            raise ValueError(f"Damaged Stage {stage['id']} completion record")
        expected = stage_identity(stage, config, source, artifact_predecessor(state, registry, stage), skill_dir)
        if record["identity"] != expected:
            raise ValueError(f"Incompatible conversion: regenerate Stage {stage['id']} and dependents")
        for relative, expected_hash in record["files"].items():
            path = record_path(workspace, output, relative)
            if not path.is_file() or file_hash(path) != expected_hash:
                raise ValueError(f"Changed/missing Stage {stage['id']} artifact: {relative}")
        from .pdf_artifacts import validate_stage_artifacts
        validate_stage_artifacts(stage, workspace, source, snapshot=True)
        shared_outputs.update(record.get("shared_outputs", {}))
        previous = record["completion"]
    current_shared = {}
    for record in state.get("stages", {}).values():
        current_shared.update(record.get("shared_outputs", {}))
    for name in shared_outputs:
        entry = current_shared[name]
        path = workspace / name
        # Missing working copies are reconstructed from validated snapshots.
        # Only manifests needed by retained predecessors affect a restart.
        if path.exists() and (not path.is_file() or file_hash(path) != entry["hash"]):
            raise ValueError(f"Changed shared conversion manifest: {name}")
    return previous


def restore_shared(state, registry, count, workspace):
    outputs = {}
    for stage in registry[:count]:
        outputs.update(state["stages"][stage["id"]].get("shared_outputs", {}))
    for name in {name for names in SHARED_MANIFESTS.values() for name in names} - outputs.keys():
        (workspace / name).unlink(missing_ok=True)
    for name, entry in outputs.items():
        shutil.copyfile(workspace / entry["snapshot"], workspace / name)


def complete_stage(state, stage, identity, workspace, output):
    from .pdf_artifacts import validate_stage_artifacts
    validate_stage_artifacts(stage, workspace, identity["source"])
    paths = []
    directory = workspace / stage["dir"]
    shared_outputs = {}
    for name in SHARED_MANIFESTS.get(stage["id"], ()):
        snapshot = directory / ".manifests" / name
        snapshot.parent.mkdir(exist_ok=True)
        shutil.copyfile(workspace / name, snapshot)
        shared_outputs[name] = {"hash": file_hash(snapshot), "snapshot": snapshot.relative_to(workspace).as_posix()}
    paths.extend(("workspace", p) for p in sorted(directory.rglob("*")) if p.is_file())
    if stage.get("artifact_contract") == "pdf_text_layer":
        from .pdf_artifacts import validate_text_layer
        _, pdf = validate_text_layer(workspace, identity["source"])
        paths.append(("prepared", pdf))
    if stage["id"] == "14" and output is not None:
        paths.extend(("output", p) for p in sorted(output.rglob("*")) if p.is_file() and p.suffix != ".json")
    files = {}
    for anchor, path in paths:
        if path.suffix == ".json":
            json.loads(path.read_text(encoding="utf-8"))
        base = workspace if anchor == "workspace" else (path.parent if anchor == "prepared" else output)
        files[f"{anchor}:{path.relative_to(base).as_posix()}"] = file_hash(path)
    if not files:
        raise ValueError(f"Stage {stage['id']} produced no artifacts")
    record = {"status": "completed", "identity": identity, "files": files, "shared_outputs": shared_outputs}
    record["completion"] = digest(record)
    state.setdefault("stages", {})[stage["id"]] = record
    write_state(workspace, state)
    return record["completion"]
