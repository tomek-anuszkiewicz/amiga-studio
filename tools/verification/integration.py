"""Freeze a reviewed candidate and conditionally publish its mechanical commits."""
import json
from pathlib import Path
import subprocess

from evidence import BatchEvidence, capture_inputs, digest, execution_conditions, file_digest, group_identity
from repository import candidate_tree, collect_changes, git, merge_heads, resolve_base, snapshot, TIMING_REPORT
from integration_state import (ancestor, checkout, clean, collisions, common, no_operation,
                               publication_lock, RefTransaction, represents, save, value)
from integration_sources import cleanup_record, source_locks, validate_sources


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def validate_evidence(root, directory):
    """Only current canonical completion evidence can cross the commit boundary."""
    record = read(directory / "commit.json")
    base = record.get("integration_base")
    if not base or record.get("outcome") != "passed":
        raise ValueError("Passing canonical commit evidence with an integration base is required")
    original_source = record.get("integration_source")
    batch = BatchEvidence(root, directory, "commit", base, original_source)
    batch.load()
    if batch.record != record or record.get("source") != snapshot(root):
        raise ValueError("Completion evidence does not identify the current candidate")
    inputs = capture_inputs(root)
    conditions = execution_conditions(root)
    if record.get("conditions") != conditions or read(directory / record["inputs"]) != inputs:
        raise ValueError("Completion inputs or execution conditions changed")
    if set(record.get("scope", ())) - {TIMING_REPORT} != collect_changes(root, base) - {TIMING_REPORT}:
        raise ValueError("Integration scope changed")
    # Recompute selection, rather than accepting a truncated required list.
    from cli import parse_args
    from commands import select_phases
    from repository import divergent_integration
    _, args = parse_args(["commit", "--integration-base", base])
    args.integration_divergent = divergent_integration(root, base, original_source)
    phases = select_phases(args, set(record["scope"]))
    groups = [("Coverage" if phase == "Coverage" else items[0][0], items)
              for phase, commands in phases
              for items in ([commands] if phase == "Coverage" else [[item] for item in commands]) if items]
    if record.get("required") != [name for name, _ in groups]:
        raise ValueError("Required completion groups do not match current selection")
    for name, commands in groups:
        # The verifier gives SingleStep a results directory before recording commands.
        commands = [(title, command, {**env, "SINGLESTEP_RESULTS_DIR": str(directory / "singlestep")}
                     if "tools/verification/run_singlestep.py" in command else env)
                    for title, command, env in commands]
        if batch.reason(name, group_identity(name, commands, inputs, conditions)):
            raise ValueError("Missing or stale completion evidence: " + name)
    return record, inputs


def freeze(root, directory, review_path, partition_path, target, sources_path=None):
    record, inputs = validate_evidence(root, directory)
    review, partition = read(review_path), read(partition_path)
    if (review.get("verdict") != "APPROVED" or not review.get("reviewer")
            or review.get("source") != record["source"]
            or review.get("integration_base") != record["integration_base"]
            or review.get("partition") != digest(partition)):
        raise ValueError("Independent approval must identify this source, baseline and partition")
    source_ref = value(root, "symbolic-ref", "HEAD")
    target_ref = "refs/heads/" + target
    if source_ref == target_ref:
        raise ValueError("Primary-branch work uses ordinary preparation, without self-integration")
    target_root = checkout(root, target_ref)
    base = record["integration_base"]
    sources = (read(sources_path) if sources_path else
               [{"ref": source_ref, "root": str(root.resolve()), "tip": record["head"]}])
    validate_sources(root, sources, source_ref, target_ref, record["head"], merge_heads(root))
    if sources_path and review.get("sources") != digest(sources):
        raise ValueError("Independent approval must identify the selected source set")
    if value(root, "rev-parse", target_ref) != base:
        raise ValueError("Target advanced; synchronize and prepare against its new immutable baseline")
    if not isinstance(partition, list):
        raise ValueError("Partition must contain prepared messages and tree IDs")
    start = record["head"]
    commits = []
    pending = collect_changes(root)
    for item in partition:
        if set(item) != {"message", "tree"}:
            raise ValueError("Each prepared commit requires exactly message and tree")
        tree = item["tree"]
        paths = value(root, "diff", "--name-only", "--no-renames", "-z", start, tree).split("\0")
        if (not item["message"].strip() or value(root, "cat-file", "-t", tree) != "tree"
                or any(path not in pending for path in paths if path)):
            raise ValueError("Prepared trees must partition only pending candidate paths")
        commits.append({"message": item["message"], "tree": tree})
        start = tree
    start = value(root, "rev-parse", start + "^{tree}" )
    changed_output = set(value(root, "diff", "--name-only", "--no-renames", "-z", record["tree"], start).split("\0")) - {""}
    if start != candidate_tree(root) or changed_output - {TIMING_REPORT} or (pending and not commits):
        raise ValueError("Partition does not consume the complete verified candidate")
    # Timing output is generated after testing; bind its approved bytes.
    record = {**record, "tree": start}
