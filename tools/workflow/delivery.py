"""Discover completed plans and bind a selected delivery to native integration."""

import argparse
from hashlib import sha256
import json
from pathlib import Path
import re
import subprocess
import sys

sys.path.append(str(Path(__file__).resolve().parents[1] / "verification"))
from integration import verify_confirmation
from integration_closure import capture_binding, closure_record, published_target
from integration_sources import cleanup_record
from integration_state import checkout, clean, common, no_operation, publication_lock, save, value, worktrees
from repository import candidate_tree, git, resolve_base, snapshot

from result import SCHEMA_PATH, portable, render, validate


MASTER = "refs/heads/master"
HEADS = "refs/heads/"
WORKFLOW = Path(".agent/tmp/workflow-TA")
INTEGRATIONS = Path(".agent/tmp/integration")
SCHEMA = json.loads(SCHEMA_PATH.read_bytes())
REMAINING = ("assembly", "verification", "review", "closure-content", "publication", "cleanup")
INPUT_ERRORS = (ValueError, RuntimeError, OSError, KeyError, TypeError)


def identifier(task):
    if not re.fullmatch(SCHEMA["properties"]["task_id"]["pattern"], task):
        raise ValueError("Expected a stable plan ID")
    return task


def local_path(root, name):
    if not portable(name) or not (root / name).resolve().is_relative_to(root):
        raise ValueError(f"Path escapes the recorded checkout: {name}")
    return root / name


def reference(root, path, *, optional=False):
    path = local_path(root, path)
    try:
        content = path.read_bytes()
    except FileNotFoundError:
        if optional:
            return None
        raise
    return {"path": path.relative_to(root).as_posix(), "sha256": sha256(content).hexdigest()}


def read(path):
    return json.loads(path.read_bytes())


def contains(root, older, newer):
    process = subprocess.run(
        ["git", "merge-base", "--is-ancestor", older, newer], cwd=root, capture_output=True)
    if process.returncode not in (0, 1):
        raise RuntimeError(process.stderr.decode("utf-8", errors="replace").strip())
    return process.returncode == 0


def target(root, entries):
    location = checkout(root, MASTER, entries)
    if Path(entries[0]["worktree"]).resolve() != location:
        raise ValueError("Local master must be in the primary checkout")
    return location, value(root, "rev-parse", MASTER)


def readiness_path(task):
    return WORKFLOW / identifier(task) / "readiness.json"


def amendment_path(plan_path):
    return Path(plan_path).with_suffix(".amendments.md").as_posix()


def resolved_amendments(text):
    """Follow explicit decisions by ID while retaining the recorded proposals."""
    decision_id = r"[A-Z][A-Z0-9]*(?:[-_.][A-Z0-9]+)+"
    disposition = re.compile(r"^[ \t]*\*\*(?:Disposition|Status)\.?\*\*[ \t]*:?[ \t]*(\w+)",
                             re.MULTILINE | re.IGNORECASE)
    decision = re.compile(
        rf"^[ \t]*(?:[-*][ \t]+)?\*\*({decision_id})[ \t]+(\w+)"
        r"(?: for this delivery)?\.?\*\*(?:[ \t\r]|$)", re.MULTILINE)
    sections = re.split(r"^##[ \t]+([^\r\n]+)\r?(?:\n|\Z)", text, flags=re.MULTILINE)
    record = {}
    anonymous = []
    for title, body in zip(["", *sections[1::2]], sections[::2]):
        proposal = re.match(rf"^({decision_id})(?=$|[ \t:])", title)
        statuses = disposition.findall(body)
        if len(statuses) > 1:
            return False
        if proposal:
            name = proposal[1]
            if name in record or not statuses:
                return False
            record[name] = statuses[0].lower()
        else:
            anonymous.extend(status.lower() for status in statuses)
        updated = set()
        for name, status in decision.findall(body):
            if name not in record or name in updated:
                return False
            record[name] = status.lower()
            updated.add(name)
    states = [*record.values(), *anonymous]
    return bool(states) and all(state in ("approved", "declined") for state in states)


def amendments(root, plan_path, *, require_resolved=False):
    name = amendment_path(plan_path)
    path = local_path(root, name)
    if not path.exists():
        return None
    content = path.read_bytes()
    if require_resolved and not resolved_amendments(content.decode("utf-8-sig")):
        raise ValueError("The material amendment record has a pending or unconfirmed disposition")
    return {"path": name, "sha256": sha256(content).hexdigest()}


def plan_identity(root, plan_path):
    if not Path(plan_path).is_relative_to(".agent/tasks") or not plan_path.endswith(".md"):
        raise ValueError("Use the authoritative Markdown plan under .agent/tasks")
    plan = local_path(root, plan_path).read_text(encoding="utf-8-sig")
    flags = re.MULTILINE | re.IGNORECASE | re.ASCII
    identity = re.search(r"^\*\*Task ID\*\*\s+`([A-Z0-9_.-]+)`", plan, flags)
    revision = re.search(r"^\*\*(?:Revision|Version)\*\*\s+([A-Z0-9_.-]+)", plan, flags)
    if identity is None or revision is None:
        raise ValueError("The plan must state its task ID and revision")
    return identifier(identity[1]), revision[1], reference(root, plan_path)


def capture_source(root, plan_path, registration):
    """Capture implementation output before writing the completed result pair."""
    task, _, plan = plan_identity(root, plan_path)
    primary, _ = target(root, worktrees(root))
    execution = identifier(read(registration)["execution_id"])
    expected = primary / ".agent/task-archives" / task / execution / "registration.json"
    if registration.resolve() != expected.resolve():
        raise ValueError("Select the authoritative registration for this source plan")
    binding = capture_binding(primary, task, execution)
    ref = value(root, "symbolic-ref", "HEAD")
    if ref == MASTER or checkout(root, ref) != root:
        raise ValueError("Capture completed implementation in its task checkout")
    no_operation(root)
    before = snapshot(root)
    head = value(root, "rev-parse", "HEAD")
    tree = candidate_tree(root)
    material = amendments(root, plan_path)
    if (snapshot(root) != before or value(root, "symbolic-ref", "HEAD") != ref
            or reference(root, plan_path) != plan or amendments(root, plan_path) != material):
        raise ValueError("Implementation or plan changed during capture")
    return {"task_id": task, "plan": plan, "amendments": material, "execution": binding,
            "ref": ref, "head": head, "tree": tree}


def completed_source(root, primary, task, plan, material, source, evidence, result_relative):
    rows = [item for item in evidence["evidence"] if item["id"] == "delivery-source"]
    if len(rows) != 1 or not rows[0]["artifact_ref"]:
        raise ValueError("The implementation result must identify its captured delivery source")
    receipt = rows[0]["artifact_ref"]
    if (Path(receipt["path"]).parent != result_relative.parent
            or reference(root, receipt["path"]) != receipt):
        raise ValueError("Captured delivery source evidence changed")
    output = read(local_path(root, receipt["path"]))
    if (output["task_id"], output["plan"], output["amendments"], output["ref"]) != (
            task, plan, material, source["ref"]):
        raise ValueError("Captured delivery source identifies different plan inputs or branch")
    execution = output["execution"]
    if capture_binding(primary, task, execution["execution_id"]) != execution:
        raise ValueError("Captured source execution registration changed")
    result_capture = evidence["observations"].get("capture")
    if result_capture and result_capture["execution_id"] != execution["execution_id"]:
        raise ValueError("The implementation result names a different capture execution")
    head = resolve_base(root, output["head"])
    tip = source["tip"]
    if head == tip:
        head_tree = tip_tree = value(root, "rev-parse", head + "^{tree}")
    else:
        head_tree, tip_tree = value(root, "rev-parse", "--revs-only", head + "^{tree}", tip + "^{tree}",
                                    "--").splitlines()
    if output["tree"] == head_tree:
        matches = head == tip
    else:
        # A working snapshot may be committed once, directly on its recorded HEAD.
        matches = value(root, "rev-list", "--parents", "-n", "1", tip).split() == [tip, head]
    if not matches or tip_tree != output["tree"]:
        raise ValueError("Committed tip or tree differs from captured implementation")
    return receipt, execution


def completion(root, primary, task, plan, material, source, result_path):
    path = local_path(root, result_path)
    evidence = read(path)
    errors = validate(evidence, SCHEMA, root, check_references=False)
    if errors:
        raise ValueError("Invalid implementation result: " + errors[0])
    expected = WORKFLOW / task / evidence["work_item_id"] / evidence["run_id"] / "result.json"
    if path.relative_to(root) != expected or evidence["task_id"] != task:
        raise ValueError("Implementation result belongs to a different task or invocation")
    if evidence["status"] != "completed" or evidence["result"]["kind"] != "implementation":
        raise ValueError("The implementation result has not completed")
    if any(item["severity"] in ("major", "blocking") and item["disposition"] in ("open", "deferred")
           for item in evidence["findings"]):
        raise ValueError("Implementation has an unresolved required finding")
    inputs = evidence["inputs"]["plans"]
    if plan not in inputs or any(reference(root, item["path"]) != item for item in inputs):
        raise ValueError("The implementation result refers to changed plan inputs")
    rendered_path = path.with_suffix(".md").relative_to(root).as_posix()
    if local_path(root, rendered_path).read_text(encoding="utf-8") != render(evidence):
        raise ValueError("Implementation result pair differs")
    receipt, execution = completed_source(root, primary, task, plan, material, source, evidence, path.relative_to(root))
    candidate = evidence["inputs"]["candidate"]
    if candidate is None or not contains(root, resolve_base(root, candidate["base_commit"]), source["tip"]):
        raise ValueError("The committed source does not contain the implementation result baseline")
    return {"json": reference(root, result_path), "text": reference(root, rendered_path),
            "inputs": inputs, "source_receipt": receipt, "execution": execution}


def source_identity(root, source, entries):
    if (checkout(root, source["ref"], entries) != root or value(root, "symbolic-ref", "HEAD") != source["ref"]
            or value(root, "rev-parse", "--revs-only", "HEAD", source["ref"], "--").splitlines()
            != [source["tip"], source["tip"]]):
        raise ValueError("Source checkout or committed tip changed")
    clean(root)


def verify_plan_inputs(root, primary, declaration):
    task, revision, plan = plan_identity(root, declaration["plan"]["path"])
    if (task, revision, plan) != (declaration["task_id"], declaration["revision"], declaration["plan"]):
        raise ValueError("The recorded plan or revision changed")
    if reference(root, amendment_path(plan["path"]), optional=True) != declaration["amendments"]:
        raise ValueError("The material amendment record changed")
    material = amendments(root, plan["path"], require_resolved=True)
    evidence = declaration["completion"]
    if completion(root, primary, task, plan, material, declaration["source"], evidence["json"]["path"]) != evidence:
        raise ValueError("The recorded implementation completion evidence changed")


def verify_readiness(declaration):
    if declaration["state"] != "ready" or not declaration["blocker_disposition"].strip():
        raise ValueError("The implementation is paused, pending or has no blocker disposition")
    remaining = declaration["remaining"]
    if (not remaining or set(remaining) - set(REMAINING) or len(set(remaining)) != len(remaining)
            or type(declaration["working_commit"]) is not bool):
        raise ValueError("Invalid remaining delivery steps or working-commit status")
    for task, revision in declaration["dependencies"].items():
        if identifier(task) == declaration["task_id"] or not revision:
            raise ValueError("Invalid plan dependency")


def declare(root, plan_path, result_path, disposition, remaining, dependencies, working_commit):
    """Record the main session's explicit implementation completion decision."""
    task, revision, plan = plan_identity(root, plan_path)
    branch = value(root, "symbolic-ref", "HEAD")
    source = {"ref": branch, "root": str(root), "tip": value(root, "rev-parse", "HEAD")}
    entries = worktrees(root)
    if branch == MASTER:
        raise ValueError("The authoritative plan must be in its own task checkout")
    source_identity(root, source, entries)
    material = amendments(root, plan_path, require_resolved=True)
    primary, _ = target(root, entries)
    evidence = completion(root, primary, task, plan, material, source, result_path)
    declaration = {
        "state": "ready", "task_id": task, "revision": revision, "plan": plan,
        "amendments": material,
        "source": source, "completion": evidence, "blocker_disposition": disposition,
        "remaining": list(remaining), "dependencies": dependencies, "working_commit": working_commit,
    }
    verify_readiness(declaration)
    path = local_path(root, readiness_path(task).as_posix())
    if path.exists():
        previous = read(path)
        if (previous["completion"].get("source_receipt") == evidence["source_receipt"]
                and previous["source"]["tip"] != source["tip"]):
            raise ValueError("This completed implementation already identifies a different committed tip")
    save(path, declaration)
    return path


def withdraw(root, task, reason):
    path = local_path(root, readiness_path(task).as_posix())
    declaration = read(path)
    if not reason.strip() or Path(declaration["source"]["root"]).resolve() != root:
        raise ValueError("Withdraw readiness from the owning checkout and record its reason")
    declaration.update(state="withdrawn", blocker_disposition=reason)
    save(path, declaration)
    return path


def publications(root, target_root, base):
    """Read confirmed delivery identities from the native surviving records."""
    published = {}
    for path in sorted((target_root / INTEGRATIONS).glob("*/integration.json")):
        frozen = read(path)
        if (frozen.get("state") != "published" or frozen.get("target_ref") != MASTER
                or Path(frozen["target_root"]).resolve() != target_root
                or cleanup_record(frozen) != path or not contains(root, frozen["commit"], base)):
            continue
        for plan in frozen.get("delivery", {}).get("plans", []):
            completions = published.setdefault(plan["task_id"], [])
            completions.append((plan["revision"], plan["source"], frozen["commit"], str(path)))
    return published


def discover(root):
    entries = worktrees(root)
    target_root, base = target(root, entries)
    published = publications(root, target_root, base)
    records, ownership = {}, {}
    common_directory = common(root).resolve()
    for entry in entries:
        location = Path(entry["worktree"]).resolve()
        if location != target_root:
            paths = list((location / WORKFLOW).glob("*/readiness.json"))
            ownership[location] = len(paths)
            for path in sorted(paths):
                records.setdefault(path.parent.name, []).append(location)
    ready, excluded = {}, {}
    for task, locations in records.items():
        if len(locations) != 1:
            excluded[task] = "Several registered checkouts declare ownership of the same plan"
            continue
        source_root = locations[0]
        try:
            if ownership[source_root] != 1:
                raise ValueError("The checkout declares ownership of several plans")
            declaration = read(local_path(source_root, readiness_path(task).as_posix()))
            source = declaration["source"]
            if task in published:
                raise ValueError("A surviving publication already records this plan")
            verify_readiness(declaration)
            if (declaration["task_id"] != task or Path(source["root"]).resolve() != source_root
                    or common(source_root).resolve() != common_directory):
                raise ValueError("The declaration names a different source checkout or task")
            if contains(root, resolve_base(root, source["tip"]), base):
                raise ValueError("The source tip is already in master; check the actual publication record")
            source_identity(source_root, source, entries)
            verify_plan_inputs(source_root, target_root, declaration)
            ready[task] = declaration
        except INPUT_ERRORS as error:
            excluded[task] = str(error)
    return target_root, base, ready, excluded, published


def dependency_order(plans, published):
    ordered, active, emitted = [], set(), set()

    def visit(task):
        if task in emitted:
            return
        if task in active:
            raise ValueError("Cyclic plan dependency at " + task)
        active.add(task)
        for needed, revision in sorted(plans[task]["dependencies"].items()):
            if needed in plans and plans[needed]["revision"] == revision:
                visit(needed)
            elif needed in plans or not any(item[0] == revision for item in published.get(needed, [])):
                raise ValueError(f"{task} needs selected or published {needed} revision {revision}")
        active.remove(task)
        emitted.add(task)
        ordered.append(plans[task])

    for task in sorted(plans):
        visit(task)
    return ordered


def offer(root):
    location, base, ready, excluded, published = discover(root)
    available = {}
    for task in ready:
        try:
            needed = {}
            pending = [task]
            while pending:
                current = pending.pop()
                if current not in needed:
                    needed[current] = ready[current]
                    pending.extend(dependency for dependency in ready[current]["dependencies"] if dependency in ready)
            dependency_order(needed, published)
            available[task] = ready[task]
        except ValueError as error:
            excluded[task] = str(error)
    plans = dependency_order(available, published)
    return {"target_root": str(location), "base": base, "plans": plans, "excluded": excluded,
            "route_choice": "combined-or-single" if len(plans) > 1 else "single" if plans else "none"}


def tested_confirmation(root, base, declaration, path):
    if path is None or declaration["working_commit"] or "closure-content" in declaration["remaining"]:
        return None
    source = declaration["source"]
    location = Path(source["root"])
    path = path.resolve()
    relative = path.relative_to(location).as_posix()
    evidence = reference(location, relative)
    if not Path(relative).is_relative_to(".agent/tmp"):
        raise ValueError("Tested-tip confirmation belongs in the selected checkout under .agent/tmp")
    frozen = {"commit": source["tip"], "base": base,
              "source_ref": source["ref"], "tree": value(root, "rev-parse", source["tip"] + "^{tree}"),
              "confirmation": read(path), "confirmation_path": str(path),
              "confirmation_digest": evidence["sha256"]}
    verify_confirmation(location, frozen)
    return evidence


def push_identity(root, remote):
    if not remote or remote.startswith("-") or remote not in value(root, "remote").splitlines():
        raise ValueError("Select one configured push remote by name")
    urls = git(root, "remote", "get-url", "--push", "--all", remote).splitlines()
    if len(urls) != 1 or not urls[0]:
        raise ValueError("The selected remote must have exactly one push destination")
    mirror = subprocess.run(["git", "config", "--bool", "--get", f"remote.{remote}.mirror"],
                            cwd=root, capture_output=True)
    if mirror.returncode not in (0, 1) or mirror.stdout.strip() == b"true":
        raise ValueError("The selected remote must support an ordinary push without mirroring")
    return {"remote": remote, "destination_sha256": sha256(urls[0]).hexdigest(), "ref": MASTER}


def select(root, offered, task_ids, answer, *, assembly_branch=None, confirmation=None, cleanup=(), remote=None):
    target_root, base, ready, excluded, published = discover(root)
    if not answer.strip() or not task_ids or len(task_ids) != len(set(task_ids)):
        raise ValueError("Record the user's answer and a unique set of plan IDs")
    if missing := [task for task in task_ids if task not in ready]:
        raise ValueError("Selected plan is ineligible: " + ", ".join(
            f"{task} ({excluded.get(task, 'no readiness declaration')})" for task in missing))
    if (offered["target_root"], offered["base"]) != (str(target_root), base):
        raise ValueError("Master changed since the offered delivery; reassess the selection")
    offered_plans = {plan["task_id"]: plan for plan in offered["plans"]}
    if any(offered_plans.get(task) != ready[task] for task in task_ids):
        raise ValueError("A selected plan changed since the offer; reassess the selection")
    plans = dependency_order({task: ready[task] for task in task_ids}, published)
    sources = [plan["source"] for plan in plans]
    assembly = None
    tested = None
    if len(plans) > 1:
        if (not assembly_branch or confirmation is not None
                or not re.fullmatch(r"[A-Z0-9_.-]+", assembly_branch, re.IGNORECASE | re.ASCII)):
            raise ValueError("Combined delivery needs a separate assembly branch and full verification")
        branch = HEADS + assembly_branch
        git(root, "check-ref-format", branch)
        assembly_root = target_root.with_name(target_root.name + "-" + assembly_branch)
        if (assembly_root.exists() or branch_exists(root, branch)
                or any(Path(entry["worktree"]).resolve() == assembly_root for entry in worktrees(root))):
            raise ValueError("The proposed assembly worktree or branch already exists")
        assembly = {"ref": branch, "root": str(assembly_root), "tip": base}
        sources = [assembly, *sources]
        route = "combined"
    elif assembly_branch:
        raise ValueError("Single delivery uses its existing task checkout")
    elif contains(root, base, sources[0]["tip"]):
        tested = tested_confirmation(root, base, plans[0], confirmation)
        route = "tested" if tested else "verify"
    else:
        route = "merge"
    permitted = {source["ref"] for source in sources}
    if len(cleanup) != len(set(cleanup)) or set(cleanup) - permitted:
        raise ValueError("Cleanup can include only explicitly selected source or assembly branches")
    return {"version": 1, "target_root": str(target_root), "base": base, "plans": plans, "answer": answer,
            "assembly": assembly, "sources": sources, "route": route, "confirmation": tested,
            "cleanup": list(cleanup), "push": push_identity(target_root, remote) if remote else None}


def branch_exists(root, branch):
    process = subprocess.run(["git", "show-ref", "--verify", "--quiet", branch], cwd=root)
    if process.returncode not in (0, 1):
        raise ValueError("Cannot verify branch availability")
    return process.returncode == 0


def selection_sources(selected):
    plans, assembly = selected["plans"], selected["assembly"]
    if (not plans or len({plan["task_id"] for plan in plans}) != len(plans)
            or (assembly is not None) != (len(plans) > 1)):
        raise ValueError("The selection must bind unique plans and the applicable assembly")
    original = [plan["source"] for plan in plans]
    sources = [assembly, *original] if assembly else original
    if (sources != selected["sources"] or len({source["ref"] for source in sources}) != len(sources)
            or set(selected["cleanup"]) - {source["ref"] for source in sources}):
        raise ValueError("The selected source or cleanup identities differ")
    if assembly and (assembly["tip"] != selected["base"] or selected["route"] != "combined"):
        raise ValueError("The selected assembly must start at recorded master")
    return plans, assembly, original


def revalidate(root, selected, *, provisioned=False):
    plans, assembly, _ = selection_sources(selected)
    target_root, base, ready, excluded, published = discover(root)
    if (str(target_root), base) != (selected["target_root"], selected["base"]):
        raise ValueError("Master changed; record the fresh baseline and reassess the delivery")
    for plan in plans:
        task = plan["task_id"]
        if ready.get(task) != plan:
            raise ValueError(f"Selected source changed: {task} ({excluded.get(task, 'readiness changed')})")
    if dependency_order({plan["task_id"]: plan for plan in plans}, published) != plans:
        raise ValueError("The dependency order changed")
    if assembly:
        location = Path(assembly["root"])
        entries = worktrees(root)
        if provisioned:
            source_identity(location, assembly, entries)
        elif branch_exists(root, assembly["ref"]) or location.exists() or any(
                Path(entry["worktree"]).resolve() == location for entry in entries):
            raise ValueError("Assembly location or branch is no longer available")
    if selected["confirmation"]:
        plan = selected["plans"][0]
        path = Path(plan["source"]["root"]) / selected["confirmation"]["path"]
        if tested_confirmation(root, base, plan, path) != selected["confirmation"]:
            raise ValueError("The tested-tip confirmation changed")
    return selected


def assembly_verification(frozen, plans):
    """Link the completed canonical attempt to the assembly and original executions."""
    primary = Path(frozen["target_root"]).resolve()
    batch = frozen.get("evidence", {})
    report = batch.get("case_report")
    if not report or batch.get("outcome") != "passed":
        raise ValueError("Combined delivery requires a completed assembly verification invocation")
    if "capture" not in frozen:
        raise ValueError("Combined delivery requires the selected assembly task and execution")
    journal = Path(report["path"]).with_suffix(".jsonl").as_posix()
    contents = local_path(primary, journal).read_bytes()
    events = [json.loads(line) for line in contents.splitlines()]
    if not events:
        raise ValueError("The canonical verification journal is empty")
    header, last = events[0], events[-1]
    expected = {"event": "batch_completed", "profile": "commit", "attempt": batch["attempt"],
                "source": batch["source"], "tree": frozen["tree"], "integration_base": frozen["base"]}
    execution = capture_binding(primary, header["task_id"], header["execution_id"])
    directory = Path(".agent/task-archives") / execution["task_id"] / execution["execution_id"] / "test-runs"
    sources = [plan["completion"]["execution"] for plan in plans]
    if (Path(journal).parent != directory or header.get("event") != "invocation"
            or header.get("producer") != "Verifier" or header.get("invocation_id") != report["invocation_id"]
            or header.get("revision") != batch["head"] or events.count(expected) != 1
            or last.get("event") != "finished" or last.get("exit_code") != 0 or last.get("evidence") != "recorded"
            or execution["task_id"] in {source["task_id"] for source in sources}
            or execution != frozen["capture"]):
        raise ValueError("The canonical verification journal does not identify the selected assembly attempt")
    receipt = {"path": journal, "sha256": sha256(contents).hexdigest()}
    return {"execution": execution, "case_report": report, "journal": receipt,
            "attempt": batch["attempt"], "sources": sources}


def bind_publication(root, path, frozen):
    """Bind the selected plan identities to the surviving native publication record."""
    selected = read(path)
    if (selected["target_root"], selected["base"], frozen["target_ref"]) != (
            frozen["target_root"], frozen["base"], MASTER):
        raise ValueError("Delivery selection identifies a different integration baseline")
    plans, assembly, original = selection_sources(selected)
    if assembly:
        if (len(plans) < 2 or selected["route"] != "combined"
                or assembly["ref"] != frozen["source_ref"] or selected["sources"] != frozen["sources"]
                or frozen.get("sources_explicit") is not True or frozen["verification"] != "canonical"):
            raise ValueError("Combined delivery must match the sealed, explicitly reviewed source set")
    elif (len(plans) != 1 or len(frozen["sources"]) != 1
          or (original[0]["ref"], original[0]["root"]) != (frozen["source_ref"], frozen["source_root"])
          or (selected["route"] == "tested") != (frozen["verification"] == "confirmed-fast-forward")):
        raise ValueError("Single delivery must match the selected checkout and acceptance route")
    if selected["route"] == "tested" and (
            original[0]["tip"] != frozen["commit"]
            or selected["confirmation"]["sha256"] != frozen["confirmation_digest"]):
        raise ValueError("The sealed tested tip uses a different confirmation")
    published = publications(root, Path(frozen["target_root"]), frozen["base"])
    if dependency_order({plan["task_id"]: plan for plan in plans}, published) != plans:
        raise ValueError("The recorded plan order no longer applies")
    for plan in plans:
        source = plan["source"]
        location = Path(source["root"])
        if plan["task_id"] in published or not contains(root, source["tip"], frozen["commit"]):
            raise ValueError("The selected source is already published or missing from the accepted commit")
        current = read(local_path(location, readiness_path(plan["task_id"]).as_posix()))
        if current != plan:
            raise ValueError("Selected source readiness changed")
        verify_readiness(plan)
        primary, _ = target(location, worktrees(location))
        verify_plan_inputs(location, primary, plan)
    return {**selected, "assembly_verification": assembly_verification(frozen, plans)} if assembly else selected


def preservation_inputs(frozen):
    """Return the known original evidence required before authorized teardown."""
    selection = frozen.get("delivery", {})
    required = {}
    for plan in selection.get("plans", []):
        source = plan["source"]
        if source["ref"] not in selection["cleanup"]:
            continue
        completion = plan["completion"]
        refs = [*completion["inputs"], completion["json"], completion["text"]]
        if "source_receipt" in completion:
            refs.append(completion["source_receipt"])
        if plan["amendments"] is not None:
            refs.append(plan["amendments"])
        location = Path(source["root"])
        required[plan["task_id"]] = {str(location / ref["path"]): ref for ref in refs}
    return required


def check_preservation_selection(frozen, state, selected):
    required = preservation_inputs(frozen)
    if not required:
        return
    previous = state.get("files", {})
    available = {previous.get(key, {}).get("source", str(item.source))
                 for key, item in (selected or {}).items()}
    if any(paths.keys() - available for paths in required.values()):
        raise ValueError("Preservation must include the exact known inputs of every selected cleanup plan")
    retained = {item["source"] for item in previous.values() if "destination" in item}
    for paths in required.values():
        for source, expected in paths.items():
            if source not in retained and sha256(Path(source).read_bytes()).hexdigest() != expected["sha256"]:
                raise ValueError(f"Selected evidence changed before preservation: {expected['path']}")


def preservation_receipts(root, frozen, state):
    archive = root / ".agent/task-archives" / state["task_id"] / state["execution_id"]
    by_source = {item["source"]: item for item in state.get("files", {}).values()}
    receipts = {}
    for task, inputs in preservation_inputs(frozen).items():
        files, missing = [], []
        for source, expected in inputs.items():
            destination = by_source.get(source, {}).get("destination")
            try:
                observed = reference(archive, destination) if destination else None
            except INPUT_ERRORS:
                observed = None
            if observed is None or observed["sha256"] != expected["sha256"]:
                missing.append(expected["path"])
            else:
                files.append({"source": expected, "archive": observed})
        receipts[task] = {"state": "pending" if missing else "preserved", "files": files, "missing": missing}
    return receipts


def record_preservation(root, frozen, state):
    receipts = preservation_receipts(root, frozen, state)
    if any(receipt["state"] != "preserved" for receipt in receipts.values()):
        raise ValueError("Preserved copies differ from required selected plan evidence")
    if receipts:
        state["plans"] = receipts


def require_preserved_source(root, frozen, branch, location):
    plans = frozen.get("delivery", {}).get("plans", [])
    for plan in plans:
        if plan["source"]["ref"] != branch:
            continue
        _, closure = closure_record(frozen)
        state = closure["preservation"]
        task = plan["task_id"]
        actual = preservation_receipts(root, frozen, state).get(task)
        if not actual or actual["state"] != "preserved" or actual != state.get("plans", {}).get(task):
            raise ValueError(f"Required preserved plan evidence is unavailable: {task}")
        if location.exists():
            primary, _ = target(location, worktrees(location))
            verify_plan_inputs(location, primary, plan)


def push(root, record):
    """Push the recorded published commit to its one authorized destination."""
    with publication_lock(common(root)):
        frozen = published_target(root, record)
        authorization = frozen.get("delivery", {}).get("push")
        if authorization is None:
            raise ValueError("This delivery grants no remote push")
        if not contains(root, frozen["commit"], value(root, "rev-parse", MASTER)):
            raise ValueError("Local master no longer contains the published delivery")
        path, closure = closure_record(frozen)
        previous = closure.get("push", {})
        if previous.get("state") == "pushed":
            return previous
        remote = authorization["remote"]
        if push_identity(root, remote) != authorization:
            raise ValueError("The recorded push destination changed")
        # A retry after interruption checks whether the selected ref already landed.
        closure["push"] = {"state": "pending", "destination": authorization, "commit": frozen["commit"]}
        save(path, closure)
        destination = value(root, "remote", "get-url", "--push", remote)
        observed = subprocess.run(["git", "ls-remote", "--refs", "--", destination, MASTER],
                                  cwd=root, capture_output=True)
        already_pushed = observed.returncode == 0 and observed.stdout.split() == [
            frozen["commit"].encode(), MASTER.encode()]
        if not already_pushed:
            pushed = subprocess.run(
                ["git", "push", "--porcelain", "--no-follow-tags", "--recurse-submodules=no", "--no-verify", "--",
                 destination, frozen["commit"] + ":" + MASTER], cwd=root, capture_output=True)
            if pushed.returncode:
                closure["push"].update(state="rejected", exit_status=pushed.returncode)
                save(path, closure)
                raise ValueError(f"Push rejected by {remote}; local publication is retained")
        closure["push"].update(state="pushed", exit_status=0)
        save(path, closure)
        return closure["push"]


def status(root, record):
    frozen = published_target(root, record)
    _, closure = closure_record(frozen)
    selection = frozen.get("delivery", {})
    outcomes = frozen.get("cleanup", {})
    preserved = closure.get("preservation")
    receipts = preservation_receipts(root, frozen, preserved) if preserved else {}
    required = preservation_inputs(frozen)

    def cleanup_state(branch):
        return outcomes.get(branch, "pending" if branch in selection.get("cleanup", []) else "unselected")

    return {
        "published_commit": frozen["commit"], "closure_commit": closure.get("graph", {}).get("commit"),
        "plans": [{"task_id": plan["task_id"], "revision": plan["revision"],
                   "original_tip": plan["source"]["tip"], "published_commit": frozen["commit"],
                   "source_execution": plan["completion"].get("execution"),
                   "preservation": receipts.get(plan["task_id"], "pending" if plan["task_id"] in required
                                               else "unselected"),
                   "cleanup": cleanup_state(plan["source"]["ref"])} for plan in selection.get("plans", [])],
        "assembly_cleanup": cleanup_state(frozen["source_ref"]) if selection.get("assembly") else None,
        "assembly_verification": selection.get("assembly_verification"),
        "push": closure.get("push", "pending" if selection.get("push") else "unselected"),
    }


def dependency(value):
    task, separator, revision = value.partition("=")
    if not separator or not revision:
        raise argparse.ArgumentTypeError("Specify TASK=REVISION")
    try:
        return identifier(task), revision
    except ValueError as error:
        raise argparse.ArgumentTypeError(str(error)) from error


def fresh_output(root, path, result):
    output = path.resolve()
    if not output.is_relative_to(root / ".agent/tmp") or output.exists():
        raise ValueError("Write a new delivery artifact under the owning checkout's .agent/tmp")
    save(output, result)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    discover_parser = commands.add_parser("discover", help="Show eligible plans and proposed dependency order")
    discover_parser.add_argument("--output", type=Path)
    snapshot_parser = commands.add_parser("snapshot", help="Capture completed implementation for its result pair")
    snapshot_parser.add_argument("--plan", required=True)
    snapshot_parser.add_argument("--registration", type=Path, required=True)
    snapshot_parser.add_argument("--output", type=Path, required=True)
    declare_parser = commands.add_parser("declare", help="Record completed implementation in its checkout")
    declare_parser.add_argument("--plan", required=True)
    declare_parser.add_argument("--result", required=True)
    declare_parser.add_argument("--blocker-disposition", required=True)
    declare_parser.add_argument("--remaining", action="append", required=True, choices=REMAINING)
    declare_parser.add_argument("--depends-on", action="append", type=dependency, default=[])
    declare_parser.add_argument("--working-commit", action="store_true")
    withdraw_parser = commands.add_parser("withdraw", help="Withdraw readiness from its task checkout")
    withdraw_parser.add_argument("--task", required=True)
    withdraw_parser.add_argument("--reason", required=True)
    select_parser = commands.add_parser("select", help="Record the user's exact delivery selection")
    select_parser.add_argument("--offer", required=True, type=Path)
    select_parser.add_argument("--task", action="append", required=True)
    select_parser.add_argument("--answer", required=True)
    select_parser.add_argument("--assembly-branch")
    select_parser.add_argument("--confirmation", type=Path)
    select_parser.add_argument("--cleanup", action="append", default=[], metavar="FULL_REF")
    select_parser.add_argument("--push-remote")
    select_parser.add_argument("--output", required=True, type=Path)
    ready_parser = commands.add_parser("revalidate", help="Confirm the selected inputs before assembly")
    ready_parser.add_argument("selection", type=Path)
    ready_parser.add_argument("--provisioned", action="store_true")
    ready_parser.add_argument("--sources-output", type=Path)
    for action in ("push", "status", "preservation-inputs"):
        record_parser = commands.add_parser(action)
        record_parser.add_argument("publication", type=Path)
    args = parser.parse_args()
    try:
        root = Path(value(Path.cwd(), "rev-parse", "--show-toplevel")).resolve()
        if args.command == "snapshot":
            result = capture_source(root, args.plan, args.registration)
            fresh_output(root, args.output, result)
        elif args.command == "declare":
            dependencies = dict(args.depends_on)
            if len(dependencies) != len(args.depends_on):
                raise ValueError("A plan dependency must specify one revision")
            result = str(declare(root, args.plan, args.result, args.blocker_disposition, args.remaining,
                                 dependencies, args.working_commit))
        elif args.command == "withdraw":
            result = str(withdraw(root, args.task, args.reason))
        elif args.command == "select":
            result = select(root, read(args.offer), args.task, args.answer, assembly_branch=args.assembly_branch,
                            confirmation=args.confirmation, cleanup=args.cleanup, remote=args.push_remote)
            fresh_output(root, args.output, result)
        elif args.command == "revalidate":
            result = revalidate(root, read(args.selection), provisioned=args.provisioned)
            if args.sources_output:
                output = args.sources_output.resolve()
                assembly = result["assembly"]
                if not args.provisioned or not assembly:
                    raise ValueError("The sources manifest requires the provisioned assembly checkout")
                fresh_output(Path(assembly["root"]), output, result["sources"])
        elif args.command == "push":
            result = push(root, args.publication)
        elif args.command == "status":
            result = status(root, args.publication)
        elif args.command == "preservation-inputs":
            result = preservation_inputs(published_target(root, args.publication))
        else:
            result = offer(root)
            if args.output:
                fresh_output(root, args.output, result)
        print(json.dumps(result, indent=2))
    except INPUT_ERRORS as error:
        print(f"Delivery refused: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
