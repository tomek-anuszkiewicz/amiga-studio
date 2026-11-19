"""Preserve only the selected whole files in one private execution directory."""

from dataclasses import dataclass
import os
from pathlib import Path, PurePosixPath
import shutil
from tempfile import NamedTemporaryFile

from record_sources import identifier, linked, safe_file
from registry import Project, git, read_json, thread_id, timestamp


@dataclass(frozen=True)
class SelectedFile:
    source: Path
    destination: PurePosixPath
    complete: bool


def selected_files(spec, archive_path):
    if set(spec) != {"files"} or not isinstance(spec["files"], list) or not spec["files"]:
        raise ValueError("Preservation requires a nonempty list of known files")
    selected = {}
    for item in spec["files"]:
        if not isinstance(item, dict) or set(item) != {"id", "source", "path", "complete"}:
            raise ValueError("Each file needs id, source, path and producer completion")
        key = identifier(item["id"])
        source = Path(item["source"])
        if key in selected or not source.is_absolute() or ".." in source.parts or type(item["complete"]) is not bool:
            raise ValueError(f"Invalid or repeated preservation selection: {key}")
        selected[key] = SelectedFile(source, archive_path(item["path"]), item["complete"])
    return selected


def reject_links(path):
    if any(linked(part) for part in (path, *path.parents)):
        raise ValueError(f"Private archive cannot use a linked path: {path}")


def require_ignored(primary, destination):
    if git(primary, "check-ignore", "--quiet", "--", str(destination)).returncode:
        raise ValueError(f"Private archive file must be ignored: {destination}")


def copy_whole(primary, source, destination):
    """Publish a complete copy under an exclusive name without reading occupied files."""
    reject_links(destination)
    require_ignored(primary, destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    reject_links(destination)
    with NamedTemporaryFile(dir=destination.parent, prefix=".pending-", delete=False) as output:
        temporary = Path(output.name)
        try:
            with source.open("rb") as original:
                shutil.copyfileobj(original, output)
            output.close()
            for index in range(1, 2**31):
                target = destination if index == 1 else destination.with_name(
                    f"{destination.stem}-{index}{destination.suffix}")
                reject_links(target)
                if target != destination:
                    require_ignored(primary, target)
                try:
                    os.link(temporary, target)
                except FileExistsError:
                    continue
                return target
            raise ValueError(f"No available archive name: {destination}")
        finally:
            output.close()
            temporary.unlink()


def registered_execution(primary, task_id, execution_id):
    """Read only the known registration and return its stable identity."""
    project = Project(primary, primary)
    path = project.registration_path(task_id, execution_id)
    project.require_ignored(path)
    try:
        registration = read_json(safe_file(path, single_link=True))
        if (registration["schema_version"] != 1
                or registration["task_id"] != task_id or registration["execution_id"] != execution_id
                or not project.matches_common(registration["git_common_dir"])
                or "pending_after_turn_id" in registration):
            raise ValueError("Registration does not identify this active or completed execution")
        start = registration["start_utc"]
        timestamp(start)
        return {"task_id": task_id, "execution_id": execution_id,
                "root_thread_id": thread_id(registration["root_thread_id"]), "start_utc": start}
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise ValueError(f"Cannot use the existing capture registration for {task_id}/{execution_id}: {error}") from error


def copy_selected(primary, archive, selected, state, save_progress):
    """Remember completed paths through ordinary retries when closure has a control record."""
    initial = {key: {"source": str(item.source), "path": item.destination.as_posix()}
               for key, item in selected.items()}
    previous = state.setdefault("files", initial)
    if previous.keys() != selected.keys():
        raise ValueError("Keep the same required file selection when resuming preservation")
    for key, item in selected.items():
        stored = previous[key]
        resumed = (archive / stored["destination"]) if "destination" in stored else None
        if ((str(item.source) != stored["source"] or item.destination.as_posix() != stored["path"])
                and (resumed is None or item.source != resumed)):
            raise ValueError(f"Preservation selection changed for {key}")
    save_progress()

    for key, item in selected.items():
        stored = previous[key]
        source = item.source
        if "destination" in stored:
            destination = safe_file(archive / stored["destination"])
            require_ignored(primary, destination)
            print(f"[RETAINED] {key}: {stored['destination']}", flush=True)
            continue
        if source.is_relative_to(archive):
            destination = safe_file(source)
            require_ignored(primary, destination)
            label = "RETAINED"
        elif source.is_relative_to(archive.parent.parent):
            raise ValueError(f"Selected file belongs to another private execution: {source}")
        elif not item.complete:
            raise ValueError(f"Selected producer is still active: {source}")
        else:
            try:
                destination = copy_whole(primary, safe_file(source, single_link=True), archive / item.destination)
            except (ValueError, OSError) as error:
                raise ValueError(f"Cannot preserve {source} as {item.destination}: {error}") from error
            label = "COPIED"
        stored["destination"] = destination.relative_to(archive).as_posix()
        save_progress()
        print(f"[{label}] {key}: {stored['destination']}", flush=True)
