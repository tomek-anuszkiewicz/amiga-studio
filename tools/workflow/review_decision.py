"""Read the human checkpoint and its one-use continuation receipt."""

from hashlib import sha256
import json
from pathlib import Path

from review_inputs import (
    CANDIDATE, REFERENCE, SLUG, STRING, ReviewInputError, array, choices, fingerprint, normalize_candidate,
    object_fields,
)


CHOICES = ("accept", "continue-one", "pause")
ORDINAL = {"type": "integer", "minimum": 0}
DECISION = object_fields({
    "kind": {"const": "human_review_decision"}, "guidance_id": SLUG, "checkpoint": STRING,
    "candidate": CANDIDATE, "round": ORDINAL, "issue_ids": array(STRING, 1),
    "choice": choices(*CHOICES), "conversation": REFERENCE, "thread_id": STRING,
    "offer_ordinal": ORDINAL, "reply_ordinal": ORDINAL,
})
CONSUMPTION = object_fields({
    "kind": {"const": "review_continuation"}, "checkpoint": STRING, "decision": REFERENCE,
    "from_round": ORDINAL, "next_round": ORDINAL, "reply_identity": STRING,
    "issues": array({"type": "object"}, 1),
})


def checkpoint_identity(task, item, record, issues):
    return fingerprint({
        "task_id": task, "work_item_id": item, "candidate": record["candidate"],
        "round": record["number"], "issues": issues,
    })


def conversation_message(row, role):
    payload = row.get("payload", {})
    if row.get("type") != "response_item" or payload.get("type") != "message" or payload.get("role") != role:
        return None
    return "\n".join(part["text"] for part in payload.get("content", [])
                     if part.get("type") in ("input_text", "output_text"))


def check_reply(artifacts, decision):
    lines = artifacts.read(decision["conversation"]).splitlines()
    if not lines:
        raise ReviewInputError("checkpoint conversation evidence is empty")
    header = json.loads(lines[0])
    if header["type"] != "session_meta" or header["payload"]["id"] != decision["thread_id"]:
        raise ReviewInputError("checkpoint conversation belongs to another thread")
    relevant = (decision["offer_ordinal"], decision["reply_ordinal"])
    rows = {}
    for raw in lines[1:]:
        row = json.loads(raw)
        ordinal = row.get("ordinal")
        if ordinal in relevant:
            if ordinal in rows:
                raise ReviewInputError("checkpoint conversation has conflicting message ordinals")
            rows[ordinal] = (row, sha256(raw).hexdigest())
    if len(rows) != 2 or relevant[0] >= relevant[1]:
        raise ReviewInputError("the human reply must follow the named checkpoint offer")
    offer = conversation_message(rows[relevant[0]][0], "assistant")
    reply = conversation_message(rows[relevant[1]][0], "user")
    prompt = f"review {decision['guidance_id']} {decision['checkpoint']}"
    if offer is None or any(f"{prompt} {choice}" not in offer for choice in CHOICES):
        raise ReviewInputError("conversation evidence lacks the exact three checkpoint choices")
    if reply is None or reply.strip() != f"{prompt} {decision['choice']}":
        raise ReviewInputError("the recorded user reply does not identify one exact current checkpoint choice")
    return rows[relevant[1]][1]


def check_decision(artifacts, ref, record, checkpoint, issue_ids):
    decision = artifacts.read_json(ref, DECISION)
    if (decision["checkpoint"] != checkpoint or decision["round"] != record["number"]
            or normalize_candidate(decision["candidate"]) != record["candidate"]
            or sorted(decision["issue_ids"]) != issue_ids):
        raise ReviewInputError("the human reply is stale or names different findings or evidence gaps")
    return decision, check_reply(artifacts, decision)


def canonical_receipt(request, checkpoint):
    return (Path(".agent/tmp/workflow-TA") / request["task_id"] / request["work_item_id"] /
            "review-decisions" / f"{checkpoint}.json").as_posix()


def continuation(artifacts, request, record, decision_ref, checkpoint, reply_identity, issues):
    expected = {
        "kind": "review_continuation", "checkpoint": checkpoint, "decision": decision_ref,
        "from_round": record["number"], "next_round": record["number"] + 1,
        "reply_identity": reply_identity, "issues": issues,
    }
    path = canonical_receipt(request, checkpoint)
    location = artifacts.path(path)
    existing = location.exists()
    if existing:
        ref = {"path": path, "sha256": sha256(location.read_bytes()).hexdigest()}
        if artifacts.read_json(ref, CONSUMPTION) != expected:
            raise ReviewInputError("the continuation receipt disagrees with the consumed choice")
    return {"path": path, "record": expected, "already_recorded": existing}


def saved_continuation(artifacts, path):
    location = artifacts.path(path)
    if location.exists():
        ref = {"path": path, "sha256": sha256(location.read_bytes()).hexdigest()}
        return artifacts.read_json(ref, CONSUMPTION)
    return None


def check_prior_authorization(artifacts, request, previous, record):
    if previous["number"] < 3:
        if record.get("started_by") is not None or previous["candidate"] == record["candidate"]:
            raise ReviewInputError("a retry on unchanged candidate inputs must remain in the same round")
        return
    receipt_ref = record.get("started_by")
    if receipt_ref is None:
        raise ReviewInputError("a round after the checkpoint needs its consumed one-round human authorization")
    receipt = artifacts.read_json(receipt_ref, CONSUMPTION)
    if (receipt_ref["path"] != canonical_receipt(request, receipt["checkpoint"])
            or receipt["from_round"] != previous["number"] or receipt["next_round"] != record["number"]):
        raise ReviewInputError("a consumed continuation covers only its specified next round")
    checkpoint = checkpoint_identity(request["task_id"], request["work_item_id"], previous, receipt["issues"])
    if checkpoint != receipt["checkpoint"]:
        raise ReviewInputError("the consumed continuation does not describe the preceding checkpoint")
    decision, reply = check_decision(artifacts, receipt["decision"], previous, receipt["checkpoint"],
                                    sorted(item["id"] for item in receipt["issues"]))
    if decision["choice"] != "continue-one" or reply != receipt["reply_identity"]:
        raise ReviewInputError("the next round lacks the consumed human continuation")
