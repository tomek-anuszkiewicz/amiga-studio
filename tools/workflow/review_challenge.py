"""Bind a review challenge to collected reports and the original lane verdicts."""

from hashlib import sha256
import json
from pathlib import Path

from result import references
from review_inputs import CONTEXT, ReviewInputError, fingerprint, normalize_candidate, qualified


def collection(router, rejected):
    findings = []
    for key, row in sorted(router.assessments.items()):
        if row["kind"] != "required" or key in router.settled or key in rejected:
            continue
        lane = router.head["lane_states"][key[0]]
        report = lane.get("report")
        if not report or not any(item["id"] == key[1] for item in router.artifacts.report(report)["findings"]):
            continue
        findings.append({"finding": row["finding"], "report": report, "input": lane["input"],
                         "basis": row["basis"], "evidence": row["evidence"]})
    return {"task_id": router.task, "work_item_id": router.item, "round": router.number,
            "candidate": router.head["candidate"], "lanes": router.head["lane_states"],
            "findings": findings}


def identity(manifest):
    return {name: manifest[name] for name in ("task_id", "work_item_id", "round", "candidate")}


def receipt_path(manifest):
    slot = {**identity(manifest), "previous": manifest.get("previous", {}).get("collection")}
    return (Path(".agent/tmp/workflow-TA") / manifest["task_id"] / manifest["work_item_id"]
            / "review-routing" / "challenges" / f"round-{fingerprint(slot)}" / "collection.json")


def claim(root, manifest):
    """Reserve one assessment before dispatch, also when current lane states change."""
    path = root / receipt_path(manifest)
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("x", encoding="utf-8", newline="\n") as output:
            json.dump({**manifest, "integrity": fingerprint(manifest)}, output, indent=2)
            output.write("\n")
    except FileExistsError:
        return False
    return True


def receipt(artifacts, manifest, *, historical=None):
    path = receipt_path(manifest)
    location = artifacts.path(path.as_posix())
    if not location.exists() and "previous" not in manifest:
        # Preserve reservations written by the original collection format.
        legacy = []
        for candidate in sorted(location.parent.parent.glob("*/collection.json")):
            if candidate.parent.name.startswith("round-"):
                continue
            saved = json.loads(candidate.read_bytes())
            if identity(saved) == identity(manifest):
                legacy.append(candidate.relative_to(artifacts.root))
        if len(legacy) > 1:
            raise ReviewInputError("multiple original reservations exist for this review round")
        if legacy:
            path = legacy[0]
            location = artifacts.path(path.as_posix())
    if not location.exists():
        return path.as_posix(), None, None
    content = location.read_bytes()
    saved = json.loads(content)
    digest = saved.pop("integrity", None) if path.parent.name.startswith("round-") else path.parent.name
    if (digest != fingerprint(saved) or identity(saved) != identity(manifest)
            or saved.get("previous") != manifest.get("previous")):
        raise ReviewInputError("saved challenge collection identity is damaged or stale")
    ref = {"path": path.as_posix(), "sha256": sha256(content).hexdigest()}
    if historical is None:
        artifacts.check_all(saved)
    else:
        artifacts.check_snapshot_evidence(references(saved), historical)
    return path.as_posix(), ref, saved


def finding_rows(manifest):
    return {qualified(row["finding"]): {
        "input": manifest["lanes"][row["finding"]["lane"]]["input"],
        "decisions": manifest.get("decisions", []), **row,
    } for row in manifest["findings"]}


def collect_decisions(router, rows, completed, rejected):
    refs = router.request.get("risk_decisions", [])
    router.artifacts.check_all(refs)
    for key, row in rows.items():
        row["decisions"] = list(completed.get(key, {}).get("decisions", []))
    selected = {qualified(row) for row in router.request["selected_findings"]}
    for entry in refs:
        if "finding" in entry:
            key, ref = qualified(entry["finding"]), entry["evidence"]
            if key in rejected:
                continue
            scope = {key}
        else:
            ref = entry
            if any(ref in row["decisions"] for row in (completed | rejected).values()):
                continue
            scope = selected if completed else rows.keys()
        if not scope or not scope <= rows.keys():
            raise ReviewInputError("new challenge decisions must identify the affected current findings")
        for key in scope:
            if ref not in rows[key]["decisions"]:
                rows[key]["decisions"].append(ref)


def read_report(router, report, collection_ref, required, contexts, record):
    payload = report["result"]
    if (report["skill"] != "TA-challenge-findings" or report["status"] != "completed"
            or (report["task_id"], report["work_item_id"]) != (router.task, router.item)
            or normalize_candidate(report["inputs"]["candidate"]) != record["candidate"]
            or payload["collection"] != collection_ref):
        raise ReviewInputError("challenge result belongs to a different or incomplete collection")
    router.check_review_evidence(references(payload), record)
    context = router.artifacts.read_json(payload["context"], CONTEXT)
    if context["id"] in contexts or context["run_id"] != report["run_id"]:
        raise ReviewInputError("challenge context must be fresh and independent of prior reviewers")
    contexts.add(context["id"])
    decisions = payload["decisions"]
    if required != {qualified(row["finding"]) for row in decisions} or len(required) != len(decisions):
        raise ReviewInputError("challenge must account for every selected lane and finding ID exactly once")
    for row in decisions:
        duplicate = row["duplicate_of"]
        if duplicate and qualified(duplicate) not in required:
            raise ReviewInputError("unknown duplicate finding in challenge report")
    return decisions


def separate_returns(router):
    """Retain closed claims across ordinary review rounds with their original evidence."""
    previous = {}
    contexts = set(router.contexts)
    dispositions = {}
    current = []
    refs = list(router.request.get("prior_challenge_reports", []))
    current_ref = router.request.get("challenge_report")
    if current_ref is not None:
        refs.append(current_ref)
    for ref in refs:
        report = router.artifacts.report(ref)
        if report["result"]["kind"] != "finding_challenge":
            raise ReviewInputError("a challenge return requires an independent challenge result")
        collection_ref = report["result"]["collection"]
        manifest = router.artifacts.read_json(collection_ref)
        number = manifest["round"]
        if not 1 <= number <= router.number:
            raise ReviewInputError("challenge return is outside the recorded review rounds")
        if number == router.number:
            current.append((ref, report))
            continue
        if ref == current_ref:
            raise ReviewInputError("the current challenge return names an earlier review round")
        record = router.rounds[number - 1][1]
        expected = {"task_id": router.task, "work_item_id": router.item, "round": number,
                    "candidate": record["candidate"], **previous.get(number, {})}
        _, found_ref, saved = receipt(router.artifacts, expected, historical=record)
        if not saved or found_ref != collection_ref:
            raise ReviewInputError("historical challenge return lacks its exact reserved collection")
        if set(saved["lanes"]) != set(router.required) or any(
            lane["state"] != "returned" for lane in saved["lanes"].values()
        ):
            raise ReviewInputError("historical challenge collection lacks required primary reviews")
        rows = finding_rows(saved)
        original_findings = {}
        for key, row in rows.items():
            lane = record["lane_states"][key[0]]
            original = router.artifacts.report(row["report"])
            lane_reports = [lane.get("report"), *record.get("earlier_reports", [])]
            reserved = saved["lanes"][key[0]]
            if (row["report"] not in lane_reports or row["report"] != reserved.get("report")
                    or row["input"] != reserved["input"]
                    or row["input"] != original["result"].get("routing", {}).get("input")):
                raise ReviewInputError("historical challenge finding lacks its original primary review")
            finding = next((item for item in original["findings"] if item["id"] == key[1]), None)
            if key not in router.latest or finding is None:
                raise ReviewInputError("historical challenge finding is outside the recorded review rounds")
            original_findings[key] = finding
        for row in read_report(router, report, collection_ref, rows.keys(), contexts, record):
            key = qualified(row["finding"])
            assessment = router.assessments[key]
            same_evidence = all(assessment[field] == rows[key][field] for field in ("basis", "evidence"))
            if (same_evidence and router.latest[key] == (original_findings[key], number)
                    and number >= dispositions.get(key, (0, ""))[0]):
                dispositions[key] = (number, row["disposition"], {
                    "finding": row["finding"], "report": ref, "decisions": rows[key]["decisions"],
                })
        previous[number] = {"previous": {"collection": collection_ref, "report": ref}}
    rejected = {key: evidence for key, (_, decision, evidence) in dispositions.items() if decision == "reject"}
    if rejected:
        decisions = router.request.get("risk_decisions", [])
        router.artifacts.check_all(decisions)
        selected = {qualified(row) for row in router.request["selected_findings"]}
        for entry in decisions:
            if "finding" in entry:
                key = qualified(entry["finding"])
                if key in rejected and entry["evidence"] not in rejected[key]["decisions"]:
                    del rejected[key]
            elif not any(entry in row["decisions"] for _, _, row in dispositions.values()):
                rejected = {key: row for key, row in rejected.items() if selected and key not in selected}
    return rejected, current, contexts


def assess(router, manifest, returns, contexts, rejected):
    if not manifest["findings"]:
        if returns:
            raise ReviewInputError("challenge return has no current required findings")
        return {"state": "skipped", "reason": "No current required repair was collected."}
    missing = sorted(lane for lane, state in router.verdict_states.items() if state != "returned")
    if missing:
        return {"state": "pending_lanes", "lanes": missing,
                "findings": [row["finding"] for row in manifest["findings"]]}

    completed = {}
    judgments = {}
    prior = {}
    for result_ref, report in returns:
        path, ref, saved = receipt(router.artifacts, {**manifest, **prior})
        if not saved:
            raise ReviewInputError("challenge return lacks its reserved collection")
        frozen = finding_rows(saved)
        decisions = read_report(router, report, ref, frozen.keys(), contexts, router.head)
        if any(row == completed.get(key) for key, row in frozen.items()):
            raise ReviewInputError("challenge reassessment repeats an unchanged finding")
        completed.update(frozen)
        judgments.update({qualified(row["finding"]): {**row, "report": result_ref} for row in decisions})
        prior = {"previous": {"collection": ref, "report": result_ref}}

    rows = finding_rows(manifest)
    collect_decisions(router, rows, completed, rejected)
    changed = {key for key, row in rows.items() if row != completed.get(key)}
    retained = [row for key, row in judgments.items() if key in rows and key not in changed]
    summary = {"eligible_repairs": [row for row in retained if row["disposition"] == "uphold"],
               "rejected": [row for row in retained if row["disposition"] == "reject"]}

    if not changed:
        return {**summary, "manifest": saved, "path": path, "state": "returned", "report": result_ref}

    current = {**manifest, **prior, "findings": [rows[key] for key in sorted(changed)]}
    path, _, saved = receipt(router.artifacts, current)
    if saved is not None:
        # Collect the original return before reopening findings already being assessed.
        if finding_rows(saved) != finding_rows(current):
            summary["pending_findings"] = [rows[key]["finding"] for key in sorted(changed)]
        return {**summary, "manifest": saved, "path": path, "state": "awaiting_return"}

    return {**summary, "manifest": current, "path": path, "state": "ready_to_claim"}
