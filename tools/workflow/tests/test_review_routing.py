"""Exercise review routing with actual Git candidates and hashed report pairs."""

from hashlib import sha256
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
from result import render, validate
from guidance import check_review_route, validate_guidance
from review_challenge import claim
from review_inputs import Artifacts, SCHEMA, fingerprint, qualification
from review_routing import recommend
from skill_matrix import MATRIX_PATH, MATRIX_REFERENCE


TASK = "fixture-task"
ITEM = "fixture-item"
LANES = ("behavior", "architecture", "tests_evidence", "documentation")
SKILLS = dict(zip(LANES, ("TA-review-behavior", "TA-review-architecture",
                         "TA-review-tests-evidence", "TA-review-docs"), strict=True))


def key(lane, identifier):
    return {"lane": lane, "id": identifier}


class RoutingTests(unittest.TestCase):
    def setUp(self):
        temp = TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name).resolve()
        self.git("init", "-q")
        self.data("source.txt", b"baseline\n")
        self.data(".gitignore", b".agent/\n")
        self.git("add", "source.txt", ".gitignore")
        self.commit("Baseline")
        self.base = self.git("rev-parse", "HEAD").decode().strip()
        self.req = self.data(".agent/requirements", b"Preserve pending IRQ exactly once.\n")
        self.criteria = self.data(".agent/criteria", b"Check full scope.\n")
        self.partition = self.data(".agent/partition", b"One commit.\n")
        self.probe = self.data(".agent/probe", b"Known fixture observation.\n")
        self.profiles = {
            lane: self.json(f".agent/profile-{lane}.json", {
                "lane": lane, "scope": ["source.txt"], "requirements": [self.req],
                "criteria": [self.criteria], "partition": self.partition, "evidence": [self.probe],
            }) for lane in LANES
        }

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.root, capture_output=True, check=True).stdout

    def commit(self, message):
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "commit", "--allow-empty", "-qm", message)

    def data(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return {"path": name, "sha256": sha256(content).hexdigest()}

    def json(self, name, value):
        return self.data(name, json.dumps(value, sort_keys=True).encode())

    def candidate(self, name, content, *, base=None, untracked=None):
        self.data("source.txt", content.encode() + b"\n")
        scope = ["source.txt"]
        if untracked:
            scope.extend(ref["path"] for ref in untracked)
        base = base or self.base
        patch = self.git("diff", "--binary", "--no-ext-diff", "--no-textconv", base, "--", *scope)
        ref = self.data(f".agent/{name}.diff", patch)
        return {"base_commit": base, "scope": sorted(scope), "diff_sha256": ref["sha256"],
                "untracked": untracked or []}, ref

    def round(self, name, number, candidate, patch, previous=None, reports=None, *,
              running=(), missing=(), gate="passed", authorization=None, profiles=None, lanes=LANES):
        reports = reports or {}
        profiles = profiles or self.profiles
        states = {lane: {"input": profiles[lane], "state": "due"} for lane in lanes}
        for lane in running:
            states[lane]["state"] = "running"
        for lane in missing:
            states[lane].update(state="missing", reason="The scoped independent report is missing.")
        for lane, ref in reports.items():
            states[lane].update(state="returned", report=ref)
        gates = [{"id": "commit", "state": gate, "impact": "The canonical commit gate blocks publication."}]
        if gate in ("passed", "failed"):
            gates[0]["evidence"] = self.gate_report(name, candidate, gate)
        record = {"task_id": TASK, "work_item_id": ITEM, "number": number, "previous": previous,
                  "candidate": candidate, "patch": patch, "lane_states": states, "gates": gates}
        if authorization:
            record["started_by"] = authorization
        return self.json(f".agent/{name}.json", record)

    def gate_report(self, name, candidate, verdict):
        stem = f".agent/{name}-gate"
        code = 0 if verdict == "passed" else 1
        artifact = self.data(f"{stem}-output.txt", f"Verifier fixture exit {code}\n".encode())
        result = {
            "schema_version": 2, "task_id": TASK, "work_item_id": ITEM, "run_id": f"{name}-gate",
            "previous_run_id": None, "agent": "main-session", "skill": "TA-verify-batch", "status": "completed",
            "rounds": {"used": 1, "limit": 1, "stop_reason": "Executed fixture gate."},
            "summary": "Verifier evidence for the fixture candidate.",
            "inputs": {"candidate": candidate, "plans": [], "prior_results": [], "instructions": [],
                       "configuration_ref": None, "identity_limits": []},
            "result": {"kind": "verification", "profile": "commit", "verdict": verdict,
                       "report_refs": [artifact], "limits": []},
            "findings": [], "changed_paths": [],
            "evidence": [{"id": "exit", "origin": "executed", "command": "python tools/verification/verify.py commit",
                          "exit_code": code, "artifact_ref": artifact, "candidate_ref": fingerprint(candidate),
                          "validity_basis": "Fixture gate exit."}],
            "observations": {"started_at": None, "finished_at": None, "duration_seconds": None,
                             "model": None, "configuration": None, "limits": []},
            "recommendation": "Preserve the gate status.",
        }
        self.data(f"{stem}.md", render(result).encode())
        return self.json(f"{stem}.json", result)

    def finding(self, identifier, *, severity="major", trigger="Restore pending IRQ"):
        return {"id": identifier, "category": "behavior", "severity": severity, "confidence": "high",
                "scope": "source.txt", "scenario": trigger, "expected": "One IRQ.", "observed": "Two IRQs.",
                "disposition": "open", "origin": "implementation_defect", "evidence_refs": [self.probe]}

    def report(self, name, lane, candidate, *, findings=(), comparisons=(), verdict="approved", fresh=True, routed=True,
               context_id=None):
        context = self.json(f".agent/{name}-context.json", {
            "id": context_id or name, "run_id": name, "fresh": fresh, "inherited_author_history": False,
        })
        result = {
            "schema_version": 2, "task_id": TASK, "work_item_id": ITEM, "run_id": name, "previous_run_id": None,
            "agent": "TA-code-reviewer", "skill": SKILLS[lane], "status": "completed",
            "rounds": {"used": 1, "limit": 1, "stop_reason": "Complete"},
            "summary": "Independent fixture review.",
            "inputs": {"candidate": candidate, "plans": [self.req], "prior_results": [], "instructions": [],
                       "configuration_ref": None, "identity_limits": []},
            "result": {"kind": "review", "lane": lane, "verdict": verdict, "inspected_scope": ["source.txt"],
                       "limits": [], "routing": {"input": self.profiles[lane], "context": context,
                                                  "comparisons": list(comparisons)}},
            "findings": list(findings), "changed_paths": [],
            "evidence": [{"id": "probe", "origin": "inspected", "command": None, "exit_code": None,
                          "artifact_ref": self.probe, "candidate_ref": fingerprint(candidate),
                          "validity_basis": "Fixture source and expected observation."}],
            "observations": {"started_at": None, "finished_at": None, "duration_seconds": None, "model": None,
                             "configuration": None, "limits": []},
            "recommendation": "Return scoped observations.",
        }
        if not routed:
            result["result"].pop("routing")
        self.data(f".agent/{name}.md", render(result).encode())
        return self.json(f".agent/{name}.json", result)

    def assessment(self, lane, identifier, source, *, owner=None, kind="required", duplicate=None):
        row = {"finding": key(lane, identifier), "source": source, "owner": owner or lane, "kind": kind,
               "impact": "Interrupts can be delivered twice.", "basis": "Fixture IRQ requirement.",
               "evidence": [self.probe]}
        if duplicate:
            row["duplicate_of"] = duplicate
        return row

    def compare(self, lane, finding, parent, current, *, previous="fail", now="pass",
                parent_scope="covered", change="none", verified=True):
        comparison = {
            "finding": key(lane, finding["id"]), "trigger": finding["scenario"],
            "parent": {"candidate_ref": fingerprint(parent), "outcome": previous,
                       "evidence": [] if previous == "unavailable" else [self.probe]},
            "current": {"candidate_ref": fingerprint(current), "outcome": now, "evidence": [self.probe]},
            "parent_scope": parent_scope, "change": change,
        }
        if verified:
            comparison["disposition"] = "verified"
        return comparison

    def request(self, head, *, before=None, effects=(), selected=(), assessments=(), paths=("source.txt",), **options):
        data = {"operation": "batch", "task_id": TASK, "work_item_id": ITEM, "head": head,
                "selected_findings": list(selected), "assessments": list(assessments), **options}
        if before:
            data["repair"] = {"before": before, "after": head, "paths": list(paths),
                              "effects": [{"facet": effect, "reason": f"Observed {effect} in fixture.",
                                           "evidence": [self.probe]} for effect in effects]}
        return data

    def advice(self, *args, **kwargs):
        return recommend(self.request(*args, **kwargs), self.root)

    def assert_lanes(self, advice, phase, *lanes):
        self.assertEqual(advice["phase"], phase, advice)
        self.assertEqual([row["lane"] for row in advice["recommended_lanes"]], list(lanes))

    def decision(self, name, advice, choice, *, role="user", reply=None):
        guidance = f"guidance-{name}"
        checkpoint = advice["checkpoint"]["id"]
        prompt = f"review {guidance} {checkpoint}"
        def message(ordinal, role, text):
            return {"type": "response_item", "ordinal": ordinal,
                    "payload": {"type": "message", "role": role, "content": [{"type": "input_text", "text": text}]}}
        transcript = [
            {"type": "session_meta", "payload": {"id": "fixture-thread"}},
            message(1, "assistant", "\n".join(f"{prompt} {code}" for code in ("accept", "continue-one", "pause"))),
            message(2, role, reply if reply is not None else f"{prompt} {choice}"),
        ]
        conversation = self.data(f".agent/{name}.jsonl",
                                 "".join(json.dumps(line) + "\n" for line in transcript).encode())
        return self.json(f".agent/{name}-decision.json", {
            "kind": "human_review_decision", "candidate": advice["candidate"], "round": advice["round"],
            "guidance_id": guidance, "choice": choice, "checkpoint": checkpoint, "conversation": conversation,
            "issue_ids": [item["id"] for item in advice["issues"]], "thread_id": "fixture-thread",
            "offer_ordinal": 1, "reply_ordinal": 2,
        })

    def challenge_return(self, name, challenge, source, *, rejected=(), duplicates=None):
        receipt = self.data(challenge["path"], (self.root / challenge["path"]).read_bytes())
        result = json.loads((self.root / source["path"]).read_bytes())
        result.update(skill="TA-challenge-findings", run_id=name, findings=[])
        context = self.json(f".agent/{name}-context.json", {
            "id": name, "run_id": name, "fresh": True, "inherited_author_history": False,
        })
        duplicates = duplicates or {}
        decisions = [dict(finding=row["finding"],
                          disposition="reject" if row["finding"]["lane"] in rejected else "uphold",
                          preconditions="Open registration.",
                          consequence="Upgrade cannot finish.", severity="major", relevance="Supported handoff.",
                          urgency="Before release.", accepted_risk="None.", reason="The installed revision rejects stop.",
                          support=[self.req], evidence_against=[self.probe],
                          duplicate_of=duplicates.get(row["finding"]["lane"]))
                     for row in challenge["manifest"]["findings"]]
        result["result"] = dict(kind="finding_challenge", collection=receipt, context=context, decisions=decisions)
        self.data(f".agent/{name}.md", render(result).encode())
        return self.json(f".agent/{name}.json", result)

    def test_outstanding_primary_lane_precedes_single_challenge_reservation(self):
        candidate, patch = self.candidate("ready", "Candidate")
        reports = {lane: self.report(lane, lane, candidate,
                                    findings=[self.finding("REQ")] if lane == "architecture" else [],
                                    verdict="changes_requested" if lane == "architecture" else "approved")
                   for lane in LANES if lane != "documentation"}
        assessments = [self.assessment("architecture", "REQ", reports["architecture"])]
        head = self.round("due", 1, candidate, patch, reports=reports)
        advice = self.advice(head, assessments=assessments)
        self.assert_lanes(advice, "changes_requested", "documentation")
        self.assertEqual(advice["challenge"]["state"], "pending_lanes")
        running = self.round("running", 1, candidate, patch, reports=reports, running=("documentation",))
        advice = self.advice(running, assessments=assessments)
        self.assert_lanes(advice, "changes_requested")
        self.assertEqual(advice["outstanding_lanes"], {"documentation": "running"})

        reports["documentation"] = self.report("docs", "documentation", candidate)
        head = self.round("returned", 1, candidate, patch, reports=reports)
        first = self.advice(head, assessments=assessments)["challenge"]
        self.assertEqual(first["state"], "ready_to_claim")
        self.assertTrue(claim(self.root, first["manifest"]))
        reports["documentation"] = self.report("docs-late", "documentation", candidate)
        refreshed = self.round("late-clean", 1, candidate, patch, reports=reports)
        resumed = self.advice(refreshed, assessments=assessments)["challenge"]
        self.assertEqual(resumed["state"], "awaiting_return")
        self.assertEqual(resumed["path"], first["path"])
        self.assertFalse(claim(self.root, resumed["manifest"]))
        self.data(first["path"], b"{}")
        damaged = self.advice(refreshed, assessments=assessments)
        self.assertEqual(damaged["phase"], "needs_input")
        self.assertNotIn("challenge", damaged)

    def test_reassessment_retains_unaffected_findings_and_live_reservation(self):
        candidate, patch = self.candidate("claims", "Candidate")
        ids = {"architecture": "A-first", "behavior": "B-second"}
        reports = {lane: self.report(lane, lane, candidate,
                                    findings=[self.finding(ids[lane])] if lane in ids else [],
                                    verdict="changes_requested" if lane in ids else "approved")
                   for lane in LANES}
        head = self.round("claims", 1, candidate, patch, reports=reports)
        request = self.request(head, assessments=[self.assessment(lane, item, reports[lane])
                                                  for lane, item in ids.items()])
        first = recommend(request, self.root)["challenge"]
        self.assertTrue(claim(self.root, first["manifest"]))
        earlier = self.challenge_return("first-challenger", first, reports["architecture"])
        first_key = key("architecture", ids["architecture"])
        second_key = key("behavior", ids["behavior"])
        request["risk_decisions"] = [{"finding": first_key, "evidence": self.data(
            ".agent/changed-decision", b"The architecture premise has new evidence.")}]
        pending = recommend(request, self.root)["challenge"]
        self.assertEqual(pending["path"], first["path"])
        self.assertEqual(pending["state"], "awaiting_return")
        request["challenge_report"] = earlier
        reassessment = recommend(request, self.root)["challenge"]
        self.assertEqual(reassessment["state"], "ready_to_claim")
        self.assertEqual([row["finding"] for row in reassessment["manifest"]["findings"]], [first_key])
        self.assertEqual(reassessment["manifest"]["previous"]["report"], earlier)
        self.assertEqual([row["finding"] for row in reassessment["eligible_repairs"]], [second_key])
        self.assertTrue(claim(self.root, reassessment["manifest"]))
        self.assertEqual(recommend(request, self.root)["challenge"]["state"], "awaiting_return")
        request["prior_challenge_reports"] = [earlier]
        request["challenge_report"] = self.challenge_return("second-challenger", reassessment, reports["architecture"])
        returned = recommend(request, self.root)["challenge"]
        self.assertEqual(returned["state"], "returned")
        self.assertEqual({(row["finding"]["lane"], row["finding"]["id"]) for row in returned["eligible_repairs"]},
                         set(ids.items()))
        self.assertEqual(returned["path"], reassessment["path"])

    def challenge_fixture(self):
        candidate, patch = self.candidate("handoff", "supported registration migration")
        ids = {"architecture": "ARCH-12", "documentation": "DOC-WF01-03"}
        reports = {lane: self.report(lane, lane, candidate,
                                     findings=[self.finding(ids[lane])] if lane in ids else [],
                                     verdict="changes_requested" if lane in ids else "approved")
                   for lane in LANES}
        assessments = [self.assessment(lane, item, reports[lane]) for lane, item in ids.items()]
        return candidate, patch, ids, reports, assessments

    def test_upheld_findings_both_reach_repair_with_duplicate_link(self):
        candidate, patch, ids, reports, assessments = self.challenge_fixture()
        head = self.round("handoff", 1, candidate, patch, reports=reports)
        request = self.request(head, assessments=assessments)
        challenge = recommend(request, self.root)["challenge"]
        self.assertEqual(challenge["state"], "ready_to_claim")
        self.assertTrue(claim(self.root, challenge["manifest"]))
        self.assertFalse(claim(self.root, challenge["manifest"]))
        self.assertEqual(recommend(request, self.root)["challenge"]["state"], "awaiting_return")
        architecture, documentation = (key(lane, identifier) for lane, identifier in ids.items())
        request["challenge_report"] = self.challenge_return(
            "upheld", challenge, reports["architecture"], duplicates={"documentation": architecture},
        )
        advice = recommend(request, self.root)
        self.assert_lanes(advice, "changes_requested")
        self.assertEqual(set(advice["approvals"]), {"behavior", "tests_evidence"})
        self.assertFalse(advice["resolved_lanes"])
        self.assertEqual(advice["challenge"]["rejected"], [])
        repairs = advice["challenge"]["eligible_repairs"]
        self.assertEqual([row["finding"] for row in repairs], [architecture, documentation])
        self.assertEqual(repairs[1]["duplicate_of"], architecture)

    def test_all_rejected_findings_close_without_repeating_primary_reviews(self):
        candidate, patch, ids, reports, assessments = self.challenge_fixture()
        head = self.round("handoff", 1, candidate, patch, reports=reports)
        request = self.request(head, assessments=assessments)
        challenge = recommend(request, self.root)["challenge"]
        self.assertTrue(claim(self.root, challenge["manifest"]))
        response = self.challenge_return("rejected", challenge, reports["architecture"], rejected=tuple(ids))
        request["challenge_report"] = response
        advice = recommend(request, self.root)
        self.assert_lanes(advice, "done")
        self.assertEqual(set(advice["approvals"]), {"behavior", "tests_evidence"})
        self.assertEqual(advice["resolved_lanes"], {
            lane: {"report": reports[lane], "findings": [{"finding": key(lane, identifier), "report": response}]}
            for lane, identifier in ids.items()
        })
        self.assertEqual(advice["challenge"]["eligible_repairs"], [])
        self.assertEqual([row["finding"] for row in advice["challenge"]["rejected"]],
                         [key(lane, identifier) for lane, identifier in ids.items()])
        self.assertEqual(recommend(request, self.root), advice)
        for lane in ids:
            original = (self.root / reports[lane]["path"]).read_bytes()
            self.assertEqual(sha256(original).hexdigest(), reports[lane]["sha256"])
            self.assertEqual(json.loads(original)["result"]["verdict"], "changes_requested")

        for gate in ("failed", "missing"):
            with self.subTest(gate=gate):
                request["head"] = self.round(f"{gate}-gate", 1, candidate, patch, reports=reports, gate=gate)
                blocked = recommend(request, self.root)
                self.assert_lanes(blocked, "changes_requested")
                self.assertEqual(blocked["resolved_lanes"], advice["resolved_lanes"])
                self.assertTrue(any(row["id"] == f"gate:commit:{gate}" for row in blocked["issues"]))

        partial = dict(reports)
        partial.pop("behavior")
        request["head"] = self.round("missing-lane", 1, candidate, patch, reports=partial, missing=("behavior",))
        blocked = recommend(request, self.root)
        self.assertNotEqual(blocked["phase"], "done")
        self.assertEqual(blocked["outstanding_lanes"], {"behavior": "missing"})

    def test_reject_stays_closed_after_other_finding_is_repaired(self):
        original, original_patch, ids, primary, assessments = self.challenge_fixture()
        parent = self.round("handoff", 1, original, original_patch, reports=primary)
        first_request = self.request(parent, assessments=assessments)
        challenge = recommend(first_request, self.root)["challenge"]
        self.assertTrue(claim(self.root, challenge["manifest"]))
        decision = self.challenge_return(
            "mixed", challenge, primary["architecture"], rejected=("architecture",),
        )
        first_request["challenge_report"] = decision
        first = recommend(first_request, self.root)
        supported = key("documentation", ids["documentation"])
        rejected = key("architecture", ids["architecture"])
        self.assertEqual([row["finding"] for row in first["challenge"]["eligible_repairs"]], [supported])
        self.assertEqual([row["finding"] for row in first["challenge"]["rejected"]], [rejected])

        repaired, repaired_patch = self.candidate("repaired", "Corrected registration handoff")
        reports = {
            lane: self.report(
                f"repaired-{lane}", lane, repaired,
                comparisons=[self.compare(lane, self.finding(ids[lane]), original, repaired)]
                if lane == "documentation" else (),
            )
            for lane in LANES
        }
        head = self.round("repaired", 2, repaired, repaired_patch, previous=parent, reports=reports)
        request = self.request(
            head, before=parent, effects=("wording",), selected=[supported], assessments=assessments,
            prior_challenge_reports=[decision],
        )
        completed = recommend(request, self.root)
        self.assert_lanes(completed, "done")
        self.assertEqual(completed["issues"], [])
        self.assertEqual(completed["verified"], [qualification(TASK, ITEM, ("documentation", supported["id"]))])
        self.assertEqual(set(completed["approvals"]), set(LANES))
        self.assertEqual(completed["challenge"]["state"], "skipped")

        current_finding = self.finding(ids["architecture"])
        current_finding["evidence_refs"] = [self.data(
            ".agent/current-architecture-probe", b"The repaired candidate introduces duplicate IRQ delivery.\n",
        )]
        comparison = self.compare("architecture", current_finding, original, repaired,
                                  previous="pass", now="fail", verified=False)
        comparison["current"]["evidence"] = current_finding["evidence_refs"]
        reports["architecture"] = self.report(
            "regressed-architecture", "architecture", repaired, findings=[current_finding],
            comparisons=[comparison], verdict="changes_requested",
        )
        request["head"] = self.round("regressed", 2, repaired, repaired_patch, previous=parent, reports=reports)
        opened = recommend(request, self.root)
        self.assert_lanes(opened, "changes_requested")
        self.assertEqual(opened["challenge"]["state"], "ready_to_claim")
        offered, = opened["challenge"]["manifest"]["findings"]
        self.assertEqual(offered["finding"], rejected)
        self.assertEqual(offered["report"], reports["architecture"])

    def test_mixed_duplicate_decisions_keep_the_upheld_findings_own_trigger(self):
        original, patch = self.candidate("mixed-duplicates", "Shared buggy path")
        architecture = self.finding("ARCH-COMMON", trigger="Resume the upgraded installation")
        documentation = self.finding("DOC-COMMON", trigger="Stop a registration before upgrade")
        findings = {"architecture": architecture, "documentation": documentation}
        reports = {lane: self.report(lane, lane, original, findings=[findings[lane]] if lane in findings else [],
                                     verdict="changes_requested" if lane in findings else "approved")
                   for lane in LANES}
        rejected = key("architecture", architecture["id"])
        upheld = key("documentation", documentation["id"])
        assessments = [self.assessment("architecture", architecture["id"], reports["architecture"]),
                       self.assessment("documentation", documentation["id"], reports["documentation"],
                                       owner="architecture", duplicate=rejected)]
        first = self.round("mixed-duplicates", 1, original, patch, reports=reports)
        request = self.request(first, assessments=assessments)
        challenge = recommend(request, self.root)["challenge"]
        self.assertTrue(claim(self.root, challenge["manifest"]))
        returned = self.challenge_return("mixed-duplicate-decisions", challenge, reports["architecture"],
                                         rejected=("architecture",), duplicates={"documentation": rejected})
        decisions = recommend({**request, "challenge_report": returned}, self.root)["challenge"]
        self.assertEqual([row["finding"] for row in decisions["rejected"]], [rejected])
        self.assertEqual([row["finding"] for row in decisions["eligible_repairs"]], [upheld])
        self.assertEqual(decisions["eligible_repairs"][0]["duplicate_of"], rejected)

        corrected, corrected_patch = self.candidate("corrected-duplicate", "Shared path handles old revision")
        current = {lane: self.report(f"corrected-{lane}", lane, corrected) for lane in LANES}
        current["architecture"] = self.report(
            "corrected-owner", "architecture", corrected,
            comparisons=[self.compare("documentation", documentation, original, corrected)],
        )
        second = self.round("corrected-duplicate", 2, corrected, corrected_patch, previous=first, reports=current)
        advice = self.advice(second, before=first, effects=("behavior",), assessments=assessments,
                             selected=[upheld], prior_challenge_reports=[returned])
        self.assert_lanes(advice, "done")
        self.assertEqual(advice["issues"], [])
        self.assertEqual(advice["challenge"]["state"], "skipped")
        self.assertEqual(advice["verified"], [qualification(TASK, ITEM, ("documentation", documentation["id"]))])
        self.assertEqual(advice["duplicate_observations"], [{
            "observation": qualification(TASK, ITEM, ("documentation", documentation["id"])),
            "finding": qualification(TASK, ITEM, ("architecture", architecture["id"])),
        }])

    def test_unsupported_or_incomplete_challenge_returns_cannot_authorize_work(self):
        candidate, patch, _, reports, assessments = self.challenge_fixture()
        head = self.round("handoff", 1, candidate, patch, reports=reports)
        request = self.request(head, assessments=assessments)
        challenge = recommend(request, self.root)["challenge"]
        self.assertTrue(claim(self.root, challenge["manifest"]))
        response = self.challenge_return("invalid", challenge, reports["architecture"])
        original = (self.root / response["path"]).read_bytes()
        for disposition in ("narrow", "reject_basis", "missing_fact"):
            with self.subTest(disposition=disposition):
                result = json.loads(original)
                result["result"]["decisions"][0]["disposition"] = disposition
                self.assertTrue(validate(result, SCHEMA, self.root))
                self.data(".agent/invalid.md", render(result).encode())
                request["challenge_report"] = self.json(response["path"], result)
                self.assert_lanes(recommend(request, self.root), "needs_input")

        result = json.loads(original)
        result["result"]["decisions"].pop()
        self.data(".agent/invalid.md", render(result).encode())
        request["challenge_report"] = self.json(response["path"], result)
        self.assert_lanes(recommend(request, self.root), "needs_input")

    def test_first_batch_standalone_and_table_match_the_approved_scope(self):
        a, ap = self.candidate("a", "A")
        first = self.round("a", 1, a, ap)
        self.assert_lanes(self.advice(first), "first", *LANES)
        only = self.round("standalone", 1, a, ap, lanes=("documentation",))
        self.assert_lanes(self.advice(only, operation="TA-review-docs"), "first", "documentation")
        table = subprocess.check_output([sys.executable, str(TOOLS / "review_routing.py"), "--table"], text=True)
        expected = [
            ("Round 3", ("Stop", "Stop", "Stop", "Stop")),
            ("First batch", ("First", "First", "First", "First")),
            ("Descriptive wording", ("", "", "", "First")),
            ("Observable behavior", ("First", "", "", "")),
            ("Ownership", ("", "First", "", "")),
            ("Assertions", ("", "", "First", "")),
            ("A binding requirement", ("First", "First", "First", "First")),
            ("Several known", ("Include if affected",) * 4),
            ("Impact cannot", ("First",) * 4),
            ("All focused", ("Remaining",) * 4),
            ("Every required", ("Done",) * 4),
        ]
        for line, (label, cells) in zip(table.splitlines()[2:], expected, strict=True):
            row = [value.strip() for value in line.split("|")[1:-1]]
            self.assertTrue(row[0].startswith(label), row)
            self.assertEqual(tuple(row[1:]), cells)

    def test_qualified_duplicates_pending_and_fresh_remaining_approvals(self):
        a, ap = self.candidate("a", "A")
        docs_issue, tests_issue = self.finding("X"), self.finding("X", trigger="Check gate output")
        docs = self.report("docs-a", "documentation", a, findings=[docs_issue], verdict="changes_requested")
        tests = self.report("tests-a", "tests_evidence", a, findings=[tests_issue], verdict="changes_requested")
        behavior_issue = self.finding("Y", trigger="Restore IRQ after a second wait")
        alias = self.report("behavior-a", "behavior", a, findings=[behavior_issue], verdict="changes_requested")
        ar = self.round("a", 1, a, ap, reports={"documentation": docs, "tests_evidence": tests, "behavior": alias})
        claims = [
            self.assessment("documentation", "X", docs), self.assessment("tests_evidence", "X", tests),
            self.assessment("behavior", "Y", alias, owner="documentation", duplicate=key("documentation", "X")),
        ]
        pending = [key("behavior", "Y"), key("documentation", "X"), key("tests_evidence", "X")]
        b, bp = self.candidate("b", "B")
        br = self.round("b", 2, b, bp, previous=ar)
        options = {"before": ar, "effects": ("wording", "evidence"), "assessments": claims, "selected": pending}
        advice = self.advice(br, **options)
        self.assert_lanes(advice, "focused", "tests_evidence", "documentation")
        self.assertEqual(len(advice["pending"]), 3)
        self.assertEqual(len(advice["duplicate_observations"]), 1)
        self.assertEqual(advice, self.advice(br, **{**options, "assessments": claims[::-1], "selected": pending[::-1]}))
        self.assert_lanes(self.advice(br, **{**options, "selected": [key("behavior", "unknown")]}), "needs_input")
        invalid_alias = {**claims[2], "duplicate_of": key("behavior", "Y")}
        self.assert_lanes(self.advice(br, **{**options, "assessments": [*claims[:2], invalid_alias]}), "needs_input")
        stale = self.round("stale", 2, b, bp, previous=ar, reports={"documentation": docs})
        self.assert_lanes(self.advice(stale, **options), "needs_input")
        for_done = {
            "documentation": self.report("docs-b", "documentation", b,
                                         comparisons=[self.compare("documentation", docs_issue, a, b),
                                                      self.compare("behavior", behavior_issue, a, b)]),
            "tests_evidence": self.report("tests-b", "tests_evidence", b,
                                          comparisons=[self.compare("tests_evidence", tests_issue, a, b)]),
        }
        br = self.round("b-verified", 2, b, bp, previous=ar, reports=for_done)
        advice = self.advice(br, **options)
        self.assert_lanes(advice, "remaining", "behavior", "architecture")
        self.assertEqual(advice["verified"], [qualification(TASK, ITEM, finding) for finding in (
            ("behavior", "Y"), ("documentation", "X"), ("tests_evidence", "X"),
        )])
        for lane in ("behavior", "architecture"):
            for_done[lane] = self.report(f"{lane}-b", lane, b)
        br = self.round("b-complete", 2, b, bp, previous=ar, reports=for_done)
        self.assert_lanes(self.advice(br, **options), "done")

    def test_effect_union_and_changed_baseline_use_actual_delta(self):
        a, ap = self.candidate("a", "A")
        finding = self.finding("arbitrary-opaque-id")
        test_report = self.report("tests-a", "tests_evidence", a, findings=[finding], verdict="changes_requested")
        ar = self.round("a", 1, a, ap, reports={"tests_evidence": test_report})
        b, bp = self.candidate("b", "B")
        br = self.round("b", 2, b, bp, previous=ar)
        options = {"before": ar, "effects": ("behavior", "wording"),
                   "assessments": [self.assessment("tests_evidence", finding["id"], test_report)],
                   "selected": [key("tests_evidence", finding["id"])]}
        self.assert_lanes(self.advice(br, **options), "focused", "behavior", "tests_evidence", "documentation")
        self.assert_lanes(self.advice(br, **{**options, "effects": ("requirements",)}), "focused", *LANES)
        self.assert_lanes(self.advice(br, **{**options, "effects": ("unknown",)}), "focused", *LANES)
        self.assert_lanes(self.advice(br, paths=("unrelated.txt",), **options), "needs_input")
        self.commit("New immutable baseline")
        next_base = self.git("rev-parse", "HEAD").decode().strip()
        c, cp = self.candidate("c", "C", base=next_base)
        changed = self.round("changed-base", 2, c, cp, previous=ar)
        self.assert_lanes(self.advice(changed, **options), "focused", *LANES)

    def test_repair_delta_covers_native_multi_file_binary_and_rename_paths(self):
        self.data("old name.txt", b"Native rename source\n")
        self.data("table.bin", bytes(range(256)))
        self.git("add", "old name.txt", "table.bin")
        self.commit("Add native diff fixtures")
        base = self.git("rev-parse", "HEAD").decode().strip()

        def snapshot(name):
            patch = self.git("diff", "--binary", "--no-ext-diff", "--no-textconv", base, "--", ".")
            patch_ref = self.data(f".agent/{name}.diff", patch)
            return {"patch": patch_ref, "candidate": {"base_commit": base, "untracked": []}}

        self.data("source.txt", b"First checked source\n")
        self.data("table.bin", b"\0first binary contents")
        before = snapshot("before")
        self.data("source.txt", b"Second checked source\n")
        self.data("table.bin", b"\0second binary contents")
        self.git("mv", "old name.txt", "destination name.txt")
        after = snapshot("after")
        prior_index = (self.root / ".git/index").read_bytes()
        prior_objects = self.git("count-objects", "-v")
        paths = Artifacts(self.root).changed_paths(before, after)
        self.assertEqual(paths, ["destination name.txt", "old name.txt", "source.txt", "table.bin"])
        self.assertEqual((self.root / ".git/index").read_bytes(), prior_index)
        self.assertEqual(self.git("count-objects", "-v"), prior_objects)

    def test_corrected_source_citation_uses_the_exact_parent_candidate(self):
        self.git("config", "--local", "core.autocrlf", "true")
        a, _ = self.candidate("discarded", "First candidate")
        original = self.data("source.txt", b"First candidate\r\n")
        ap = self.data(".agent/a.diff", self.git("diff", "--binary", "--no-ext-diff", "--no-textconv",
                                               self.base, "--", "source.txt"))
        self.assertEqual(ap["sha256"], a["diff_sha256"])
        finding = self.finding("fixed")
        finding["evidence_refs"] = [original]
        old = self.report("behavior-a", "behavior", a, findings=[finding], verdict="changes_requested")
        ar = self.round("a", 1, a, ap, reports={"behavior": old})
        b, bp = self.candidate("b", "Corrected candidate")
        comparison = self.compare("behavior", finding, a, b)
        comparison["parent"]["evidence"] = [original]
        passed = self.data(".agent/corrected-observation.txt", b"The IRQ was delivered once.\n")
        comparison["current"]["evidence"] = [passed]
        report = self.report("behavior-b", "behavior", b, comparisons=[comparison])
        head = self.round("b", 2, b, bp, previous=ar, reports={"behavior": report})
        options = {"before": ar, "effects": ("behavior",), "selected": [key("behavior", "fixed")],
                   "assessments": [self.assessment("behavior", "fixed", old)]}
        advice = self.advice(head, **options)
        self.assert_lanes(advice, "remaining", "architecture", "tests_evidence", "documentation")
        self.assertEqual([finding["id"] for finding in advice["verified"]], ["fixed"])
        self.assertEqual(advice["approvals"], {"behavior": "behavior-b"})

        wrong = {**original, "sha256": "0" * 64}
        external = self.data(".agent/external-source-proof.txt", b"Original measurement")
        self.data(external["path"], b"Different measurement")
        for ref in (wrong, external):
            with self.subTest(reference=ref):
                comparison["parent"]["evidence"] = [ref]
                changed = self.report("behavior-invalid", "behavior", b, comparisons=[comparison])
                invalid_head = self.round("invalid", 2, b, bp, previous=ar, reports={"behavior": changed})
                advice = self.advice(invalid_head, **options)
                self.assert_lanes(advice, "needs_input")
                self.assertIn(f"changed reference: {ref['path']}", advice["gaps"])

        (self.root / passed["path"]).unlink()
        self.assert_lanes(self.advice(head, **options), "needs_input")
        c, cp = self.candidate("c", "Next candidate")
        third = self.round("c", 3, c, cp, previous=head, gate="missing")
        advice = self.advice(third, **{**options, "before": head, "selected": []})
        self.assert_lanes(advice, "human_decision_required")
        self.assertEqual([issue["id"] for issue in advice["checkpoint"]["issues"]], ["gate:commit:missing"])

    def test_historical_baseline_source_outside_diff_and_private_artifact(self):
        self.git("config", "--local", "core.autocrlf", "true")
        baseline = self.data("rule.txt", b"Original recorded criterion\r\n")
        self.git("add", "rule.txt")
        self.commit("Record a separate baseline criterion")
        self.base = self.git("rev-parse", "HEAD").decode().strip()
        external = self.data(".agent/private-observation.txt", b"Original measurement")
        old_candidate, old_patch = self.candidate("old", "Proposed implementation")
        self.assertNotIn("rule.txt", old_candidate["scope"])
        old = self.finding("fixed")
        old["evidence_refs"] = [baseline, external]
        old_report = self.report("old", "behavior", old_candidate, findings=[old], verdict="changes_requested")
        old_head = self.round("old-round", 1, old_candidate, old_patch, reports={"behavior": old_report})

        current = self.data("rule.txt", b"Changed criterion\r\n")
        new_candidate, new_patch = self.candidate("new", "Corrected implementation")
        comparison = self.compare("behavior", old, old_candidate, new_candidate)
        comparison["parent"]["evidence"] = [baseline]
        options = {"before": old_head, "effects": ("behavior",), "selected": [key("behavior", "fixed")],
                   "assessments": [self.assessment("behavior", "fixed", old_report)]}

        def route(name):
            ref = self.report(name, "behavior", new_candidate, comparisons=[comparison])
            head = self.round(f"{name}-round", 2, new_candidate, new_patch, previous=old_head, reports={"behavior": ref})
            return self.advice(head, **options)

        valid = route("valid")
        self.assertEqual([row["id"] for row in valid["verified"]], ["fixed"])
        self.assertEqual(valid["approvals"], {"behavior": "valid"})
        comparison["parent"]["evidence"] = [current]
        self.assertIn("changed reference: rule.txt", route("future-citation")["gaps"])
        comparison["parent"]["evidence"] = [baseline]
        self.data(external["path"], b"Changed private measurement")
        damaged = route("damaged-private")
        self.assert_lanes(damaged, "needs_input")
        self.assertIn(f"changed reference: {external['path']}", damaged["gaps"])

    def test_changed_tracked_requirement_reaches_the_current_checkpoint(self):
        lane = "documentation"
        original_profile = json.loads((self.root / self.profiles[lane]["path"]).read_bytes())

        def profile(name, ref):
            self.profiles[lane] = self.json(f".agent/{name}-doc-input.json",
                                            {**original_profile, "requirements": [ref]})

        a, ap = self.candidate("a", "First requirement")
        original = self.data("source.txt", b"First requirement\n")
        profile("first", original)
        first_report = self.report("docs-a", lane, a)
        ar = self.round("a", 1, a, ap, reports={lane: first_report})

        b, bp = self.candidate("b", "Revised requirement")
        revised = self.data("source.txt", b"Revised requirement\n")
        profile("revised", revised)
        br = self.round("b", 2, b, bp, previous=ar)
        advice = self.advice(br, before=ar, effects=("requirements",))
        self.assert_lanes(advice, "focused", *LANES)
        self.assertEqual(advice["approvals"], {})

        c, cp = self.candidate("c", "Current requirement")
        current = self.data("source.txt", b"Current requirement\n")
        profile("current", current)
        current_report = self.report("docs-c", lane, c)
        cr = self.round("c", 3, c, cp, previous=br, reports={lane: current_report}, gate="missing")
        advice = self.advice(cr, before=br, effects=("requirements",))
        self.assert_lanes(advice, "human_decision_required")
        self.assertEqual(advice["round"], 3)
        self.assertEqual(advice["approvals"], {lane: "docs-c"})
        self.assertEqual(advice["executable_options"], [])
        self.assertEqual([issue["id"] for issue in advice["checkpoint"]["issues"]], ["gate:commit:missing"])

        profile("stale-current", revised)
        invalid = self.round("stale-current", 3, c, cp, previous=br, gate="missing")
        advice = self.advice(invalid, before=br, effects=("requirements",))
        self.assert_lanes(advice, "needs_input")
        self.assertIn("changed reference: source.txt", advice["gaps"])

    def test_revised_untracked_requirement_and_retired_profile_inputs_allow_checkpoint(self):
        lane = "documentation"
        plan = self.data(".agent/initial-plan.md", b"Initial plan.\n")
        criteria = self.data(".agent/initial-criteria.md", b"Initial review criteria.\n")
        partition = self.data(".agent/initial-partition.md", b"Initial batch partition.\n")
        observation = self.data(".agent/initial-review.txt", b"Initial review observation.\n")
        profile = json.loads((self.root / self.profiles[lane]["path"]).read_bytes())
        req, probe = self.req, self.probe
        previous = None
        for number in (1, 2, 3):
            acceptance = self.data("acceptance.md", f"Approved requirement {number}.\n".encode())
            candidate, patch = self.candidate(f"candidate-{number}", f"Change {number}", untracked=[acceptance])
            scoped = {**profile, "requirements": [req, acceptance]}
            if number == 1:
                scoped.update(requirements=[plan, acceptance], criteria=[criteria],
                              partition=partition, evidence=[observation])
                self.req, self.probe = plan, observation
            self.profiles[lane] = self.json(f".agent/doc-profile-{number}.json", scoped)
            report = self.report(f"docs-{number}", lane, candidate)
            self.req, self.probe = req, probe
            head = self.round(f"round-{number}", number, candidate, patch, previous=previous,
                              reports={lane: report}, gate="missing" if number == 3 else "passed")
            if number < 3:
                previous = head

        self.data(plan["path"], b"Revised initial plan.\n")
        for retired in (criteria, partition, observation):
            (self.root / retired["path"]).unlink()
        advice = self.advice(head, before=previous, effects=("requirements",), paths=("source.txt", "acceptance.md"))
        self.assert_lanes(advice, "human_decision_required")
        self.assertEqual(advice["approvals"], {lane: "docs-3"})
        self.assertEqual(advice["gaps"], [])
        self.assertEqual([issue["id"] for issue in advice["checkpoint"]["issues"]], ["gate:commit:missing"])

    def test_review_comparison_distinguishes_regression_missed_scope_and_unknown(self):
        a, ap = self.candidate("a", "A")
        old = self.finding("lost")
        source = self.report("behavior-a", "behavior", a, findings=[old], verdict="changes_requested")
        ar = self.round("a", 1, a, ap, reports={"behavior": source})
        b, bp = self.candidate("b", "B")
        new = [self.finding(name, trigger=name) for name in ("double", "truncated", "gap", "unknown")]
        comparisons = [self.compare("behavior", old, a, b)]
        comparisons.extend(self.compare("behavior", finding, a, b, previous=parent, now="fail",
                                        parent_scope=scope, verified=False)
                           for finding, parent, scope in zip(new, ("pass", "fail", "fail", "unavailable"),
                                                           ("covered", "covered", "documented_gap", "unknown")))
        report = self.report("behavior-b", "behavior", b, findings=new, comparisons=comparisons,
                             verdict="changes_requested")
        br = self.round("b", 2, b, bp, previous=ar, reports={"behavior": report})
        assessments = [self.assessment("behavior", "lost", source)]
        assessments.extend(self.assessment("behavior", finding["id"], report) for finding in new)
        advice = self.advice(br, before=ar, effects=("behavior",), selected=[key("behavior", "lost")],
                             assessments=assessments)
        self.assert_lanes(advice, "changes_requested", "architecture", "documentation", "tests_evidence")
        self.assertEqual([row["id"] for row in advice["verified"]], ["lost"])
        self.assertEqual({issue["finding"]["id"]: issue["history"] for issue in advice["issues"]}, {
            "double": "repair_regression", "truncated": "previously_missed",
            "gap": "previously_unchecked", "unknown": "uncertain",
        })
        self.assertTrue(all(issue["origin"] == "implementation_defect" for issue in advice["issues"]))

    def three_rounds(self):
        a, ap = self.candidate("a", "A")
        ar = self.round("a", 1, a, ap)
        b, bp = self.candidate("b", "B")
        br = self.round("b", 2, b, bp, previous=ar)
        c, cp = self.candidate("c", "C")
        return br, c, cp

    def test_checkpoint_waits_for_running_work_and_stops_before_new_lanes(self):
        br, c, cp = self.three_rounds()
        focused = {lane: self.report(f"{lane}-c", lane, c) for lane in ("documentation", "tests_evidence")}
        cr = self.round("clean-wave", 3, c, cp, previous=br, reports=focused)
        opts = {"before": br, "effects": ("evidence", "wording")}
        self.assert_lanes(self.advice(cr, **opts), "remaining", "behavior", "architecture")
        cr = self.round("running-wave", 3, c, cp, previous=br, reports=focused,
                        running=("behavior", "architecture"))
        self.assert_lanes(self.advice(cr, **opts), "awaiting_evidence")
        late = self.report("late", "behavior", c, findings=[self.finding("late")], verdict="changes_requested")
        cr = self.round("stopped-wave", 3, c, cp, previous=br, reports={**focused, "behavior": late},
                        running=("architecture",))
        opts["assessments"] = [self.assessment("behavior", "late", late)]
        advice = self.advice(cr, **opts)
        self.assert_lanes(advice, "human_decision_required")
        self.assertEqual(advice["executable_options"], [])
        self.assertEqual(advice["round"], 3)
        focused["architecture"] = self.report("late-architecture", "architecture", c)
        refreshed = self.round("collected-wave", 3, c, cp, previous=br, reports={**focused, "behavior": late})
        self.assertEqual(advice["checkpoint"]["id"], self.advice(refreshed, **opts)["checkpoint"]["id"])
        missing = self.round("confirmed-missing", 3, c, cp, previous=br, missing=("behavior",))
        self.assert_lanes(self.advice(missing, before=br, effects=("evidence",)), "human_decision_required")

    def test_saved_checkpoint_resumes_collection_and_final_challenge(self):
        previous, candidate, patch = self.three_rounds()
        reports = {lane: self.report(lane, lane, candidate,
                                    findings=[self.finding("REQ")] if lane == "architecture" else [],
                                    verdict="changes_requested" if lane == "architecture" else "approved")
                   for lane in LANES if lane != "documentation"}
        options = {"before": previous, "effects": ("architecture",),
                   "assessments": [self.assessment("architecture", "REQ", reports["architecture"])]}
        head = self.round("unstarted", 3, candidate, patch, previous=previous, reports=reports)
        blocked = self.advice(head, **options)
        self.assert_lanes(blocked, "human_decision_required")
        self.assertEqual(blocked["challenge"], {"state": "checkpoint_pending",
                                               "findings": [key("architecture", "REQ")]})
        self.assertEqual(blocked["outstanding_lanes"], {"documentation": "due"})
        decision = self.decision("continue-collection", blocked, "continue-one")
        authorized = self.advice(head, **options, decision=decision)
        self.assertEqual(authorized["challenge"]["state"], "checkpoint_pending")
        continuation = authorized["checkpoint"]["continuation"]
        saved = self.json(continuation["path"], continuation["record"])
        waiting = self.advice(head, **options)
        self.assert_lanes(waiting, "continuation_already_selected", "documentation")
        self.assertEqual(waiting["challenge"]["state"], "pending_lanes")

        def publish(request, source):
            ref = self.json(".agent/authorized-route.json", request)
            self.publish_route(ref)
            return {"review_route_ref": ref, "task_id": TASK, "work_item_id": ITEM,
                    "current_candidate": candidate, "source_result": source}

        def guidance_errors(guidance, skill, ids, inputs=()):
            ops = [{"id": "selection", "kind": "operation", "assignment": {
                "skill": skill, "selected_findings": ids, "round_limit": 1, "input_refs": list(inputs),
            }}]
            source = json.loads((self.root / guidance["source_result"]["path"]).read_bytes())
            errors = []
            check_review_route({**guidance, "options": ops}, source, self.root, errors)
            return errors

        collection = publish(self.request(head, **options), reports["architecture"])
        self.assertEqual(guidance_errors(collection, "TA-review-docs", []), [])
        self.assertTrue(guidance_errors(collection, "TA-review-behavior", []))
        self.assertTrue(guidance_errors(collection, "TA-commit-batch", []))
        self.assertTrue(guidance_errors(collection, "TA-challenge-findings", ["architecture/REQ"]))
        reports["documentation"] = self.report("docs", "documentation", candidate)
        head = self.round("collected", 3, candidate, patch, previous=previous, reports=reports)
        request = self.request(head, **options)
        ready = recommend(request, self.root)
        self.assert_lanes(ready, "continuation_already_selected")
        self.assertEqual(ready["checkpoint"]["id"], blocked["checkpoint"]["id"])
        self.assertEqual(ready["challenge"]["manifest"]["round"], 3)
        self.assertEqual(ready["challenge"]["manifest"]["candidate"], candidate)
        source = reports["architecture"]
        guidance = publish(request, source)
        self.assertEqual(guidance_errors(guidance, "TA-challenge-findings", ["architecture/REQ"]), [])
        self.assertTrue(guidance_errors(guidance, "TA-challenge-findings", ["OTHER"]))
        self.assertTrue(guidance_errors(guidance, "TA-fix-findings", ["architecture/REQ"]))
        self.assertTrue(guidance_errors(guidance, "TA-review-docs", []))
        cli = [sys.executable, str(TOOLS / "review_routing.py"), guidance["review_route_ref"]["path"],
               "--claim-challenge"]
        process = subprocess.run(cli, cwd=self.root, capture_output=True, encoding="utf-8")
        self.assertEqual(process.returncode, 0, process.stderr)
        claimed = json.loads(process.stdout)["challenge"]
        self.assertEqual(claimed["state"], "claimed")
        self.assertTrue(guidance_errors(guidance, "TA-challenge-findings", ["architecture/REQ"]))

        for rejected in ((), ("architecture",)):
            with self.subTest(rejected=rejected):
                response = self.challenge_return(
                    "challenger-rejected" if rejected else "challenger-upheld",
                    claimed, source, rejected=rejected,
                )
                request["challenge_report"] = response
                returned = recommend(request, self.root)
                self.assert_lanes(returned, "done" if rejected else "continuation_already_selected")
                guidance = publish(request, response)
                self.assertTrue(guidance_errors(guidance, "TA-review-architecture", ["architecture/REQ"], [source]))
                self.assertTrue(guidance_errors(guidance, "TA-review-architecture", [], [source]))
                repair_errors = guidance_errors(guidance, "TA-fix-findings", ["architecture/REQ"], [source])
                if rejected:
                    self.assertTrue(repair_errors)
                    self.assertEqual(guidance_errors(guidance, "TA-commit-batch", []), [])
                else:
                    self.assertEqual(repair_errors, [])
                    self.assertTrue(guidance_errors(guidance, "TA-commit-batch", []))

        owner = self.report("owner", "architecture", candidate, verdict="approved")
        request["lane_determinations"] = [owner]
        invalid = recommend(request, self.root)
        self.assert_lanes(invalid, "needs_input")
        self.assertTrue(any("lane_determinations" in gap for gap in invalid["gaps"]))
        request.pop("lane_determinations")

        self.data(saved["path"], b"{}")
        damaged = recommend(request, self.root)
        self.assert_lanes(damaged, "human_decision_required")
        self.assertTrue(damaged["gaps"])
        self.assertTrue(guidance_errors(guidance, "TA-commit-batch", []))

    def test_checkpoint_collects_other_states_when_a_returned_lane_report_is_lost(self):
        br, c, cp = self.three_rounds()
        reports = {lane: self.report(f"{lane}-c", lane, c) for lane in ("behavior", "documentation")}
        head = self.round("lost-report", 3, c, cp, previous=br, reports=reports,
                          running=("tests_evidence",), gate="failed")
        result = self.root / reports["behavior"]["path"]
        options = {"before": br, "effects": ("behavior",)}
        for report_file in (result.with_suffix(".md"), result):
            with self.subTest(path=report_file.name):
                original = report_file.read_bytes()
                report_file.unlink()
                advice = self.advice(head, **options)
                self.assert_lanes(advice, "human_decision_required")
                self.assertEqual(advice["round"], 3)
                self.assertEqual(advice["candidate"], c)
                self.assertEqual(advice["approvals"], {"documentation": "documentation-c"})
                self.assertEqual(advice["executable_options"], [])
                self.assertEqual(advice["checkpoint"]["choices"], ["accept", "continue-one", "pause"])
                self.assertEqual([issue["id"] for issue in advice["issues"]],
                                 ["gate:commit:failed", "lane:behavior:missing"])
                lane_issue = advice["issues"][1]
                self.assertTrue(lane_issue["mandatory"])
                self.assertEqual(lane_issue["artifact"]["path"], f".agent/{report_file.name}")
                self.assertTrue(any(report_file.name in gap for gap in advice["gaps"]))
                report_file.write_bytes(original)

        original = result.read_bytes()
        result.write_bytes(b"{}")
        advice = self.advice(head, **options)
        self.assert_lanes(advice, "needs_input")
        self.assertNotIn("checkpoint", advice)
        self.assertIn(f"changed reference: {reports['behavior']['path']}", advice["gaps"])
        result.write_bytes(original)

    def test_human_authorization_is_single_use_and_next_round_can_restore_evidence_only(self):
        br, c, cp = self.three_rounds()
        reports = {lane: self.report(f"{lane}-c", lane, c) for lane in LANES}
        cr = self.round("c", 3, c, cp, previous=br, reports=reports, gate="failed")
        opts = {"before": br, "effects": ("evidence",)}
        initial = self.advice(cr, **opts)
        self.assert_lanes(initial, "human_decision_required")
        accepted = self.decision("accept", initial, "accept")
        self.assert_lanes(self.advice(cr, **opts, decision=accepted), "human_decision_required")
        self.assert_lanes(self.advice(cr, **opts, decision=self.decision("pause", initial, "pause")), "paused")
        coordinator = self.decision("coordinator", initial, "continue-one", role="assistant")
        self.assert_lanes(self.advice(cr, **opts, decision=coordinator), "human_decision_required")
        ambiguous = self.decision("ambiguous", initial, "continue-one", reply="go ahead")
        self.assert_lanes(self.advice(cr, **opts, decision=ambiguous), "human_decision_required")
        selected = self.decision("continue", initial, "continue-one")
        authorized = self.advice(cr, **opts, decision=selected)
        self.assert_lanes(authorized, "one_round_authorized")
        receipt = authorized["checkpoint"]["continuation"]
        saved = self.json(receipt["path"], receipt["record"])
        self.assert_lanes(self.advice(cr, **opts, decision=selected), "continuation_already_selected")
        self.assert_lanes(self.advice(cr, **opts), "continuation_already_selected")
        restored = self.round("restored", 4, c, cp, previous=cr, reports=reports, authorization=saved)
        reused = self.advice(restored, before=cr, effects=("evidence",), paths=())
        self.assert_lanes(reused, "done")
        self.assertEqual(reused["round"], 4)
        self.assertEqual(reused["approvals"], {lane: f"{lane}-c" for lane in LANES})
        impostor = self.report("docs-d", "documentation", c, context_id="documentation-c")
        collision = self.round("collision", 4, c, cp, previous=cr, authorization=saved,
                               reports={**reports, "documentation": impostor})
        rejected = self.advice(collision, before=cr, effects=("evidence",), paths=())
        self.assert_lanes(rejected, "needs_input")
        self.assertIn("independent review contexts are missing or reused", rejected["gaps"])

        dr = self.round("d", 4, c, cp, previous=cr, authorization=saved, gate="missing")
        next_step = self.advice(dr, before=cr, effects=("evidence",), paths=())
        self.assert_lanes(next_step, "human_decision_required")
        self.assertEqual(next_step["round"], 4)
        self.assertNotEqual(initial["checkpoint"]["id"], next_step["checkpoint"]["id"])
        invalid = self.round("e", 5, c, cp, previous=dr, authorization=saved)
        self.assert_lanes(self.advice(invalid, before=dr, effects=("evidence",), paths=()), "needs_input")

    def test_evidence_only_restart_retains_a_rejected_primary_claim(self):
        parent, candidate, patch = self.three_rounds()
        lane = "documentation"
        finding = self.finding("DOC-REJECTED")
        identifier = key(lane, finding["id"])
        reports = {name: self.report(f"{name}-r3", name, candidate, findings=[finding] if name == lane else [],
                                     verdict="changes_requested" if name == lane else "approved")
                   for name in LANES}
        assessments = [self.assessment(lane, finding["id"], reports[lane])]
        original_decision = {"finding": identifier, "evidence": self.req}
        third = self.round("gate-missing-r3", 3, candidate, patch, previous=parent, reports=reports, gate="missing")
        request = self.request(third, before=parent, effects=("evidence",), assessments=assessments,
                               risk_decisions=[original_decision])
        stopped = recommend(request, self.root)
        self.assert_lanes(stopped, "human_decision_required")
        decision = self.decision("restore-canonical-evidence", stopped, "continue-one")
        continuation = recommend({**request, "decision": decision}, self.root)["checkpoint"]["continuation"]
        saved = self.json(continuation["path"], continuation["record"])
        challenge = recommend(request, self.root)["challenge"]
        self.assertTrue(claim(self.root, challenge["manifest"]))
        returned = self.challenge_return("binary-reject", challenge, reports[lane], rejected=(lane,))
        blocked = recommend({**request, "challenge_report": returned}, self.root)
        resolution = {lane: {"report": reports[lane], "findings": [{"finding": identifier, "report": returned}]}}
        self.assertEqual(blocked["resolved_lanes"], resolution)
        self.assertEqual([row["id"] for row in blocked["issues"]], ["gate:commit:missing"])

        fourth = self.round("gate-restored-r4", 4, candidate, patch, previous=third, reports=reports, authorization=saved)
        options = dict(before=third, effects=("evidence",), paths=(), assessments=assessments,
                       prior_challenge_reports=[returned], risk_decisions=[original_decision])
        resumed = self.advice(fourth, **options)
        self.assert_lanes(resumed, "done")
        self.assertEqual(resumed["resolved_lanes"], resolution)
        self.assertEqual(resumed["issues"], [])
        self.assertEqual(resumed["challenge"]["state"], "skipped")

        incomplete = self.round("gate-and-lane-r4", 4, candidate, patch, previous=third, authorization=saved,
                                reports={name: ref for name, ref in reports.items() if name != "behavior"},
                                gate="failed", missing=("behavior",))
        blocked = self.advice(incomplete, **options)
        self.assert_lanes(blocked, "human_decision_required")
        self.assertEqual(blocked["resolved_lanes"], resolution)
        self.assertEqual(blocked["outstanding_lanes"], {"behavior": "missing"})
        self.assertEqual([row["id"] for row in blocked["issues"]], ["gate:commit:failed", "lane:behavior:missing"])
        self.assertEqual(blocked["challenge"], {"state": "skipped", "findings": []})

        new_fact = self.data(".agent/changed-premise", b"The earlier supported entry point now reproduces the defect.")
        changed = self.advice(fourth, **{**options, "risk_decisions": [
            original_decision, {"finding": identifier, "evidence": new_fact},
        ]})
        self.assert_lanes(changed, "human_decision_required")
        self.assertEqual(changed["resolved_lanes"], {})
        self.assertEqual(changed["challenge"], {"state": "checkpoint_pending", "findings": [identifier]})

    def test_only_accepted_deferrable_issue_allows_clean_third_round(self):
        br, c, cp = self.three_rounds()
        reports = {lane: self.report(f"{lane}-c", lane, c, findings=[self.finding("minor", severity="minor")]
                                     if lane == "behavior" else []) for lane in LANES}
        cr = self.round("c", 3, c, cp, previous=br, reports=reports)
        options = {"before": br, "effects": ("behavior",),
                   "assessments": [self.assessment("behavior", "minor", reports["behavior"], kind="deferrable")]}
        advice = self.advice(cr, **options)
        self.assert_lanes(advice, "human_decision_required")
        confirmed = self.decision("deferral", advice, "accept")
        self.assert_lanes(self.advice(cr, **options, decision=confirmed), "done")
        missing = self.round("incomplete-path", 3, c, cp, previous=None, reports=reports)
        self.assert_lanes(self.advice(missing, **options), "needs_input")
        self.data(self.criteria["path"], b"Changed criteria")
        self.assert_lanes(self.advice(cr, **options), "needs_input")

    def publish_route(self, ref):
        path = f".agent/tmp/workflow-TA/{TASK}/{ITEM}/review-routing/current.json"
        self.json(path, ref)
        return self.root / path

    def test_stale_gate_receipt_and_review_checkpoint_cannot_release_guidance(self):
        br, c, cp = self.three_rounds()
        cr = self.round("c", 3, c, cp, previous=br, gate="failed")
        request = self.request(cr, before=br, effects=("evidence",))
        ref = self.json(".agent/routing.json", request)
        self.publish_route(ref)
        gate = json.loads((self.root / cr["path"]).read_bytes())["gates"][0]["evidence"]
        guidance = {"review_route_ref": ref, "task_id": TASK, "work_item_id": ITEM, "current_candidate": c,
                    "source_result": gate, "options": [{"id": "repair", "kind": "operation",
                                                       "assignment": {"skill": "TA-fix-findings"}}]}
        errors = []
        check_review_route(guidance, json.loads((self.root / gate["path"]).read_bytes()), self.root, errors)
        self.assertEqual(errors, ["the review checkpoint or missing history suppresses operation options"])

        for state in ("failed", "passed"):
            head = self.round(state, 3, c, cp, previous=br, gate=state)
            artifact = f".agent/{state}-gate-output.txt"
            for condition in ("changed", "missing"):
                with self.subTest(state=state, condition=condition):
                    if condition == "changed":
                        self.data(artifact, b"Tampered verifier evidence.\n")
                    else:
                        (self.root / artifact).unlink()
                    advice = self.advice(head, before=br, effects=("evidence",))
                    self.assert_lanes(advice, "human_decision_required")
                    self.assertEqual(advice["round"], 3)
                    self.assertEqual(advice["candidate"], c)
                    self.assertEqual(advice["executable_options"], [])
                    self.assertEqual(advice["checkpoint"]["choices"], ["accept", "continue-one", "pause"])
                    issue, = advice["issues"]
                    self.assertEqual(issue["id"], "gate:commit:missing")
                    self.assertTrue(issue["mandatory"])
                    self.assertEqual(issue["artifact"]["path"], artifact)
                    self.assertTrue(any(artifact in gap for gap in advice["gaps"]))

        # A mismatched verification report for a late entry cannot manufacture a second issue.
        record = json.loads((self.root / cr["path"]).read_bytes())
        record["gates"].append({"id": "unrecognized", "state": "failed", "evidence": gate,
                                "impact": "This report describes the commit gate."})
        head = self.json(".agent/unrecognized.json", record)
        self.data(".agent/c-gate-output.txt", b"Tampered verifier evidence.\n")
        advice = self.advice(head, before=br, effects=("evidence",))
        self.assert_lanes(advice, "human_decision_required")
        self.assertEqual([issue["id"] for issue in advice["issues"]], ["gate:commit:missing"])
        self.assertIn("gate identity or actual result disagrees: unrecognized", advice["gaps"])

    def test_same_candidate_guidance_uses_current_route_and_its_member_reports(self):
        br, c, cp = self.three_rounds()
        docs = self.report("docs-c", "documentation", c)
        cr = self.round("focused", 3, c, cp, previous=br, reports={"documentation": docs})
        stale = self.json(".agent/focused-route.json", self.request(cr, before=br, effects=("wording",)))
        authority = self.publish_route(stale)
        matrix = self.data(MATRIX_REFERENCE, MATRIX_PATH.read_bytes())
        guidance = {
            "schema_version": 2, "kind": "guidance", "guidance_id": "fixture-guidance",
            "task_id": TASK, "work_item_id": ITEM, "source_run_id": "docs-c", "source_result": docs,
            "source_candidate": c, "current_candidate": c, "supersedes": None, "review_route_ref": stale,
            "outcome": "Focused review returned.", "consequence": "Review behavior in the remaining wave.",
            "blockers": [], "uncertainty": [], "identity_limits": [], "recommended_option_id": "next",
            "recommendation_reason": "The documentation lane approved.", "matrix_ref": matrix,
            "options": [{
                "id": "next", "kind": "operation", "availability": "ready", "missing_inputs": [],
                "label": "Review behavior", "effect": "Check the remaining behavior lane.",
                "instruction": "Review the candidate behavior.", "authorization_impact": "Select the review only.",
                "transition": {"condition": "REV", "basis": "Behavior is in the remaining wave."},
                "assignment": {"skill": SKILLS["behavior"], "executor": "TA-code-reviewer",
                               "input_refs": [self.req], "selected_findings": [], "read_scope": ["source.txt"],
                               "write_scope": [], "round_limit": 1, "previous_run_id": None,
                               "cumulative_history": [], "new_approach": None},
            }],
        }

        def errors(ref=docs):
            source = json.loads((self.root / ref["path"]).read_bytes())
            return validate_guidance({**guidance, "source_result": ref, "source_run_id": source["run_id"]},
                                     SCHEMA, self.root)

        self.assertEqual(errors(), [])
        foreign = self.report("foreign", "documentation", c, routed=False)
        with self.subTest("unrelated source"):
            self.assertIn("review routing: guidance source result is outside the current review history", errors(foreign))

        blocker = self.report("late-c", "behavior", c, findings=[self.finding("late")],
                              verdict="changes_requested", routed=False)
        stopped_round = self.round("late", 3, c, cp, previous=br,
                                   reports={"documentation": docs, "behavior": blocker})
        stopped_route = self.json(".agent/late-route.json", self.request(
            stopped_round, before=br, effects=("wording",),
            assessments=[self.assessment("behavior", "late", blocker)],
        ))
        self.publish_route(stopped_route)
        stale_error = "review guidance requires the current authoritative routing request"
        with self.subTest("same-candidate replay of earlier source"):
            self.assertIn(stale_error, errors())
        with self.subTest("new source with earlier route"):
            self.assertIn(stale_error, errors(blocker))
        guidance["review_route_ref"] = stopped_route
        with self.subTest("current route with known stop"):
            self.assertIn("the review checkpoint or missing history suppresses operation options", errors(blocker))

        guidance.pop("review_route_ref")
        with self.subTest("member source without optional routing"):
            self.assertIn(stale_error, errors(blocker))
        authority.unlink()
        with self.subTest("missing authority on resume"):
            self.assertTrue(any(error.startswith("current review routing request is unavailable:")
                                for error in errors(blocker)))

    def test_guidance_requires_the_supported_lane_for_a_shared_local_id(self):
        candidate, patch = self.candidate("shared-id", "Candidate")
        reports = {lane: self.report(lane, lane, candidate,
                                    findings=[self.finding("X", trigger=lane)] if lane in ("architecture", "behavior") else [],
                                    verdict="changes_requested" if lane in ("architecture", "behavior") else "approved")
                   for lane in LANES}
        head = self.round("shared-id", 1, candidate, patch, reports=reports)
        request = self.request(head, assessments=[
            self.assessment(lane, "X", reports[lane]) for lane in ("architecture", "behavior")
        ])
        challenge = recommend(request, self.root)["challenge"]
        self.assertTrue(claim(self.root, challenge["manifest"]))
        response = self.challenge_return(
            "challenge-local-id", challenge, reports["architecture"], rejected=("architecture",),
        )
        result = json.loads((self.root / response["path"]).read_bytes())
        source = request["challenge_report"] = response
        advice = recommend(request, self.root)
        self.assert_lanes(advice, "changes_requested")
        self.assertEqual([row["finding"] for row in advice["challenge"]["eligible_repairs"]], [key("behavior", "X")])
        self.assertEqual([row["finding"] for row in advice["challenge"]["rejected"]], [key("architecture", "X")])
        self.assertEqual(set(advice["resolved_lanes"]), {"architecture"})
        route = self.json(".agent/shared-id-route.json", request)
        self.publish_route(route)
        matrix = self.data(MATRIX_REFERENCE, MATRIX_PATH.read_bytes())

        def errors(selection, lane):
            assignment = {
                "skill": "TA-fix-findings", "executor": "TA-implementer", "input_refs": [reports[lane]],
                "selected_findings": [selection], "read_scope": ["source.txt"], "write_scope": ["source.txt"],
                "round_limit": 1, "previous_run_id": None, "cumulative_history": [], "new_approach": None,
            }
            option = {
                "id": "repair", "kind": "operation", "availability": "ready", "missing_inputs": [],
                "label": "Repair finding", "effect": "Correct the supported behavior.",
                "instruction": f"Repair {lane}/X.", "authorization_impact": "Select one evidenced repair.",
                "transition": {"condition": "FIX", "basis": "The challenge supports the selected finding."},
                "assignment": assignment,
            }
            guidance = {
                "schema_version": 2, "kind": "guidance", "guidance_id": "fixture-guidance",
                "task_id": TASK, "work_item_id": ITEM, "source_run_id": result["run_id"],
                "source_result": source, "source_candidate": candidate, "current_candidate": candidate,
                "supersedes": None, "review_route_ref": route, "outcome": "Challenge returned.",
                "consequence": "The behavior finding proceeds to repair.", "blockers": [],
                "uncertainty": [], "identity_limits": [], "recommended_option_id": "repair",
                "recommendation_reason": "The supported finding is ready for selection.", "matrix_ref": matrix,
                "options": [option],
            }
            return validate_guidance(guidance, SCHEMA, self.root)

        self.assertEqual(errors("behavior/X", "behavior"), [])
        self.assertTrue(any("lane-qualified" in error for error in errors("X", "architecture")))
        self.assertTrue(any("upheld lane-qualified" in error for error in errors("architecture/X", "architecture")))
        self.assertTrue(any("lane reports" in error for error in errors("behavior/X", "architecture")))

    def test_untracked_content_and_current_review_partition_are_bound(self):
        selected = self.data("selected.txt", b"Selected untracked input.\n")
        a, ap = self.candidate("a", "A", untracked=[selected])
        ar = self.round("a", 1, a, ap)
        self.assert_lanes(self.advice(ar), "first", *LANES)
        self.data("selected.txt", b"Changed untracked input.\n")
        self.assert_lanes(self.advice(ar), "needs_input")
        self.data("selected.txt", b"Selected untracked input.\n")
        docs = self.report("docs-a", "documentation", a)
        original = json.loads((self.root / self.profiles["documentation"]["path"]).read_bytes())
        partition = self.data(".agent/new-partition", b"A changed partition.\n")
        profile = self.json(".agent/profile-new-doc.json", {**original, "partition": partition})
        ar = self.round("changed-partition", 1, a, ap, reports={"documentation": docs},
                        profiles={**self.profiles, "documentation": profile})
        self.assert_lanes(self.advice(ar), "needs_input")

    def test_cli_reads_frozen_inputs_without_changing_git_or_artifacts(self):
        a, patch = self.candidate("a", "A")
        head = self.round("a", 1, a, patch)
        ref = self.json(".agent/request.json", self.request(head))
        status = self.git("status", "--porcelain=v1", "-z")
        before = {path.name: path.read_bytes() for path in (self.root / ".agent").iterdir()}
        result = subprocess.run([sys.executable, str(TOOLS / "review_routing.py"), ref["path"]],
                                check=True, cwd=self.root, capture_output=True)
        self.assert_lanes(json.loads(result.stdout), "first", *LANES)
        self.assertEqual(status, self.git("status", "--porcelain=v1", "-z"))
        self.assertEqual(before, {path.name: path.read_bytes() for path in (self.root / ".agent").iterdir()})


if __name__ == "__main__":
    unittest.main()
