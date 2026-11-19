"""Freeze a reviewed candidate and conditionally publish its mechanical commits."""
import json
from pathlib import Path
import subprocess

from evidence import BatchEvidence, capture_inputs, digest, execution_conditions, file_digest, group_identity
from repository import candidate_tree, collect_changes, git, merge_heads, resolve_base, snapshot
from integration_state import (ancestor, checkout, clean, collisions, common, no_operation,
                               publication_lock, RefTransaction, represents, save, value, worktrees)
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
    if set(record.get("scope", ())) != collect_changes(root, base):
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
    registrations = worktrees(root)
    target_root = checkout(root, target_ref, registrations)
    base = record["integration_base"]
    sources = (read(sources_path) if sources_path else
               [{"ref": source_ref, "root": str(root.resolve()), "tip": record["head"]}])
    validate_sources(root, sources, source_ref, target_ref, record["head"], merge_heads(root),
                     registrations=registrations)
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
    start = value(root, "rev-parse", start + "^{tree}")
    if start != candidate_tree(root) or start != record["tree"] or (pending and not commits):
        raise ValueError("Partition does not consume the complete verified candidate")
    frozen = {"version": 2, "verification": "canonical", "base": base, "head": record["head"], "source_ref": source_ref,
              "sources": sources, "sources_explicit": sources_path is not None,
              "source_root": str(root.resolve()), "target_ref": target_ref, "target_root": str(target_root),
              "tree": start, "merge_heads": merge_heads(root), "commits": commits,
              "review": review, "evidence": record, "inputs": inputs, "state": "frozen",
              "evidence_directory": str(directory.resolve()),
              "review_artifacts": {str(path.resolve()): file_digest(path)
                                   for path in (review_path, partition_path, *([sources_path] if sources_path else []))}}
    if snapshot(root) != record["source"] or capture_inputs(root) != inputs:
        raise ValueError("Candidate changed during freeze")
    path = directory / "integration.json"
    save(path, frozen)
    return path


def verify_confirmation(root, frozen):
    """A user-confirmed tested tip is an explicit exception, not test evidence."""
    confirmation = frozen["confirmation"]
    artifact = Path(frozen["confirmation_path"])
    if (not artifact.is_file() or file_digest(artifact) != frozen["confirmation_digest"]
            or read(artifact) != confirmation):
        raise ValueError("Fast-forward confirmation artifact changed")
    if (confirmation.get("plan_approved") is not True or confirmation.get("tested") is not True
            or not isinstance(confirmation.get("confirmation"), str)
            or not confirmation["confirmation"].strip()):
        raise ValueError("Explicit plan approval and tested-commit confirmation are required")
    if (resolve_base(root, confirmation["base"]) != frozen["base"]
            or resolve_base(root, confirmation["commit"]) != frozen["commit"]
            or confirmation["tree"] != frozen["tree"]):
        raise ValueError("Fast-forward confirmation identifies a different result")
    clean(root)
    if (value(root, "symbolic-ref", "HEAD") != frozen["source_ref"]
            or value(root, "rev-parse", "HEAD", "HEAD^{tree}").splitlines() != [frozen["commit"], frozen["tree"]]):
        raise ValueError("Confirmed source branch or content changed")
    ancestor(root, frozen["base"], frozen["commit"])


def accept_tested(root, directory, confirmation_path, target):
    """Seal only an unchanged clean fast-forward explicitly confirmed as tested."""
    confirmation = read(confirmation_path)
    source_ref = value(root, "symbolic-ref", "HEAD")
    target_ref = "refs/heads/" + target
    if source_ref == target_ref:
        raise ValueError("Primary-branch work cannot use fast-forward confirmation")
    registrations = worktrees(root)
    target_root = checkout(root, target_ref, registrations)
    if checkout(root, source_ref, registrations) != root.resolve():
        raise ValueError("Expected the selected source checkout")
    frozen = {"version": 2, "verification": "confirmed-fast-forward", "state": "sealed",
              "base": confirmation["base"], "commit": confirmation["commit"], "tree": confirmation["tree"],
              "source_ref": source_ref, "source_root": str(root.resolve()),
              "sources": [{"ref": source_ref, "root": str(root.resolve()), "tip": confirmation["commit"]}],
              "target_ref": target_ref, "target_root": str(target_root),
              "confirmation": confirmation, "confirmation_path": str(confirmation_path.resolve()),
              "confirmation_digest": file_digest(confirmation_path)}
    verify_confirmation(root, frozen)
    clean(target_root, untracked=False)
    if value(root, "rev-parse", target_ref) != frozen["base"]:
        raise ValueError("Target advanced; obtain approval of an updated plan")
    path = directory / "integration.json"
    save(path, frozen)
    return path


def verify_seal(root, frozen):
    if Path(frozen["source_root"]).resolve() != root.resolve():
        raise ValueError("Mechanical completion must run in the selected source checkout")
    review = frozen["review"]
    if frozen["sources_explicit"] and review.get("sources") != digest(frozen["sources"]):
        raise ValueError("Frozen selected source approval changed")
    if (review.get("verdict") != "APPROVED" or not review.get("reviewer")
            or review.get("source") != frozen["evidence"]["source"]
            or review.get("integration_base") != frozen["base"]
            or review.get("partition") != digest(frozen["commits"])
            or frozen["tree"] != frozen["evidence"]["tree"]):
        raise ValueError("Frozen review or candidate identity changed")
    for name, expected in frozen["review_artifacts"].items():
        artifact = Path(name)
        if not artifact.is_file() or file_digest(artifact) != expected:
            raise ValueError("Frozen review or partition artifact changed")
    clean(root)
    if value(root, "symbolic-ref", "HEAD") != frozen["source_ref"]:
        raise ValueError("Source branch changed")
    current = value(root, "rev-parse", "HEAD")
    sequence = value(root, "rev-list", "--first-parent", "--reverse", frozen["head"] + ".." + current).splitlines()
    if len(sequence) != len(frozen["commits"]):
        raise ValueError("Commit count differs from the frozen partition")
    parent = frozen["head"]
    tree = None
    for index, (commit, prepared) in enumerate(zip(sequence, frozen["commits"])):
        parents, tree, message = value(root, "show", "-s", "--format=%P%x00%T%x00%B", commit).split("\0", 2)
        expected = [parent, *(frozen["merge_heads"] if index == 0 else [])]
        if parents.split() != expected or tree != prepared["tree"] or message.strip() != prepared["message"].strip():
            raise ValueError("Commit differs from the frozen tree, parents or message")
        parent = commit
    if parent != current:
        raise ValueError("HEAD does not consume the frozen history")
    if tree is None:
        tree = value(root, "rev-parse", current + "^{tree}")
    if tree != frozen["tree"]:
        raise ValueError("Final commit differs from the frozen tree")
    ancestor(root, frozen["base"], current)
    validate_sources(root, frozen["sources"], frozen["source_ref"], frozen["target_ref"], current)
    actual = capture_inputs(root)
    expected = {name: content for name, content in frozen["inputs"].items() if content is not None}
    if {name: content for name, content in actual.items() if content is not None} != expected:
        raise ValueError("Verified inputs changed across mechanical completion")
    if execution_conditions(root) != frozen["evidence"]["conditions"]:
        raise ValueError("Execution conditions changed across mechanical completion")
    return current


def seal(root, path):
    frozen = read(path)
    if frozen["state"] != "frozen":
        raise ValueError("Expected a frozen candidate")
    frozen.update(commit=verify_seal(root, frozen), state="sealed")
    save(path, frozen)


def check_source(root, frozen):
    if value(root, "rev-parse", frozen["source_ref"]) != frozen["commit"]:
        raise ValueError("Source branch advanced after sealing")
    source = checkout(root, frozen["source_ref"])
    if source != Path(frozen["source_root"]).resolve():
        raise ValueError("Source checkout changed after sealing")
    if frozen["verification"] == "confirmed-fast-forward":
        validate_sources(source, frozen["sources"], frozen["source_ref"], frozen["target_ref"], frozen["commit"])
        verify_confirmation(source, frozen)
        return
    if frozen["verification"] != "canonical" or verify_seal(source, frozen) != frozen["commit"]:
        raise ValueError("Source checkout or verification mode changed after sealing")
    # Retained logs remain acceptance evidence after mechanical commits.
    record = frozen["evidence"]
    directory = Path(frozen["evidence_directory"])
    for name in record["required"]:
        item = record["groups"][name]
        if item["status"] != "passed" or not item["artifacts"]:
            raise ValueError("Missing passing frozen evidence: " + name)
        for relative, expected in item["artifacts"].items():
            artifact = (directory / relative).resolve()
            if not artifact.is_relative_to(directory) or not artifact.is_file() or file_digest(artifact) != expected:
                raise ValueError("Frozen evidence artifact changed: " + relative)


def publish(root, path):
    frozen = read(path)
    if frozen["state"] != "sealed":
        raise ValueError("Publication requires sealed mechanical completion")
    common_directory = common(root)
    phase_path = common_directory / "local-integration.json"
    with publication_lock(common_directory):
        if phase_path.exists():
            raise ValueError("An unfinished publication requires integrate recover first")
        check_source(root, frozen)
        target = checkout(root, frozen["target_ref"])
        if target != Path(frozen["target_root"]).resolve():
            raise ValueError("Target checkout ownership changed")
        if value(target, "symbolic-ref", "HEAD") != frozen["target_ref"]:
            raise ValueError("Unexpected target HEAD")
        clean(target, untracked=False)
        if value(root, "rev-parse", frozen["target_ref"]) != frozen["base"]:
            raise ValueError("Target advanced; synchronize and verify the new integration baseline")
        collisions(target, frozen["commit"])
        transaction = RefTransaction(root, frozen["target_ref"], frozen["base"], frozen["source_ref"], frozen["commit"],
                                     sources=source_locks(frozen))
        try:
            clean(target, untracked=False)
            if value(target, "symbolic-ref", "HEAD") != frozen["target_ref"]:
                raise ValueError("Target HEAD changed while preparing reference locks")
            phase = {"record": str(path.resolve()), "record_digest": file_digest(path), "candidate": frozen}
            save(phase_path, phase)
            git(target, "read-tree", "-m", "-u", frozen["base"], frozen["commit"])
            if not represents(target, frozen["tree"]):
                raise ValueError("Target checkout changed during publication; recovery is required")
            transaction.commit()
            if (value(target, "rev-parse", "HEAD") != frozen["commit"]
                    or value(target, "symbolic-ref", "HEAD") != frozen["target_ref"]
                    or not represents(target, frozen["tree"])):
                raise ValueError("Publication committed; target checkout needs recovery")
            frozen["state"] = "published"
            save(cleanup_record(frozen), frozen)
            save(path, frozen)
            phase_path.unlink()
        finally:
            transaction.close()


def recover(root):
    common_directory = common(root)
    phase_path = common_directory / "local-integration.json"
    with publication_lock(common_directory):
        phase = read(phase_path)
        frozen = phase["candidate"]
        target = checkout(root, frozen["target_ref"])
        if target != Path(frozen["target_root"]).resolve() or value(target, "symbolic-ref", "HEAD") != frozen["target_ref"]:
            raise ValueError("Recovery target ownership changed; preserve files and request a decision")
        no_operation(target)
        current = value(root, "rev-parse", frozen["target_ref"])
        if current not in {frozen["base"], frozen["commit"]}:
            raise ValueError("Target reference changed; preserve files and request a decision")
        transaction = RefTransaction(root, frozen["target_ref"], current, frozen["source_ref"], frozen["commit"], verify_only=True)
        try:
            if value(target, "symbolic-ref", "HEAD") != frozen["target_ref"]:
                raise ValueError("Recovery target HEAD changed")
            no_operation(target)
            expected_tree = value(root, "rev-parse", current + "^{tree}")
            if current == frozen["commit"]:
                if not represents(target, frozen["tree"]):
                    raise ValueError("Committed checkout has unexpected edits; preserve files and request a decision")
                path = Path(phase["record"])
                if file_digest(path) != phase["record_digest"] and read(path) != {**frozen, "state": "published"}:
                    raise ValueError("Recovery evidence changed; preserve files and request a decision")
                frozen["state"] = "published"
                save(cleanup_record(frozen), frozen)
                save(path, frozen)
            else:
                if represents(target, frozen["tree"]):
                    collisions(target, current)
                    git(target, "read-tree", "-m", "-u", frozen["commit"], current)
                elif not represents(target, expected_tree):
                    raise ValueError("Interrupted checkout has unexpected edits; preserve files and request a decision")
            if (value(target, "symbolic-ref", "HEAD") != frozen["target_ref"]
                    or value(root, "rev-parse", frozen["target_ref"]) != current
                    or not represents(target, expected_tree)):
                raise ValueError("Recovery checkout changed; preserve the phase record")
            # Keep the prepared verification lock until the phase is cleared.
            # No reference update is needed: closing aborts this read-only transaction.
            phase_path.unlink()
        finally:
            transaction.close()


def dispatch(root, args):
    from cli import keyword_values
    if args.plan or args.exact or args.integration_base or args.integration_source:
        raise ValueError("Integration operations do not accept preview, exact or comparison options")
    if args.action in {"freeze", "accept-tested"}:
        keys = ("review", "partition", "target") if args.action == "freeze" else ("confirmation", "target")
        values = keyword_values(args.words, (*keys, "sources") if args.action == "freeze" else keys)
        if set(values) - {"sources"} != set(keys):
            raise ValueError("Use integrate " + args.action + " " + " ".join(key + " VALUE" for key in keys))
        directory = (root / (args.results_dir or ".agent/tmp/verification/commit")).resolve()
        if not directory.is_relative_to(root / ".agent/tmp"):
            raise ValueError("Integration records belong under .agent/tmp in the source checkout")
        if args.action == "freeze":
            path = freeze(root, directory, root / values["review"], root / values["partition"], values["target"],
                          root / values["sources"] if "sources" in values else None)
            print("[PASS] Frozen integration candidate: " + str(path))
        else:
            path = accept_tested(root, directory, root / values["confirmation"], values["target"])
            print("[INFO] Existing tested tip accepted by explicit confirmation; no new verification was run")
            print("[PASS] Sealed unchanged fast-forward: " + str(path))
    elif args.action == "recover":
        if args.words or args.results_dir:
            raise ValueError("Use integrate recover from a surviving checkout")
        recover(root)
        print("[PASS] Recovered local publication state")
    else:
        values = keyword_values(args.words, ("record",))
        if set(values) != {"record"} or args.results_dir:
            raise ValueError("Use integrate " + args.action + " record PATH")
        path = (root / values["record"]).resolve()
        if args.action == "cleanup":
            from integration_cleanup import cleanup
            cleanup(root, path)
        else:
            {"seal": seal, "publish": publish}[args.action](root, path)
            if args.action == "publish":
                print("[INFO] Surviving cleanup record: " + str(cleanup_record(read(path))))
        print("[PASS] Local integration " + args.action)
    return 0
