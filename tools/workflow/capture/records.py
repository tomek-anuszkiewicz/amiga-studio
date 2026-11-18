"""Collect or independently inspect one registered local execution archive."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
from hashlib import sha256
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
from tempfile import TemporaryDirectory
from uuid import uuid4

from record_sources import (
    archived_turns, cli_turn, common_dir, identifier, json_lines, linked, otlp_items,
    otlp_selection, protected_name, raw_selection, safe_file, selected_rollouts,
    Rollout, snapshot, utc,
)
from registry import git


FORMAT = 1
TOOL = "records.py/1"
ROWS = {
    "starting_inputs": "Starting inputs",
    "registration_lineage": "Execution registration and lineage",
    "communication": "Communication",
    "tools_skills": "Tools and skills",
    "timing_progress": "Timing and progress",
    "usage_verification": "Usage and verification",
    "change_history": "Change history",
    "integrity_survival": "File integrity and survival",
}
KINDS = {"cli": "cli", "otel": "otel", "artifact": "artifacts"}
SAFE_COMMIT = re.compile(r"(?:[a-f0-9]{40}|[a-f0-9]{64})\Z")
REVISION_ID = re.compile(r"[0-9]{8}T[0-9]{12}Z-[a-f0-9]{8}\Z")
WINDOWS_DEVICE = re.compile(r"(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?\Z", re.I)


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def json_object(path):
    value = json.loads(safe_file(path).read_bytes())
    if not isinstance(value, dict):
        raise ValueError(f"Expected an object in {path}")
    return value


def digest(data):
    return sha256(data).hexdigest()


def archive_path(value):
    if not isinstance(value, str):
        raise ValueError("Archive path must be a string")
    parts = value.split("/")
    if any(
        not part or part in {".", ".."} or part.endswith((".", " "))
        or any(c in part for c in '\\:<>"|?*\0')
        or WINDOWS_DEVICE.fullmatch(part)
        for part in parts
    ):
        raise ValueError(f"Unsafe archive path: {value!r}")
    return PurePosixPath(value)


def archive_context(args):
    for value in (args.task_id, args.execution_id):
        archive_path(identifier(value))
    primary = args.primary.absolute()
    for path in (primary / ".agent/task-archives", primary / ".agent", primary, *primary.parents):
        if linked(path):
            raise ValueError("Private archive cannot use a linked path")
    worktrees = git(primary, "worktree", "list", "--porcelain", "-z")
    if worktrees.returncode or not worktrees.stdout.startswith(b"worktree "):
        raise ValueError("Cannot identify the surviving primary checkout")
    first = worktrees.stdout.split(b"\0", 1)[0].removeprefix(b"worktree ")
    if Path(os.fsdecode(first)).resolve() != primary.resolve():
        raise ValueError("Use the surviving primary checkout to locate the private archive")
    primary = primary.resolve()
    repository = common_dir(primary)
    archive = primary / ".agent/task-archives" / args.task_id / args.execution_id
    if any(linked(path) for path in (archive, *archive.parents)):
        raise ValueError("Private archive cannot use a linked path")
    if git(primary, "check-ignore", "--quiet", "--", str(archive / "probe")).returncode:
        raise ValueError("Private archive directory must be ignored by Git before capture")
    tracked = git(primary, "ls-files", "-z", "--", ".agent/task-archives")
    if tracked.returncode or tracked.stdout:
        raise ValueError("Private archive files must stay outside Git")
    return primary, repository, archive


def check_registration(registration, args, repository):
    for key in ("task_id", "execution_id"):
        if registration[key] != getattr(args, key):
            raise ValueError("Registration identity differs from the requested execution")
    if Path(registration["git_common_dir"]).resolve() != repository:
        raise ValueError("Registration does not match the selected Git repository")
    if registration.get("pending_after_turn_id") is not None:
        raise ValueError("The execution boundary is still awaiting its control turn")
    if registration.get("end_utc") is None:
        raise ValueError("The execution still has an open end boundary")
    start, end = utc(registration["start_utc"]), utc(registration["end_utc"])
    if end < start:
        raise ValueError("Execution end precedes its start")
    ids = set()
    for item in registration["threads"]:
        thread = identifier(item["id"])
        if thread in ids or not start <= utc(item["start_utc"]) <= utc(item["end_utc"]) <= end:
            raise ValueError("Invalid registered thread boundaries")
        ids.add(thread)
    if registration["root_thread_id"] not in ids:
        raise ValueError("The registered root thread is missing")


def full_commit(value):
    if not isinstance(value, str) or not SAFE_COMMIT.fullmatch(value):
        raise ValueError("Expected a full Git commit identity")
    return value


def commit_exists(primary, value):
    return git(primary, "cat-file", "-e", f"{full_commit(value)}^{{commit}}").returncode == 0


def archive_file(directory, inventory, relative, data, kind, **metadata):
    key = archive_path(relative).as_posix()
    if key.casefold() in {item["path"].casefold() for item in inventory}:
        raise ValueError(f"Duplicate archive path: {key}")
    target = directory / key
    target.parent.mkdir(parents=True, exist_ok=True)
    target.touch(exist_ok=False)
    target.write_bytes(data)
    inventory.append({"path": key, "size": len(data), "sha256": digest(data), "kind": kind, **metadata})


def explicit_file(spec, source_root, roots, codex_home):
    kind = spec["kind"]
    destination = archive_path(spec["path"])
    if kind not in KINDS or destination.parts[0] != KINDS[kind] or len(destination.parts) < 2:
        raise ValueError("Selected file type must match its archive subdirectory")
    source = (source_root / spec["source"]).absolute()
    if any(part == ".." or ":" in part for part in source.parts[1:]):
        raise ValueError("Selected file path uses parent traversal or an alternate stream")
    canonical = source.resolve()
    if (not any(canonical.is_relative_to(root) for root in roots)
            or canonical.is_relative_to(codex_home)
            or protected_name(source) or protected_name(canonical)):
        raise ValueError("Selected file is outside the allowed artifact roots or names a protected location")
    safe_file(source, single_link=True)
    return kind, source, destination.as_posix()


def collect_otel(directory, inventory, source, destination, data, lineage, bounds, gaps, route):
    rows, tail = json_lines(data)
    if tail:
        gaps.append({"source": destination, "reason": tail, "provisional": True})
    segments = []
    evidence = []
    excluded = encoded_again = output_offset = 0
    for row in rows:
        try:
            selected, dropped = otlp_selection(row.value, lineage, bounds, route=route)
        except (KeyError, TypeError, ValueError, AttributeError, OverflowError):
            gaps.append({"source": destination, "reason": f"Invalid OTLP envelope at line {row.number}"})
            continue
        excluded += dropped
        if selected is None:
            continue
        if dropped:
            encoded_again += 1
            output = json.dumps(selected, ensure_ascii=False, separators=(",", ":")).encode() + b"\n"
        else:
            output = row.raw
        segments.append(output)
        evidence.append({
            "start": row.start, "end": row.end, "source_line": row.number,
            "archive_start": output_offset, "archive_end": output_offset + len(output),
            "encoded_again": bool(dropped),
        })
        output_offset += len(output)
    if segments:
        archive_file(directory, inventory, destination, b"".join(segments), "otel", source=str(source),
                     ranges=evidence, encoded_again_lines=encoded_again, excluded_records=excluded)
    if not segments:
        gaps.append({"source": destination, "reason": "No attributable in-range OTLP records"})


def latest_record(rollout):
    if not rollout.selected:
        return None
    row = rollout.selected[-1]
    payload = row.value["payload"]
    return {
        "timestamp": row.value["timestamp"], "source_line": row.number,
        "byte_start": row.start, "byte_end": row.end, "type": row.value["type"],
        "event": payload.get("type"), "turn_id": next(
            (key for key, turn in rollout.turns.items() if row in turn["rows"]), None
        ), "call_id": payload.get("call_id"),
    }


def archive_artifact(directory, inventory, destination, source, data):
    try:
        first = json.loads(data.split(b"\n", 1)[0])
    except (ValueError, UnicodeError):
        first = None
    if isinstance(first, dict) and (
        first.get("type") in ("session_meta", "thread.started")
        or set(first) & {"resourceLogs", "resourceSpans", "resourceMetrics"}
    ):
        raise ValueError("Runtime streams must pass the matching source selection")
    archive_file(directory, inventory, destination, data, "artifact", source=str(source))


def collect(args, primary, repository, archive):
    registration_path = archive / "registration.json"
    registration_raw = safe_file(registration_path).read_bytes()
    registration = json.loads(registration_raw)
    check_registration(registration, args, repository)
    spec_path = args.input.absolute()
    spec = json_object(spec_path)
    start_commit = full_commit(spec["start_commit"])
    workflow_commit = full_commit(spec["workflow_commit"])
    final_commit = spec.get("final_commit")
    if final_commit is not None:
        full_commit(final_commit)
    if not isinstance(spec["outcome"], str) or not spec["outcome"].strip():
        raise ValueError("A terminal task outcome is required")
    source_root = spec_path.parent
    roots = {source_root.resolve(), primary.resolve()}
    roots.update((source_root / entry).resolve() for entry in spec.get("artifact_roots", []))
    scoped_files = []
    gaps = []
    for entry in spec.get("files", []):
        try:
            kind, source, destination = explicit_file(entry, source_root, roots, args.codex_home)
        except (KeyError, OSError, ValueError, TypeError) as error:
            gaps.append({"source": entry.get("path") if isinstance(entry, dict) else None, "reason": str(error)})
            continue
        scoped_files.append((kind, source, destination, entry))
    spool = archive / "otel"
    if spool.exists():
        if linked(spool):
            raise ValueError("Telemetry spool is a linked directory")
        for source in sorted(spool.iterdir()):
            if source.is_file() and ".jsonl" in source.name and not linked(source):
                scoped_files.append(("otel", source, f"otel/{source.name}", {}))
    lineage, bounds, rollout_gaps = selected_rollouts(args.codex_home, registration, repository)
    gaps.extend(
        {"source": f"rollouts/{thread}.jsonl", "reason": message, "provisional": True}
        for thread, message in rollout_gaps.items()
    )
    revisions = archive / "revisions"
    if linked(revisions):
        raise ValueError("Archive revision directory is a link")
    revisions.mkdir(exist_ok=True)
    fmt = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    revision = f"{fmt}-{uuid4().hex[:8]}"
    previous = sorted(name.name for name in revisions.iterdir()
                      if REVISION_ID.fullmatch(name.name) and not linked(name) and (name / "manifest.json").is_file())
    with TemporaryDirectory(prefix=".pending-", dir=revisions) as tmp:
        directory = Path(tmp) / revision
        directory.mkdir()
        inventory = []
        archive_file(directory, inventory, "registration.json", registration_raw, "registration")
        threads = []
        for thread, item in lineage.items():
            location = f"rollouts/{thread}.jsonl"
            if item.selected:
                data, ranges = raw_selection(item.selected)
                archive_file(directory, inventory, location, data, "rollout", source=str(item.path),
                             ranges=ranges, whole_source=len(item.lines) == len(item.selected) and not item.gap)
            threads.append({
                "thread_id": thread, "parent_thread_id": item.parent, "source": str(item.path),
                "selection": bounds[thread], "turn_ids": sorted(item.covered_turns),
                "last_event": latest_record(item), "segment": location if item.selected else None,
                "cli_version": item.header["payload"].get("cli_version"),
            })
        for kind, source, destination, entry in scoped_files:
            try:
                safe_file(source, single_link=True)
                data, changing = snapshot(source)
                if changing:
                    gaps.append({"source": destination, "reason": "Source changed during the bounded read",
                                 "provisional": True})
                if kind == "otel":
                    collect_otel(directory, inventory, source, destination, data, lineage, bounds, gaps,
                                 f"{args.task_id}/{args.execution_id}")
                elif kind == "cli":
                    rows, tail = json_lines(data)
                    if tail:
                        gaps.append({"source": destination, "reason": tail, "provisional": True})
                    thread, turn, terminal = cli_turn(rows, lineage, entry.get("turn_id"), allow_incomplete=True)
                    if not terminal:
                        gaps.append({"source": destination, "reason": "CLI terminal event is absent",
                                     "provisional": True})
                    archive_file(directory, inventory, destination, b"".join(row.raw for row in rows), kind,
                                 source=str(source), thread_id=thread, turn_id=turn, complete=terminal and tail is None)
                else:
                    archive_artifact(directory, inventory, destination, source, data)
            except (OSError, ValueError) as error:
                gaps.append({"source": destination, "reason": str(error)})
        if registration.get("otel") and not any(item["kind"] == "otel" for item in inventory):
            gaps.append({"source": "otel/", "reason": "No verified receiver records were available"})
        for label, commit in [
            ("start_commit", start_commit), ("workflow_commit", workflow_commit), ("final_commit", final_commit),
        ]:
            if commit is not None and not commit_exists(primary, commit):
                gaps.append({"source": label, "reason": "Commit is unavailable in the selected repository"})
        manifest = {
            "format_version": FORMAT, "tool_version": TOOL, "revision": revision,
            "previous_revision": previous[-1] if previous else None,
            "task_id": args.task_id, "execution_id": args.execution_id,
            "git_common_dir": str(repository),
            "root_thread_id": registration["root_thread_id"],
            "start_utc": registration["start_utc"], "end_utc": registration["end_utc"],
            "start_commit": start_commit, "workflow_commit": workflow_commit,
            "initial_input_limits": spec.get("initial_input_limits", []),
            "final_commit": final_commit, "final_candidate": spec.get("final_candidate"),
            "outcome": spec["outcome"], "client": spec.get("client"), "backup": spec.get("backup"),
            "collection_status": "provisional" if any(gap.get("provisional") for gap in gaps) else
                                 "partial" if gaps else "collected",
            "threads": threads, "files": sorted(inventory, key=lambda item: item["path"]), "gaps": gaps,
        }
        if safe_file(registration_path).read_bytes() != registration_raw:
            raise ValueError("Execution registration changed during collection")
        (directory / "manifest.json").write_bytes(json_bytes(manifest))
        directory.rename(revisions / revision)
    print(json.dumps({"revision": revision, "archive": str(revisions / revision),
                      "status": manifest["collection_status"], "gaps": gaps}, ensure_ascii=False))
    return 0




def files_from_disk(directory):
    files = {}
    problems = []
    for where, subdirs, names in os.walk(directory):
        current = Path(where)
        for name in subdirs[:]:
            if linked(current / name):
                problems.append(f"Linked directory found: {(current / name).relative_to(directory).as_posix()}")
                subdirs.remove(name)
        for name in names:
            path = current / name
            relative = path.relative_to(directory).as_posix()
            if relative == "manifest.json":
                continue
            try:
                files[relative] = safe_file(path, single_link=True).read_bytes()
            except (OSError, ValueError):
                problems.append(f"Unreadable or linked archive file: {relative}")
    return files, problems


def inventory_issues(manifest, files):
    problems = []
    claimed = set()
    for item in manifest["files"]:
        path = archive_path(item["path"]).as_posix()
        if path.casefold() in claimed:
            problems.append(f"Duplicate inventory path: {path}")
        claimed.add(path.casefold())
        if path not in files:
            problems.append(f"Inventory file missing: {path}")
        elif len(files[path]) != item["size"] or digest(files[path]) != item["sha256"]:
            problems.append(f"Size or SHA-256 mismatch: {path}")
        elif item["kind"] == "rollout":
            data = files[path]
            ranges = item["ranges"]
            if not ranges or sum(part["end"] - part["start"] for part in ranges) != len(data):
                problems.append(f"Invalid rollout byte ranges: {path}")
                continue
            offset = 0
            previous_end = 0
            previous_line = 0
            for part in ranges:
                size = part["end"] - part["start"]
                lines = data[offset:offset + size].count(b"\n")
                if (size <= 0 or part["start"] < previous_end
                        or part["first_line"] <= previous_line
                        or lines != part["last_line"] - part["first_line"] + 1):
                    problems.append(f"Invalid rollout segment locator: {path}")
                    break
                offset += size
                previous_end = part["end"]
                previous_line = part["last_line"]
    problems.extend(f"File absent from inventory: {path}" for path in files if path.casefold() not in claimed)
    return problems


def expected_checks(test, files, manifest, primary, observer_root):
    kind = test["kind"]
    if kind == "commit_ref":
        commit = full_commit(test["commit"])
        ref = test["ref"]
        if not isinstance(ref, str) or not ref.startswith("refs/") or git(primary, "check-ref-format", ref).returncode:
            raise ValueError("Expected a full Git ref")
        result = git(primary, "merge-base", "--is-ancestor", commit, ref)
        return f"{ref} contains {commit}", result.returncode == 0
    path = test["path"]
    if path != "*":
        path = archive_path(path).as_posix()
    if kind == "git_blob":
        commit = full_commit(test["commit"])
        output = git(primary, "cat-file", "blob", f"{commit}:{path}")
        return f"{commit}:{path}", output.returncode == 0 and digest(output.stdout) == test["sha256"]
    if kind in {"contains", "absent"}:
        text = test["text"].encode("utf-8")
        if not text:
            raise ValueError("Expected marker must be nonempty")
        sources = files if path == "*" else {path: files[path]} if path in files else {}
        located = []
        for name, data in sources.items():
            at = data.find(text)
            if at >= 0:
                line = data.count(b"\n", 0, at) + 1
                located.append(f"{name}:{line} (byte {at})")
        return ", ".join(located) or path, bool(sources) and bool(located) == (kind == "contains")
    if path not in files:
        return path, False
    data = files[path]
    if kind == "archive":
        if "sha256" not in test and "size" not in test:
            raise ValueError("Expected archive check needs independently recorded bytes or size")
        return path, (("sha256" not in test or digest(data) == test["sha256"])
                      and ("size" not in test or len(data) == test["size"]))
    if kind == "source_range":
        source = safe_file((observer_root / test["source"]).absolute())
        ranges = test.get("ranges")
        if ranges is None:
            ranges = [{"start": test["start"], "end": test["end"]}]
        expected = []
        with source.open("rb") as stream:
            length = source.stat().st_size
            for part in ranges:
                if not 0 <= part["start"] < part["end"] <= length:
                    raise ValueError("Expected source range lies outside available source bytes")
                stream.seek(part["start"])
                expected.append(stream.read(part["end"] - part["start"]))
        claimed = next((item.get("ranges") for item in manifest["files"] if item["path"] == path), None)
        matching = claimed is not None and [
            (item["start"], item["end"]) for item in claimed
        ] == [(part["start"], part["end"]) for part in ranges]
        return f"{path} compared with {source.name} byte ranges " + ", ".join(
            f"[{part['start']}, {part['end']})" for part in ranges), matching and data == b"".join(expected)
    raise ValueError(f"Unknown independent check kind: {kind}")


def rollout_example(record, sample):
    payload = record["payload"]
    sample.update({k: payload[k] for k in ("turn_id", "call_id") if k in payload})
    event = payload.get("type")
    if record["type"] == "turn_context":
        sample.update({k: payload[k] for k in ("model", "effort") if k in payload})
        return "registration_lineage", "model and effort"
    if payload.get("phase") in {"commentary", "final_answer"} or event == "agent_message":
        return "communication", payload.get("phase") or "agent handoff"
    if event in {"function_call", "custom_tool_call", "function_call_output", "custom_tool_call_output"}:
        return "tools_skills", event
    if event in {"task_started", "task_complete", "turn_aborted", "turn_failed"}:
        sample.update({k: payload[k] for k in ("started_at", "completed_at", "duration_ms", "reason") if k in payload})
        return "timing_progress", event
    if record["type"] == "token_usage_record":
        sample["counters"] = {
            key: payload[key] for key in ("usage", "turn_token_usage", "thread_token_usage") if key in payload
        }
        return "usage_verification", "response, turn and thread usage"
    if event == "patch_apply_end":
        return "change_history", "patch result"
    item = payload.get("item")
    if event == "item_completed" and isinstance(item, dict) and item.get("type") == "FileChange":
        changes = item.get("changes", {}).values()
        sample["item_id"] = item.get("id")
        sample["status"] = item.get("status")
        sample["diff_bodies"] = sum(bool(change.get("unified_diff")) for change in changes)
        return "change_history", "native file change"
    return None


def locate_otel(record, example, examples):
    for index, (_, item, attrs) in enumerate(otlp_items(record), 1):
        if attrs is None:
            continue
        name = attrs.get("event.name") or item.get("name") or item.get("eventName")
        if not isinstance(name, str) or not name:
            continue
        sample = {**example, "record": index, "event_name": name}
        for key in (
            "conversation.id", "turn.id", "event.timestamp", "tool_name", "mcp_server",
            "duration_ms", "success", "skill.name", "output_truncated", "call_id",
            "model", "reasoning_effort", "event.kind",
        ):
            if key in attrs:
                sample[key] = attrs[key]
        for key in ("startTimeUnixNano", "endTimeUnixNano", "timeUnixNano",
                    "observedTimeUnixNano", "traceId", "spanId"):
            if key in item:
                sample[key] = item[key]
        counters = {key: value for key, value in attrs.items() if key.endswith("_token_count")}
        bucket = "tools_skills" if any(part in name for part in ("tool", "skill", "mcp")) else "timing_progress"
        if counters:
            sample["counters"] = counters
            examples["usage_verification"].setdefault(f"OTLP {name}", sample)
        examples[bucket].setdefault(f"OTLP {name}", sample)


def original_line(line, ranges):
    for part in ranges:
        count = part["last_line"] - part["first_line"] + 1
        if line <= count:
            return part["first_line"] + line - 1
        line -= count
    return None


def source_examples(files, manifest):
    examples = {row: {} for row in ROWS}
    for entry in manifest["files"]:
        path = entry["path"]
        kind = entry["kind"]
        if path not in files or kind not in {"rollout", "cli", "otel"}:
            continue
        rows, _ = json_lines(files[path])
        for row in rows:
            record = row.value
            example = {"path": path, "line": row.number, "archive_byte": row.start}
            if kind == "rollout":
                example["source_line"] = original_line(row.number, entry.get("ranges", []))
                example["thread_id"] = PurePosixPath(path).stem
            elif kind == "otel" and row.number <= len(entry.get("ranges", [])):
                example["source_line"] = entry["ranges"][row.number - 1]["source_line"]
            elif kind == "cli":
                example["thread_id"] = entry["thread_id"]
                example["turn_id"] = entry["turn_id"]
            if record.get("timestamp"):
                example["timestamp"] = record["timestamp"]
            try:
                if kind == "rollout":
                    group = rollout_example(record, example)
                    if group is not None:
                        examples[group[0]].setdefault(group[1], example)
                elif kind == "cli":
                    if record.get("type") == "turn.completed" and "usage" in record:
                        examples["usage_verification"].setdefault(
                            "CLI turn usage", {**example, "counters": record["usage"], "turn_id": entry["turn_id"]})
                    item = record.get("item", {})
                    if (record.get("type") == "item.completed" and item.get("type") == "command_execution"
                            and isinstance(item.get("exit_code"), int)):
                        label = "CLI command failure" if item["exit_code"] else "CLI successful command"
                        examples["usage_verification"].setdefault(label, {**example, "exit_code": item["exit_code"]})
                    if item.get("type") == "file_change":
                        examples["change_history"].setdefault("CLI file change notification", example)
                else:
                    locate_otel(record, example, examples)
            except (KeyError, TypeError, ValueError, AttributeError):
                continue
    return examples




def archive_correlations(files, manifest, registration):
    """Cross-check native, CLI and OTLP records against the sealed selection."""
    lineage = {}
    covered_roots = {}
    bounds = {item["id"]: item for item in registration["threads"]}
    expected = {}
    problems = []
    ranges = {entry["path"]: entry["ranges"] for entry in manifest["files"] if entry["kind"] == "rollout"}
    for claim in manifest["threads"]:
        thread = identifier(claim["thread_id"])
        segment = claim["segment"]
        if segment is None:
            continue
        if thread in expected or segment != f"rollouts/{thread}.jsonl":
            problems.append(f"Invalid thread segment mapping: {thread}")
            continue
        expected[thread] = claim
    if {name for name in files if name.startswith("rollouts/")} != {
        item["segment"] for item in expected.values() if item["segment"] in files
    }:
        problems.append("Rollout files differ from the declared thread mapping")
    deferred = list(expected)
    while deferred:
        still_deferred = []
        for thread in deferred:
            claim = expected[thread]
            segment = claim["segment"]
            if segment not in files:
                continue
            rows, tail = json_lines(files[segment])
            if not rows or tail:
                problems.append(f"Invalid archived rollout: {segment}")
                continue
            source_ranges = ranges.get(segment)
            if not source_ranges:
                problems.append(f"Archived rollout lacks source ranges: {segment}")
                continue
            first = rows[0].value
            header = first["payload"] if first["type"] == "session_meta" else {}
            parent = claim["parent_thread_id"]
            parent_roots = None
            if header and header.get("id") != thread:
                problems.append(f"Rollout header differs from its declared thread: {segment}")
                continue
            if thread not in bounds:
                if parent in expected and parent not in lineage:
                    still_deferred.append(thread)
                    continue
                try:
                    observed_parent = header["source"]["subagent"]["thread_spawn"]["parent_thread_id"]
                except (KeyError, TypeError):
                    observed_parent = None
                if parent not in lineage or parent != observed_parent:
                    problems.append(f"Descendant header does not verify lineage: {segment}")
                    continue
                bounds[thread] = {"start_utc": first["timestamp"], "end_utc": bounds[parent]["end_utc"]}
                parent_roots = covered_roots[parent]
            start, end = (utc(bounds[thread][key]) for key in ("start_utc", "end_utc"))
            if not all(start <= utc(row.value.get("timestamp")) <= end for row in rows):
                problems.append(f"Rollout records outside the registered range: {segment}")
                continue
            if claim["selection"]["start_utc"] != bounds[thread]["start_utc"] and (
                utc(claim["selection"]["start_utc"]) != start
            ):
                problems.append(f"Rollout start differs from its established bound: {segment}")
            if utc(claim["selection"]["end_utc"]) != end:
                problems.append(f"Rollout end differs from its established bound: {segment}")
            item = Rollout(Path(segment), thread, parent, start, None, first)
            item.selected = rows
            item.turns, leading_verified = archived_turns(rows, source_ranges[0], thread)
            item.covered_turns = set(item.turns)
            if not leading_verified:
                problems.append(f"Leading archived rollout records lack a verified native turn: {segment}")
            if parent_roots is None:
                covered_roots[thread] = item.covered_turns
            else:
                # Each level must carry the same root turn as a covered turn of its parent.
                covered_roots[thread] = set()
                for turn_id, turn in item.turns.items():
                    root_turn = turn["root"]
                    if isinstance(root_turn, str) and root_turn in parent_roots:
                        covered_roots[thread].add(root_turn)
                    else:
                        problems.append(f"Descendant turn {turn_id} lacks a covered ancestor root turn: {segment}")
            if thread != registration["root_thread_id"] and parent in lineage:
                parent_turns = set(lineage[parent].turns)
                if item.covered_turns & parent_turns:
                    problems.append(f"Descendant contains copied parent turns: {segment}")
            if set(claim["turn_ids"]) != item.covered_turns:
                problems.append(f"Archived native events do not verify the declared turn identities: {segment}")
            if thread == registration["root_thread_id"] and registration.get("after_turn_id") in item.covered_turns:
                problems.append(f"Rollout includes the excluded control turn: {segment}")
            last = latest_record(item)
            last_range = source_ranges[-1]
            last.update(source_line=last_range["last_line"],
                        byte_start=last_range["end"] - len(rows[-1].raw),
                        byte_end=last_range["end"])
            if claim["last_event"] != last:
                problems.append(f"Last archived event differs from its declared locator: {segment}")
            lineage[thread] = item
        if len(still_deferred) == len(deferred):
            problems.extend(f"Unverified archived parent chain for {thread}" for thread in still_deferred)
            break
        deferred = still_deferred
    for entry in manifest["files"]:
        path, kind = entry["path"], entry["kind"]
        if path not in files or kind not in {"cli", "otel"}:
            continue
        rows, tail = json_lines(files[path])
        if tail or not rows:
            problems.append(f"Invalid archived {kind} JSONL: {path}")
            continue
        if kind == "cli":
            try:
                thread, turn, _ = cli_turn(
                    rows, lineage, entry["turn_id"], allow_incomplete=not entry.get("complete", True)
                )
                if thread != entry["thread_id"] or turn not in lineage[thread].covered_turns:
                    raise ValueError("CLI thread and turn do not match the archive mapping")
            except (KeyError, TypeError, ValueError) as error:
                problems.append(f"{path}: {error}")
        else:
            route = f"{registration['task_id']}/{registration['execution_id']}"
            for row in rows:
                try:
                    result, removed = otlp_selection(row.value, lineage, bounds, route=route)
                    if result is None or removed:
                        raise ValueError("OTLP envelope includes unattributed or out-of-range records")
                except (KeyError, TypeError, ValueError, AttributeError, OverflowError) as error:
                    problems.append(f"{path}:{row.number}: {error}")
    return problems


def inspect(args, primary, repository, archive):
    archive_path(identifier(args.revision))
    directory = archive / "revisions" / args.revision
    manifest_path = directory / "manifest.json"
    manifest = json_object(manifest_path)
    files, issues = files_from_disk(directory)
    issues.extend(inventory_issues(manifest, files))
    if manifest.get("format_version") != FORMAT or manifest.get("revision") != args.revision:
        issues.append("Manifest format or revision identity differs")
    correlation_issues = []
    try:
        selected_reg = json.loads(files["registration.json"])
        check_registration(selected_reg, args, repository)
        for key in ("task_id", "execution_id", "root_thread_id", "start_utc", "end_utc"):
            if manifest.get(key) != selected_reg[key]:
                issues.append(f"Manifest differs from archived registration: {key}")
        if Path(manifest["git_common_dir"]).resolve() != repository:
            issues.append("Manifest differs from the registered Git identity")
        correlation_issues.extend(archive_correlations(files, manifest, selected_reg))
    except (KeyError, TypeError, ValueError, OSError):
        issues.append("Archived registration is missing or invalid")
    observer = json_object(args.expect.absolute())
    observer_rows = observer["rows"]
    examples = source_examples(files, manifest)
    identity_issues = [
        f"Observer expected a different {key}" for key, value in observer.get("identity", {}).items()
        if key not in manifest or manifest[key] != value
    ]
    expected_threads = observer.get("threads", [])
    actual_threads = {entry["thread_id"] for entry in manifest["threads"] if entry["segment"] in files}
    if expected_threads:
        identity_issues.extend(f"Expected thread segment missing: {key}" for key in sorted(set(expected_threads) - actual_threads))
        identity_issues.extend(f"Unexpected thread segment: {key}" for key in sorted(actual_threads - set(expected_threads)))
    reports = {}
    for row, title in ROWS.items():
        acceptance = observer_rows.get(row, {})
        findings = []
        observations = []
        for test in acceptance.get("checks", []):
            try:
                locator, verified = expected_checks(test, files, manifest, primary, args.expect.absolute().parent)
            except (KeyError, TypeError, ValueError, OSError) as error:
                locator, verified = str(error), False
            observations.append({"kind": test.get("kind"), "locator": locator, "verified": verified})
            if not verified:
                findings.append(f"Independent {test.get('kind')} evidence failed: {locator}")
        if row == "registration_lineage":
            findings.extend(identity_issues)
            findings.extend(correlation_issues)
            if not expected_threads:
                findings.append("Independent expected thread segment inventory unavailable")
        if row == "integrity_survival":
            findings.extend(issues)
            findings.extend(f"{gap['source']}: {gap['reason']}" for gap in manifest["gaps"])
        findings.extend(acceptance.get("gaps", []))
        if observations and all(item["verified"] for item in observations):
            status = "partial" if findings else "present"
        elif any(item["verified"] for item in observations):
            status = "partial"
        elif findings or observations:
            status = "missing"
        elif acceptance.get("unavailable"):
            status = "unavailable"
            findings.append(acceptance["unavailable"])
        elif acceptance.get("not_exercised"):
            status = "not_exercised"
            findings.append(acceptance["not_exercised"])
        else:
            status = "unavailable"
            findings.append("No independent expected observations supplied")
        reports[row] = {
            "title": title, "status": status, "checks": observations,
            "source_examples": examples[row], "findings": findings,
        }
    sufficient = all(row["status"] == "present" for row in reports.values())
    result = {
        "format_version": FORMAT, "tool_version": TOOL,
        "task_id": args.task_id, "execution_id": args.execution_id,
        "revision": args.revision, "manifest_sha256": digest(manifest_path.read_bytes()),
        "observer_sha256": digest(args.expect.read_bytes()),
        "client": manifest.get("client"), "start_utc": manifest.get("start_utc"),
        "end_utc": manifest.get("end_utc"), "backup": manifest.get("backup"),
        "outcome": manifest.get("outcome"), "collection_status": manifest.get("collection_status"),
        "start_commit": manifest.get("start_commit"), "workflow_commit": manifest.get("workflow_commit"),
        "final_commit": manifest.get("final_commit"),
        "verdict": "sufficient" if sufficient else "gaps",
        "integrity": "failed" if issues else "passed",
        "rows": reports,
    }
    output = args.output.absolute() if args.output else archive / "checks" / uuid4().hex
    if ".." in output.parts or any(linked(path) for path in (output, *output.parents)):
        raise ValueError("Report output cannot use linked paths or parent traversal")
    if output.is_relative_to(archive / "revisions"):
        raise ValueError("Reports must be stored apart from immutable archive revisions")
    ancestor = next((path for path in output.parents if path.exists()), None)
    if ancestor is None or git(ancestor, "check-ignore", "--quiet", "--", str(output / "result.json")).returncode:
        raise ValueError("Report output must stay under an ignored local path")
    output.mkdir(parents=True)
    (output / "result.json").write_bytes(json_bytes(result))
    (output / "result.md").write_text(render(result), encoding="utf-8", newline="\n")
    print(json.dumps({"verdict": result["verdict"], "integrity": result["integrity"], "report": str(output)}))
    return 0 if sufficient else 1


def render(report):
    lines = [
        "# Execution archive inspection",
        "",
        f"Execution `{report['task_id']}/{report['execution_id']}`",
        f"Revision `{report['revision']}`",
        f"Client `{report['client']}`",
        f"Range `{report['start_utc']}` to `{report['end_utc']}`",
        f"Independent backup `{report['backup']}`",
        f"Task outcome `{report['outcome']}`. Collection status `{report['collection_status']}`.",
        f"Starting commit `{report['start_commit']}`",
        f"Governing workflow commit `{report['workflow_commit']}`",
        f"Final commit `{report['final_commit']}`",
        f"Verdict **{report['verdict']}**. File integrity **{report['integrity']}**.",
        f"Manifest SHA-256 `{report['manifest_sha256']}`",
        f"Observer SHA-256 `{report['observer_sha256']}`",
    ]
    for row in report["rows"].values():
        lines.extend(("", f"## {row['title']}", "", f"Status **{row['status']}**.", ""))
        for check in row["checks"]:
            label = "Matched" if check["verified"] else "Failed"
            lines.append(f"- {label} {check['kind']} evidence at `{check['locator']}`")
        lines.extend(f"- {finding}" for finding in row["findings"])
        for label, source in row["source_examples"].items():
            info = json.dumps(source, ensure_ascii=False)
            lines.append(f"- {label} at `{info}`")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--primary", type=Path, required=True, help="Surviving primary checkout")
    common.add_argument("--task-id", required=True)
    common.add_argument("--execution-id", required=True)
    common.add_argument("--codex-home", type=Path, default=Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex"))
    commands = parser.add_subparsers(dest="operation", required=True)
    collector = commands.add_parser("collect", parents=[common])
    collector.add_argument("--input", type=Path, required=True, help="Selected file and Git input identities")
    checker = commands.add_parser("check", parents=[common])
    checker.add_argument("--revision", required=True)
    checker.add_argument("--expect", type=Path, required=True, help="Independently observed data and byte expectations")
    checker.add_argument("--output", type=Path, help="Fresh ignored directory for the paired report")
    args = parser.parse_args()
    args.codex_home = args.codex_home.resolve()
    try:
        primary, repository, archive = archive_context(args)
        if args.operation == "collect":
            return collect(args, primary, repository, archive)
        return inspect(args, primary, repository, archive)
    except (KeyError, ValueError, TypeError, OSError) as error:
        print(f"{args.operation}: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
