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
from result import render
from guidance import check_review_route, validate_guidance
from review_inputs import Artifacts, SCHEMA, fingerprint
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
        alias = self.report("behavior-a", "behavior", a, findings=[self.finding("Y")], verdict="changes_requested")
        ar = self.round("a", 1, a, ap, reports={"documentation": docs, "tests_evidence": tests, "behavior": alias})
        claims = [
            self.assessment("documentation", "X", docs), self.assessment("tests_evidence", "X", tests),
            self.assessment("behavior", "Y", alias, owner="documentation", duplicate=key("documentation", "X")),
        ]
        pending = [key("behavior", "Y"), key("tests_evidence", "X")]
        b, bp = self.candidate("b", "B")
        br = self.round("b", 2, b, bp, previous=ar)
        options = {"before": ar, "effects": ("wording", "evidence"), "assessments": claims, "selected": pending}
        advice = self.advice(br, **options)
        self.assert_lanes(advice, "focused", "tests_evidence", "documentation")
        self.assertEqual(len(advice["pending"]), 2)
        self.assertEqual(len(advice["duplicate_observations"]), 1)
        self.assertEqual(advice, self.advice(br, **{**options, "assessments": claims[::-1], "selected": pending[::-1]}))
        self.assert_lanes(self.advice(br, **{**options, "selected": [key("behavior", "unknown")]}), "needs_input")
        invalid_alias = {**claims[2], "duplicate_of": key("behavior", "Y")}
        self.assert_lanes(self.advice(br, **{**options, "assessments": [*claims[:2], invalid_alias]}), "needs_input")
        stale = self.round("stale", 2, b, bp, previous=ar, reports={"documentation": docs})
        self.assert_lanes(self.advice(stale, **options), "needs_input")
        for_done = {
            "documentation": self.report("docs-b", "documentation", b,
                                         comparisons=[self.compare("documentation", docs_issue, a, b)]),
            "tests_evidence": self.report("tests-b", "tests_evidence", b,
                                          comparisons=[self.compare("tests_evidence", tests_issue, a, b)]),
        }
        br = self.round("b-verified", 2, b, bp, previous=ar, reports=for_done)
        advice = self.advice(br, **options)
        self.assert_lanes(advice, "remaining", "behavior", "architecture")
        self.assertEqual(len(advice["verified"]), 2)
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
        current_profile = self.profiles[lane]
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

        # A historical citation matching live bytes must still match its frozen candidate.
        profile("future-citation", current)
        invalid_report = self.report("docs-invalid", lane, a)
        invalid_first = self.round("future-citation", 1, a, ap, reports={lane: invalid_report})
        self.profiles[lane] = current_profile
        invalid_second = self.round("invalid-second", 2, b, bp, previous=invalid_first)
        invalid_third = self.round("invalid-third", 3, c, cp, previous=invalid_second, gate="missing")
        advice = self.advice(invalid_third, before=invalid_second, effects=("requirements",))
        self.assert_lanes(advice, "needs_input")
        self.assertIn("changed reference: source.txt", advice["gaps"])

        # A partition or evidence artifact is still a live input, even in Git scope.
        for field, value in (("partition", revised), ("evidence", [revised])):
            with self.subTest(field=field):
                self.profiles[lane] = self.json(f".agent/stale-{field}-profile.json",
                                               {**original_profile, field: value})
                stale_report = self.report(f"docs-stale-{field}", lane, b)
                stale_second = self.round(f"stale-{field}", 2, b, bp, previous=ar,
                                          reports={lane: stale_report})
                self.profiles[lane] = current_profile
                stale_third = self.round(f"stale-{field}-current", 3, c, cp, previous=stale_second,
                                         gate="missing")
                advice = self.advice(stale_third, before=stale_second, effects=("requirements",))
                self.assert_lanes(advice, "needs_input")
                self.assertIn("changed reference: source.txt", advice["gaps"])

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
        self.assert_lanes(advice, "changes_requested")
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
