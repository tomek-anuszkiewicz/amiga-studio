"""Recommend independent review lanes or the human checkpoint from frozen evidence."""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import re
import sys

from review_challenge import assess, claim, collection, separate_returns
from review_decision import (
    CHOICES, DECISION, canonical_receipt, check_decision, check_prior_authorization,
    checkpoint_identity, continuation, saved_continuation,
)
from review_inputs import (
    Artifacts, CONTEXT, LANE_INPUT, LANE_SKILLS, MissingArtifact, REQUEST, ROUND, ROUTING, ReviewInputError,
    UnavailableArtifact, check_shape, fingerprint, normalize_candidate, qualification, qualified,
)


TABLE = json.loads(Path(__file__).with_name("review-routing.json").read_bytes())
BATCH = tuple(TABLE["columns"])
ROWS = {key: (reason, dict(zip(BATCH, cells, strict=True))) for key, reason, *cells in TABLE["rows"]}


def render_table():
    names = [lane.replace("_", "/").capitalize() for lane in BATCH]
    return "\n".join([
        "| Review stage or effect | " + " | ".join(names) + " |",
        "| " + " | ".join(["---"] * (len(BATCH) + 1)) + " |",
        *("| " + " | ".join([reason, *cells.values()]) + " |" for reason, cells in ROWS.values()),
    ])


def required_lanes(operation):
    if operation == "batch":
        return BATCH
    return (next(lane for lane, skill in LANE_SKILLS.items() if skill == operation),)


def finding_history(comparison):
    if not comparison or comparison["current"]["outcome"] != "fail":
        return "uncertain"
    parent = comparison["parent"]["outcome"]
    change = comparison["change"]
    if change != "none":
        return change
    if parent == "pass":
        return "repair_regression"
    if parent == "fail":
        return {"covered": "previously_missed", "documented_gap": "previously_unchecked",
                "unknown": "uncertain"}[comparison["parent_scope"]]
    return "uncertain"


class Router:
    def __init__(self, request, root):
        self.request = check_shape(request, REQUEST, "review request")
        self.artifacts = Artifacts(root)
        self.task = request["task_id"]
        self.item = request["work_item_id"]
        self.required = required_lanes(request["operation"])
        self.rounds = self.artifacts.history(request)
        self.head = self.rounds[-1][1]
        self.number = self.head["number"]
        self.reports = {}
        self.current_reports = {}
        self.approvals = {}
        self.contexts = {}
        self.gaps = []
        self.first = {}
        self.latest = {}
        self.verdict_states = {}
        self.comparisons_by_round = {}
        self.issue_reviews = {}
        self.reopened = set()
        self.settled = set()
        self.verified = set()
        self.issues = {}
        self.pending = set()
        self.assessments = {}
        self.effects = []
        self.gates = {}
        for (_, previous), (_, record) in zip(self.rounds, self.rounds[1:]):
            check_prior_authorization(self.artifacts, request, previous, record)

    def check_source(self, ref, candidate, kind):
        if ref is None or candidate is None:
            raise ReviewInputError("executable review guidance requires an identified source result")
        candidate = normalize_candidate(candidate)
        for _, record in self.rounds:
            if record["candidate"] != candidate:
                continue
            if kind == "finding_challenge" and record is self.head and (
                    ref == self.request.get("challenge_report") or ref in self.request.get("prior_challenge_reports", [])):
                return
            if kind == "review":
                links = record.get("earlier_reports", []) + [row.get("report") for row in record["lane_states"].values()]
            else:
                links = record.get("source_results", []) + [gate.get("evidence") for gate in record["gates"]]
            if ref in links:
                return
        raise ReviewInputError("guidance source result is outside the current review history")

    def issue(self, key, text, *, lane=None, mandatory=True, history="uncertain", finding=None):
        value = {"id": key, "impact": text, "mandatory": mandatory, "history": history}
        if lane is not None:
            value["owner"] = lane
        if finding is not None:
            value["finding"] = qualification(self.task, self.item, finding)
            value["origin"] = self.first[finding][1]["origin"]
        self.issues[key] = value

    def report(self, ref, record, lane=None, input_ref=None):
        result = self.artifacts.report(ref)
        candidate = result["inputs"]["candidate"]
        if any(result[name] != self.request[name] for name in ("task_id", "work_item_id")):
            raise ReviewInputError("a report belongs to another task or work item")
        if result["result"]["kind"] != "review" or not candidate:
            raise ReviewInputError("a finding or approval needs an identified independent review")
        if normalize_candidate(candidate) != record["candidate"]:
            raise ReviewInputError(f"report candidate is stale: {ref['path']}")
        role = result["result"]["lane"]
        if role not in self.required or (lane is not None and role != lane):
            raise ReviewInputError(f"report lane is outside the selected review: {ref['path']}")
        remembered = self.reports.get(ref["path"])
        if remembered and remembered != ref:
            raise ReviewInputError("a report path has conflicting hashes")
        if not remembered:
            self.reports[ref["path"]] = ref
            for item in result["findings"]:
                key = (role, item["id"])
                self.first.setdefault(key, (ref, item, record["number"]))
                self.latest[key] = (item, record["number"])
        if input_ref is not None:
            # An unfinished reviewer can establish findings but cannot settle one.
            if result["status"] == "completed" and result["result"]["verdict"] != "not_assessed":
                self.approval_inputs(role, ref, input_ref, result, record)
        return result

    def approval_inputs(self, lane, ref, input_ref, result, record):
        payload = result["result"]
        routing = payload.get("routing")
        if not routing:
            if record is self.head:
                self.issue(f"lane:{lane}:missing", "The review's scoped inputs and fresh context are unidentified.",
                           lane=lane)
            return
        check_shape(routing, ROUTING, "review routing evidence")
        if routing["input"] != input_ref:
            raise ReviewInputError(f"reviewed requirements, criteria, partition or evidence changed: {lane}")
        profile = self.artifacts.read_json(input_ref, LANE_INPUT)
        if profile["lane"] != lane or set(profile["scope"]) - set(payload["inspected_scope"]):
            if record is self.head:
                self.issue(f"lane:{lane}:missing", "The review does not cover its declared scope.", lane=lane)
            return
        # The saved profile declares the original inputs. Only current review inputs are live.
        if record is self.head:
            self.artifacts.check_all(profile)
        context = self.artifacts.read_json(routing["context"], CONTEXT)
        context_id = context["id"]
        attribution = (ref, routing["context"])
        earlier = self.contexts.get(context_id)
        if context["run_id"] != result["run_id"] or (earlier is not None and earlier != attribution):
            raise ReviewInputError("independent review contexts are missing or reused")
        if earlier is None:
            self.contexts[context_id] = attribution
        basis = fingerprint(record["candidate"])
        links = [entry["artifact_ref"] for entry in result["evidence"]
                 if entry["candidate_ref"] == basis and entry["artifact_ref"] is not None]
        if not links:
            if record is self.head:
                self.issue(f"lane:{lane}:missing", "Candidate-bound independent evidence is unavailable.", lane=lane)
            return
        if record is self.head:
            self.artifacts.check_all(links)
        if earlier is None:
            # Comparisons describe the original reviewed round and its predecessor.
            self.comparisons_by_round[record["number"], lane] = routing["comparisons"]
        if record is self.head and payload["verdict"] == "approved":
            self.approvals[lane] = result["run_id"]

    def check_review_evidence(self, refs, record):
        if record is self.head:
            self.artifacts.check_all(refs)
        else:
            self.artifacts.check_snapshot_evidence(refs, record)

    def collect(self):
        for _, record in self.rounds:
            states = record["lane_states"]
            if set(states) != set(self.required):
                raise ReviewInputError("required lane state is incomplete or adds an unassigned reviewer")
            for ref in sorted(record.get("earlier_reports", []), key=lambda row: row["path"]):
                self.report(ref, record)
            for lane, row in sorted(states.items()):
                status = row["state"]
                if record is self.head:
                    profile = self.artifacts.read_json(row["input"], LANE_INPUT)
                    if profile["lane"] != lane or (self.request["operation"] == "batch" and not profile["partition"]):
                        raise ReviewInputError(f"required scoped review input is missing: {lane}")
                    self.artifacts.check_all(profile)
                    self.verdict_states[lane] = status
                if status == "returned" and not row.get("report"):
                    raise ReviewInputError(f"returned {lane} lane lacks its paired report")
                if row.get("report"):
                    if status not in ("returned", "failed"):
                        raise ReviewInputError(f"{lane} report conflicts with its phase status")
                    try:
                        report = self.report(row["report"], record, lane,
                                             row["input"] if status == "returned" else None)
                        if record is self.head and status == "returned" and (
                                report["status"] != "completed" or report["result"]["verdict"] == "not_assessed"
                                or f"lane:{lane}:missing" in self.issues):
                            self.verdict_states[lane] = "missing"
                        elif record is self.head and status == "returned":
                            self.current_reports[lane] = report
                    except MissingArtifact as error:
                        if record is not self.head:
                            raise
                        self.verdict_states[lane] = "missing"
                        self.gaps.append(str(error))
                        issue_id = f"lane:{lane}:missing"
                        self.issue(issue_id, row.get("reason", "Required independent review evidence is unavailable."),
                                   lane=lane)
                        self.issues[issue_id]["artifact"] = error.ref
            if record is self.head:
                for gate in sorted(record["gates"], key=lambda item: item["id"]):
                    name, state = gate["id"], gate["state"]
                    if name in self.gates:
                        raise ReviewInputError(f"required gate is listed twice: {name}")
                    artifact = gate.get("evidence")
                    if state in ("passed", "failed"):
                        try:
                            self.check_gate(gate)
                        except UnavailableArtifact as error:
                            state, artifact = "missing", error.ref
                            self.gaps.append(str(error))
                    self.gates[name] = state
                    if state in ("failed", "missing"):
                        self.issue(f"gate:{name}:{state}", gate["impact"], lane=name)
                        if artifact is not None:
                            self.issues[f"gate:{name}:{state}"]["artifact"] = artifact
                if self.request["operation"] == "batch" and "commit" not in self.gates:
                    raise ReviewInputError("batch review needs the canonical commit gate state")

    def check_gate(self, gate):
        name, state = gate["id"], gate["state"]
        if not gate.get("evidence"):
            raise UnavailableArtifact(None, f"required gate has no observed report: {name}")
        worker = self.artifacts.report(gate["evidence"])
        payload = worker["result"]
        profile = name if name in ("commit", "milestone") else "focused"
        candidate = worker["inputs"]["candidate"]
        if (any(worker[field] != self.request[field] for field in ("task_id", "work_item_id"))
                or not candidate or normalize_candidate(candidate) != self.head["candidate"]
                or payload["kind"] != "verification" or payload["profile"] != profile
                or payload["verdict"] != state):
            raise ReviewInputError(f"gate identity or actual result disagrees: {name}")
        current = fingerprint(self.head["candidate"])
        receipts = [entry["artifact_ref"] for entry in worker["evidence"]
                    if entry["candidate_ref"] == current and entry["artifact_ref"] is not None
                    and entry["origin"] in ("executed", "retained") and entry["command"]
                    and (profile == "focused" or re.search(
                        rf'(?:^|[ /"])(?:tools/verification/)?verify\.py"?\s+{profile}(?:\s|$)',
                        entry["command"].replace("\\", "/")))
                    and entry["exit_code"] is not None and (entry["exit_code"] == 0) == (state == "passed")]
        if not receipts:
            raise UnavailableArtifact(gate["evidence"],
                                      f"gate lacks an actual exit-status receipt for this candidate: {name}")
        self.artifacts.check_all(receipts + payload["report_refs"])

    def read_assessments(self):
        for row in sorted(self.request["assessments"], key=lambda item: qualified(item["finding"])):
            key = qualified(row["finding"])
            if key in self.assessments or key not in self.first:
                raise ReviewInputError(f"ambiguous or unknown qualified finding: {key}")
            ref, item, number = self.first[key]
            if row["source"] != ref or row["owner"] not in self.required:
                raise ReviewInputError(f"stale source or unassigned criterion owner: {key}")
            if item["severity"] in ("blocking", "major") and row["kind"] != "required":
                raise ReviewInputError(f"required finding cannot be downgraded: {key}")
            self.artifacts.check_all(row["evidence"])
            self.check_review_evidence(item["evidence_refs"], self.rounds[number - 1][1])
            if row["kind"] != "suggestion" and not item["evidence_refs"]:
                raise ReviewInputError(f"demonstrated finding lacks evidence: {key}")
            self.assessments[key] = row
        for key, (_, item, _) in self.first.items():
            if item["severity"] != "suggestion" and key not in self.assessments:
                raise ReviewInputError(f"missing disposition/owner assessment for qualified finding: {key}")
        for key, row in self.assessments.items():
            if "duplicate_of" in row:
                duplicate = qualified(row["duplicate_of"])
                canonical = self.assessments.get(duplicate)
                if (key == duplicate or not canonical or "duplicate_of" in canonical
                        or any(canonical[name] != row[name] for name in ("owner", "kind"))):
                    raise ReviewInputError(f"conflicting or ambiguous duplicate finding link: {key}")

    def assessed_finding(self, key):
        if key not in self.assessments:
            raise ReviewInputError(f"unknown qualified finding: {key}")
        return key

    def comparisons(self):
        seen = set()
        for (number, lane), rows in sorted(self.comparisons_by_round.items()):
            for row in rows:
                key = self.assessed_finding(qualified(row["finding"]))
                source = self.first[key][1]
                if (self.assessments[key]["owner"] != lane or (number, key) in seen
                        or self.first[key][2] > number):
                    raise ReviewInputError(f"conflicting review or incorrect criterion owner: {key}")
                seen.add((number, key))
                if row["trigger"] != source["scenario"]:
                    raise ReviewInputError(f"comparison does not check the original trigger: {key}")
                for phase in ("parent", "current"):
                    observation = row[phase]
                    index = number - (2 if phase == "parent" else 1)
                    candidate_ref = fingerprint(self.rounds[index][1]["candidate"]) if index >= 0 else None
                    if observation["candidate_ref"] != candidate_ref:
                        raise ReviewInputError(f"{phase} comparison uses different candidate inputs: {key}")
                    if observation["outcome"] != "unavailable" and not observation["evidence"]:
                        raise ReviewInputError(f"{phase} outcome lacks evidence: {key}")
                    # A saved comparison contributes its outcome to the ledger.
                    # A current comparison must support its claim with exact evidence.
                    if number == self.number:
                        if index >= 0:
                            self.check_review_evidence(observation["evidence"], self.rounds[index][1])
                        else:
                            self.artifacts.check_all(observation["evidence"])
                disposition = row.get("disposition")
                if disposition:
                    if row["current"]["outcome"] != "pass" or not row["current"]["evidence"]:
                        raise ReviewInputError(f"disposition lacks independent current evidence: {key}")
                    self.settled.add(key)
                    if number == self.number:
                        self.verified.add(key)
                        self.reopened.discard(key)
                elif row["current"]["outcome"] == "fail" and key in self.settled:
                    self.settled.remove(key)
                    if number == self.number:
                        self.reopened.add(key)
                if number == self.number:
                    self.issue_reviews[key] = row

    def focused(self, selected):
        if self.number == 1 or self.rounds[-2][1]["candidate"]["base_commit"] != self.head["candidate"]["base_commit"]:
            return {lane: [ROWS["first"][0]] for lane in self.required}
        repair = self.request.get("repair")
        if not repair or repair["before"] != self.rounds[-2][0] or repair["after"] != self.rounds[-1][0]:
            raise ReviewInputError("repair needs the exact preceding/current round and its assessed delta")
        paths = self.artifacts.changed_paths(self.rounds[-2][1], self.head)
        if sorted(repair["paths"]) != paths:
            raise ReviewInputError("the assessed repair paths differ from the actual frozen delta")
        covered = defaultdict(set)
        for effect in repair["effects"]:
            self.artifacts.check_all(effect["evidence"])
            facet = effect["facet"]
            for lane in self.required:
                if lane not in BATCH or ROWS[facet][1].get(lane) == "First":
                    covered[lane].add(effect["reason"])
        for key in selected:
            row = self.assessments[key]
            if row["kind"] in ("required", "deferrable"):
                covered[row["owner"]].add(f"Assigned finding {key[0]}/{key[1]}")
        return {lane: sorted(reasons) for lane, reasons in covered.items()}

    def resolved_lanes(self, rejected):
        resolved = {}
        for lane, report in self.current_reports.items():
            if report["result"]["verdict"] != "changes_requested":
                continue
            claims = [(lane, row["id"]) for row in report["findings"]
                      if self.assessments.get((lane, row["id"]), {}).get("kind") == "required"
                      and row["disposition"] not in ("verified", "refuted")]
            if claims and all(key in rejected for key in claims):
                resolved[lane] = {
                    "report": self.head["lane_states"][lane]["report"],
                    "findings": [{field: rejected[key][field] for field in ("finding", "report")}
                                 for key in sorted(claims)],
                }
        return resolved

    def review_issues(self, selected, rejected, resolved):
        for key, row in sorted(self.assessments.items()):
            if key in rejected or row["kind"] == "suggestion":
                continue
            lane = row["owner"]
            _, _, first_number = self.first[key]
            if key in self.settled:
                latest, number = self.latest[key]
                if (number == self.number and first_number < number and key not in self.verified
                        and latest["disposition"] == "open"):
                    self.issue(f"lane:{lane}:recurrence", "A recurrence was reported without the owner's comparison.",
                               lane=lane)
                continue
            disposition = self.issue_reviews.get(key)
            if key in selected and first_number < self.number and not disposition:
                if self.verdict_states[lane] in ("due", "running"):
                    self.pending.add(key)
                    continue
            if first_number == self.number:
                history = finding_history(disposition) if self.number > 1 else "first_reported"
            elif key in self.reopened:
                history = "reopened"
            else:
                history = "persisted"
            id = f"finding:{fingerprint(qualification(self.task, self.item, key))}"
            self.issue(id, row["impact"], lane=lane, mandatory=row["kind"] == "required",
                       history=history, finding=key)
        for lane, status in self.verdict_states.items():
            if status in ("failed", "missing"):
                issue_id = f"lane:{lane}:{status}"
                if issue_id not in self.issues:
                    self.issue(issue_id, self.head["lane_states"][lane].get("reason",
                               "A required independent reviewer verdict is unavailable."), lane=lane)
            elif status == "returned" and lane not in self.approvals and lane not in resolved and not any(
                item.get("owner") == lane and item.get("finding") for item in self.issues.values()
            ):
                self.issue(f"lane:{lane}:unassessed", "The independent reviewer has not approved this candidate.",
                           lane=lane)

    def reply(self, problems):
        if not problems or self.number < 3:
            return None, set()
        checkpoint = checkpoint_identity(self.task, self.item, self.head, problems)
        path = canonical_receipt(self.request, checkpoint)
        result = {"id": checkpoint, "round": self.number, "issues": problems, "choices": list(CHOICES)}
        accepted = set()
        saved = saved_continuation(self.artifacts, path)
        ref = self.request.get("decision") or (saved["decision"] if saved else None)
        if ref:
            decision, reply = check_decision(self.artifacts, ref, self.head, checkpoint,
                                             [item["id"] for item in problems])
            if decision["choice"] == "continue-one":
                result["continuation"] = continuation(self.artifacts, self.request, self.head, ref,
                                                      checkpoint, reply, problems)
            elif decision["choice"] == "accept":
                accepted = {item["id"] for item in problems if not item["mandatory"]}
                result["accepted"] = sorted(accepted)
                result["mandatory_remaining"] = [item["id"] for item in problems if item["mandatory"]]
            else:
                result["paused"] = True
        return result, accepted

    def recommend(self):
        self.collect()
        self.read_assessments()
        selected = {self.assessed_finding(qualified(key)) for key in self.request["selected_findings"]}
        self.comparisons()
        base_issues = self.issues.copy()
        self.review_issues(selected, set(), {})
        historical_rejections, challenge_returns, challenge_contexts = separate_returns(self)
        resolved = self.resolved_lanes(historical_rejections)
        if historical_rejections:
            self.issues, self.pending = base_issues.copy(), set()
            self.review_issues(selected, historical_rejections, resolved)
        problems = [item for _, item in sorted(self.issues.items())]
        checkpoint, accepted = self.reply(problems)
        outstanding = [item for item in problems if item["id"] not in accepted]
        output = {
            "task_id": self.task, "work_item_id": self.item, "round": self.number,
            "candidate": self.head["candidate"], "candidate_ref": fingerprint(self.head["candidate"]),
            "verified": [qualification(self.task, self.item, key) for key in sorted(self.verified)],
            "pending": [qualification(self.task, self.item, key) for key in sorted(self.pending)],
            "approvals": dict(sorted(self.approvals.items())),
            "resolved_lanes": resolved,
            "issues": outstanding, "recommended_lanes": [], "executable_options": [],
            "gaps": self.gaps, "checkpoint": checkpoint,
            "duplicate_observations": [
                {"observation": qualification(self.task, self.item, key),
                 "finding": qualification(self.task, self.item, qualified(row["duplicate_of"]))}
                for key, row in sorted(self.assessments.items()) if "duplicate_of" in row
            ],
        }
        manifest = collection(self, historical_rejections)
        output["outstanding_lanes"] = {lane: state for lane, state in sorted(self.verdict_states.items())
                                       if state != "returned"}
        stopped = checkpoint and outstanding
        if stopped:
            output["challenge"] = {"state": "checkpoint_pending" if manifest["findings"] else "skipped",
                                   "findings": [row["finding"] for row in manifest["findings"]]}
            output["phase"] = "human_decision_required"
            if checkpoint.get("paused"):
                output["phase"] = "paused"
            elif checkpoint.get("continuation", {}).get("already_recorded"):
                output["phase"] = "continuation_already_selected"
            elif checkpoint.get("continuation"):
                output["phase"] = "one_round_authorized"
            if output["phase"] != "continuation_already_selected":
                return output
        challenge = assess(self, manifest, challenge_returns, challenge_contexts, historical_rejections)
        output["challenge"] = challenge
        rejected = {qualified(row["finding"]): {"finding": row["finding"], "report": row["report"]}
                    for row in challenge.get("rejected", [])}
        if rejected:
            rejected = historical_rejections | rejected
            resolved = self.resolved_lanes(rejected)
            self.issues, self.pending = base_issues.copy(), set()
            self.review_issues(selected, rejected, resolved)
            outstanding = [item for key, item in sorted(self.issues.items()) if key not in accepted]
            output["issues"] = outstanding
            output["pending"] = [qualification(self.task, self.item, key) for key in sorted(self.pending)]
            output["resolved_lanes"] = resolved
        if challenge["state"] == "pending_lanes":
            output["recommended_lanes"] = [
                {"lane": lane, "skill": LANE_SKILLS[lane], "action": "collect_primary",
                 "reasons": ["Collect the outstanding primary review."]}
                for lane in challenge["lanes"] if self.verdict_states[lane] == "due"
            ]
        if stopped and outstanding:
            return output
        required_issues = any(item["mandatory"] for item in outstanding)
        if required_issues:
            output["phase"] = "changes_requested"
            return output
        focused = self.focused(selected)
        remaining = dict.fromkeys(self.required, [ROWS["remaining"][0]])
        accepted_lanes = self.approvals.keys() | resolved.keys()
        active = focused if set(focused) - accepted_lanes or self.pending else remaining
        phase = ("first" if self.number == 1 else "focused") if active is focused else "remaining"
        active_due = [lane for lane in self.required if lane in active
                      and lane not in accepted_lanes and self.verdict_states[lane] == "due"]
        if active_due:
            output["recommended_lanes"] = [{"lane": lane, "skill": LANE_SKILLS[lane], "reasons": active[lane]}
                                          for lane in active_due]
            output["phase"] = phase
        elif set(self.required) == accepted_lanes and all(state == "passed" for state in self.gates.values()):
            output["phase"] = "done"
        else:
            output["phase"] = "awaiting_evidence"
        return output


def recommend(request, root, *, source=None):
    router = None
    try:
        router = Router(request, root)
        if source is not None:
            router.check_source(*source)
        return router.recommend()
    except (KeyError, TypeError, ValueError, OSError) as error:
        report = {"phase": "needs_input", "gaps": [str(error)], "recommended_lanes": [], "executable_options": []}
        if router:
            problems = [item for _, item in sorted(router.issues.items())]
            report["gaps"] = [*router.gaps, str(error)]
            report.update(task_id=router.task, work_item_id=router.item, round=router.number,
                          candidate=router.head["candidate"], issues=problems,
                          outstanding_lanes={lane: state for lane, state in sorted(router.verdict_states.items())
                                             if state != "returned"})
            if problems and router.number >= 3:
                checkpoint = checkpoint_identity(router.task, router.item, router.head, problems)
                findings = [row["finding"] for row in problems if row["mandatory"] and "finding" in row]
                report.update(phase="human_decision_required", checkpoint={
                    "id": checkpoint, "round": router.number, "issues": problems, "choices": list(CHOICES),
                }, challenge={"state": "checkpoint_pending" if findings else "skipped", "findings": findings})
        return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("request", type=Path, nargs="?")
    parser.add_argument("--claim-challenge", action="store_true", help="Reserve the current challenge before dispatch.")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    flags = parser.add_mutually_exclusive_group()
    flags.add_argument("--table", action="store_true")
    flags.add_argument("--describe", action="store_true")
    args = parser.parse_args()
    if args.table or args.describe:
        if args.request:
            parser.error("a rendered table or format takes no request")
        if args.table:
            print(render_table())
        else:
            print(json.dumps({"request": REQUEST, "round": ROUND, "lane_input": LANE_INPUT,
                              "report_routing": ROUTING, "decision": DECISION}, indent=2))
        return 0
    if not args.request:
        parser.error("a hashed review routing request is required")
    try:
        request = json.loads(args.request.read_bytes())
    except (ValueError, OSError) as error:
        parser.exit(2, f"review request unavailable: {error}\n")
    advice = recommend(request, args.root.resolve())
    if args.claim_challenge:
        current = advice.get("challenge", {})
        if current.get("state") != "ready_to_claim":
            parser.exit(1, "No new challenge is ready for dispatch.\n")
        if not claim(args.root.resolve(), current["manifest"]):
            parser.exit(1, "This challenge already has a dispatch reservation.\n")
        advice["challenge"]["state"] = "claimed"
    print(json.dumps(advice, indent=2, sort_keys=True))
    return 1 if advice["phase"] == "needs_input" else 0


if __name__ == "__main__":
    sys.exit(main())
