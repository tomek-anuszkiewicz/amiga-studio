"""Delivery decisions with independent Git worktrees and local push destinations."""

from hashlib import sha256
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import delivery
import execution
import integration
import verify
from case_report import CONTEXT
from evidence import digest, file_digest
from integration_cleanup import cleanup, cleanup_sources
from integration_closure import capture_binding, closure_record, finish_preservation
from result import render
from fixture_repository import baseline, empty_repository, restore


class DeliveryTests(unittest.TestCase):
    def setUp(self):
        temporary = TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.parent = Path(temporary.name).resolve()
        self.root = self.parent / "primary"
        self.base = baseline("delivery", self.parent)
        self.next_run = 0

    @staticmethod
    def git(root, *args):
        return subprocess.run(
            ["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
             "-c", "core.autocrlf=false", *map(str, args)], cwd=root, capture_output=True,
            check=True).stdout.decode("utf-8").strip()

    def tip(self, root, ref="HEAD"):
        return self.git(root, "rev-parse", ref)

    def commit(self, root, message):
        self.git(root, "commit", "--no-verify", "-m", message)

    @staticmethod
    def write(root, name, content):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    def worktree(self, task, *, shared_channel=False):
        location = self.parent / ("primary-" + task)
        restore("source-" + task, self.parent)
        if shared_channel:
            restore("channel-" + task, self.parent)
        return location

    def registration(self, task, execution="execution"):
        relative = f".agent/task-archives/{task}/{execution}/registration.json"
        path = self.root / relative
        if path.exists():
            return path
        return self.write(self.root, relative, json.dumps({
            "schema_version": 1, "task_id": task, "execution_id": execution,
            "git_common_dir": str(self.root / ".git"), "root_thread_id": "38e0b2b3-5101-4730-a1e0-6ea70126e200",
            "start_utc": "2026-11-19T10:00:00Z",
        }))

    def evidence(self, location, task, *, revision="1", status="completed", capture_cli=False):
        plan = f".agent/tasks/{task}.md"
        self.write(location, plan, f"# Fixture plan\n\n**Task ID** `{task}`\n**Revision** {revision}\n")
        self.next_run += 1
        run = f"run-{self.next_run}"
        directory = delivery.WORKFLOW / task / "finish" / run
        source = (directory / "source.json").as_posix()
        registration = self.registration(task)
        if capture_cli:
            process = subprocess.run(
                [sys.executable, "-B", str(Path(delivery.__file__)), "snapshot", "--plan", plan, "--registration",
                 str(registration), "--output", source], cwd=location, capture_output=True, encoding="utf-8")
            self.assertEqual(process.returncode, 0, process.stderr)
        else:
            self.write(location, source, json.dumps(delivery.capture_source(location, plan, registration)))
        patch = subprocess.run(
            ["git", "diff", "--binary", self.base, "HEAD", "--", task + ".txt"], cwd=location,
            capture_output=True, check=True).stdout
        result = {
            "schema_version": 2, "task_id": task, "work_item_id": "finish", "run_id": run,
            "previous_run_id": None, "agent": "TA-implementer", "skill": "TA-implement-change",
            "inputs": {"plans": [delivery.reference(location, plan)], "prior_results": [], "instructions": [],
                       "candidate": {"base_commit": self.base, "diff_sha256": sha256(patch).hexdigest(),
                                     "scope": [task + ".txt"], "untracked": []},
                       "configuration_ref": None, "identity_limits": []},
            "status": status, "rounds": {"used": 1, "limit": 3, "stop_reason": "Fixture completion"},
            "summary": "Implemented selected change",
            "result": {"kind": "implementation", "changes": ["Write fixture source"],
                       "selected_finding_ids": [], "remaining_work": []},
            "findings": [], "evidence": [
                {"id": "delivery-source", "origin": "executed", "command": "delivery.py snapshot", "exit_code": 0,
                 "artifact_ref": delivery.reference(location, source), "candidate_ref": None,
                 "validity_basis": "Source snapshot when finishing implementation"}],
            "changed_paths": [task + ".txt"],
            "observations": {"started_at": None, "finished_at": None, "duration_seconds": None,
                             "model": None, "configuration": None, "limits": []},
            "recommendation": "Select integration",
        }
        report = (directory / "result").as_posix()
        self.write(location, report + ".json", json.dumps(result))
        self.write(location, report + ".md", render(result))
        return plan, report + ".json"

    def declare(self, location, task, *, revision="1", deps=None, working=False, remaining=("verification", "review")):
        plan, result = self.evidence(location, task, revision=revision)
        return delivery.declare(location, plan, result, "All implementation blockers resolved.",
                                remaining, deps or {}, working)

    def selected(self, *tasks, **options):
        return delivery.select(self.root, delivery.offer(self.root), tasks, "Selected by fixture", **options)

    def confirmation(self, location):
        tip, tree = self.git(location, "rev-parse", "HEAD", "HEAD^{tree}").splitlines()
        return self.write(location, ".agent/tmp/confirmation.json", json.dumps({
            "base": self.base, "commit": tip, "tree": tree,
            "plan_approved": True, "tested": True, "confirmation": "User tested this exact commit.",
        }))

    def frozen(self, location, selection, *, canonical=True):
        tip, tree = self.git(location, "rev-parse", "HEAD", "HEAD^{tree}").splitlines()
        return {"version": 2, "state": "sealed", "base": selection["base"],
                "target_root": str(self.root), "target_ref": delivery.MASTER,
                "source_root": str(location), "source_ref": self.git(location, "symbolic-ref", "HEAD"),
                "commit": tip, "tree": tree,
                "sources": selection["sources"], "sources_explicit": selection["assembly"] is not None,
                "verification": "canonical" if canonical else "confirmed-fast-forward"}

    def publish_fixture(self, location, selection):
        frozen = self.frozen(location, selection)
        if selection["assembly"]:
            frozen["evidence"], _ = self.verify_candidate(location, "assembly", ["first.txt", "second.txt"])
            frozen["capture"] = capture_binding(self.root, "assembly", "execution")
        selected_path = self.write(self.root, ".agent/tmp/selection.json", json.dumps(selection))
        frozen["delivery"] = delivery.bind_publication(location, selected_path, frozen)
        # The native publisher's own fixtures exercise making this transition
        # after verification and sealing. Supply that state for push and discovery.
        self.git(self.root, "merge", "--ff-only", frozen["commit"])
        frozen["state"] = "published"
        record = delivery.cleanup_record(frozen)
        delivery.save(record, frozen)
        return record, frozen

    def verify_candidate(self, location, task, paths, *, check=None, expected_exit=0, diagnostic=None):
        from run_selected import selected_environment
        registration = self.registration(task)
        inherited = {key: value for key, value in os.environ.items() if key not in CONTEXT.values()}
        environment = selected_environment(registration, location, inherited, "verification", 1)
        if check is None:
            check = "from pathlib import Path; import sys; assert all(Path(name).is_file() for name in sys.argv[1:])"
        phases = [("Preflight", [("Fixture check", [sys.executable, "-c", check, *paths], {})])]
        with (patch.dict(os.environ, environment, clear=True), patch.object(verify, "REPO_ROOT", location),
              patch.object(execution, "REPO_ROOT", location),
              patch.object(verify, "select_phases", return_value=phases),
              patch.object(verify, "execution_conditions", return_value="fixture"),
              patch.object(verify, "stable_update_notice", return_value=""),
              redirect_stdout(io.StringIO()) as output):
            result = verify.main(["commit", "--integration-base", self.base])
        self.assertEqual(result, expected_exit, output.getvalue())
        if diagnostic:
            self.assertIn(diagnostic, output.getvalue())
        evidence = delivery.read(location / ".agent/tmp/verification/commit/commit.json")
        return evidence, phases

    def native_assembly_publication(self, location, selection, manifest, selection_path, check):
        evidence, phases = self.verify_candidate(location, "assembly", ["first.txt", "second.txt"], check=check)
        partition = self.write(location, ".agent/tmp/partition.json", "[]")
        review = self.write(location, ".agent/tmp/review.json", json.dumps({
            "verdict": "APPROVED", "reviewer": "Independent fixture reviewer", "source": evidence["source"],
            "integration_base": self.base, "partition": digest([]), "sources": digest(selection["sources"]),
        }))
        def passing_english(_, frozen, record_digest):
            directory = delivery.cleanup_record(frozen).parent
            log = self.write(directory, "english-" + "0" * 32 + ".log", "Passing fixture scan\n")
            return {"state": "passed", "commit": frozen["commit"], "tree": frozen["tree"],
                    "sealed_sha256": record_digest, "exit_code": 0, "scanner_exit_code": 0,
                    "log": log.name, "log_sha256": file_digest(log)}

        with (patch.object(integration, "execution_conditions", return_value="fixture"),
              patch("commands.select_phases", return_value=phases),
              patch.object(integration, "require_english", side_effect=passing_english)):
            record = integration.freeze(location, location / ".agent/tmp/verification/commit",
                                        review, partition, "master", manifest)
            integration.seal(location, record)
            with self.assertRaisesRegex(ValueError, "requires the selected assembly task and execution"):
                integration.publish(location, record, delivery=selection_path)
            with self.assertRaisesRegex(ValueError, "does not identify the selected assembly attempt"):
                integration.publish(location, record, task_id="first", execution_id="execution",
                                    delivery=selection_path)
            self.assertEqual(self.tip(self.root), self.base)
            integration.publish(location, record, task_id="assembly", execution_id="execution",
                                delivery=selection_path)
        frozen = delivery.read(record)
        return delivery.cleanup_record(frozen), frozen

    def test_only_current_unambiguous_completed_readiness_is_offered(self):
        ready = self.worktree("ready")
        self.declare(ready, "ready")
        self.write(ready, ".agent/cache.txt", "Ignored local state\n")
        draft = self.worktree("draft")
        plan, result = self.evidence(draft, "draft", status="needs_input")
        with self.assertRaisesRegex(ValueError, "not completed"):
            delivery.declare(draft, plan, result, "Waiting for input", ("verification",), {}, False)
        paused = self.worktree("paused")
        self.declare(paused, "paused")
        delivery.withdraw(paused, "paused", "Implementation paused.")
        dirty = self.worktree("dirty")
        self.declare(dirty, "dirty")
        self.write(dirty, "extra.txt", "Unexpected source\n")
        moved = self.worktree("moved")
        self.declare(moved, "moved")
        self.write(moved, "moved.txt", "New tip\n")
        self.git(moved, "add", "moved.txt")
        self.commit(moved, "Advance source")
        stale = self.worktree("stale")
        self.declare(stale, "stale")
        self.evidence(stale, "stale", revision="2")
        amended = self.worktree("amended")
        self.declare(amended, "amended")
        self.write(amended, ".agent/tasks/amended.amendments.md", "Pending decision.\n")

        offer = delivery.offer(self.root)
        self.assertEqual([plan["task_id"] for plan in offer["plans"]], ["ready"])
        self.assertEqual(offer["route_choice"], "single")
        self.assertNotIn("draft", offer["excluded"])
        for task, reason in (("paused", "paused"), ("dirty", "pending changes"), ("moved", "tip changed"),
                             ("stale", "plan or revision changed"), ("amended", "amendment record changed")):
            self.assertIn(reason, offer["excluded"][task])
        self.git(dirty, "add", "extra.txt")
        self.assertIn("pending changes", delivery.offer(self.root)["excluded"]["dirty"])

    def test_duplicate_ownership_excludes_both_checkouts(self):
        source = self.worktree("original")
        self.declare(source, "original")
        duplicate = self.worktree("other")
        original_claim = source / delivery.readiness_path("original")
        self.write(duplicate, delivery.readiness_path("original"), original_claim.read_text(encoding="utf-8"))
        offer = delivery.offer(self.root)
        self.assertEqual(offer["plans"], [])
        self.assertIn("Several registered checkouts", offer["excluded"]["original"])

    def test_declaring_again_requires_the_original_completed_tip_and_tree(self):
        for task, update in (("changed", "change"), ("reverted", "revert"), ("same-tree", "empty")):
            with self.subTest(task=task):
                source = self.worktree(task)
                plan, report = self.evidence(source, task)
                original = self.tip(source)
                if update == "change":
                    self.write(source, f"{task}.txt", "Changed completed work.\n")
                    self.git(source, "add", f"{task}.txt")
                    self.commit(source, "Modify implementation")
                elif update == "revert":
                    self.git(source, "revert", "--no-edit", original)
                    self.assertEqual(self.tip(source, "HEAD^{tree}"), self.tip(self.root, "HEAD^{tree}"))
                else:
                    self.git(source, "commit", "--no-verify", "--allow-empty", "-m", "New tip, same tree")
                    self.assertEqual(self.tip(source, "HEAD^{tree}"), self.tip(source, original + "^{tree}"))
                with self.assertRaisesRegex(ValueError, "tip or tree differs"):
                    delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, False)
                self.assertNotIn(task, [item["task_id"] for item in delivery.offer(self.root)["plans"]])

    def test_dirty_completion_transfers_once_to_an_exact_committed_candidate(self):
        source = self.worktree("working")
        self.write(source, "working.txt", "Ready implementation.\n")
        self.write(source, "new.txt", "New implementation input.\n")
        self.git(source, "add", "working.txt")
        index = subprocess.check_output(["git", "diff", "--cached", "--binary"], cwd=source)
        plan, report = self.evidence(source, "working", capture_cli=True)
        self.assertEqual(subprocess.check_output(["git", "diff", "--cached", "--binary"], cwd=source), index)
        self.assertIn("?? new.txt", self.git(source, "status", "--short"))
        with self.assertRaisesRegex(ValueError, "pending changes"):
            delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, True)
        self.git(source, "add", "new.txt")
        self.commit(source, "Complete working implementation")
        ready = delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, True)
        self.assertEqual(delivery.read(ready)["source"]["tip"], self.tip(source))
        self.assertEqual(self.selected("working")["route"], "verify")
        self.git(source, "commit", "--no-verify", "--allow-empty", "-m", "Advance working source")
        with self.assertRaisesRegex(ValueError, "tip or tree differs"):
            delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, True)

    def test_pending_amendment_already_captured_still_blocks_readiness(self):
        source = self.worktree("amended")
        amendment = ".agent/tasks/amended.amendments.md"
        self.write(source, amendment, "# Decisions\n\n**Disposition.** Pending a user decision.\n")
        plan, report = self.evidence(source, "amended")
        with self.assertRaisesRegex(ValueError, "pending or unconfirmed disposition"):
            delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, False)
        self.assertEqual(delivery.offer(self.root)["plans"], [])
        self.write(source, amendment, "# Decisions\n\n**Disposition.** Declined by the user.\n")
        with self.assertRaisesRegex(ValueError, "different plan inputs"):
            delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, False)
        self.declare(source, "amended")
        self.assertEqual(self.selected("amended")["plans"][0]["task_id"], "amended")

    def test_later_named_decisions_resolve_amendments_and_keep_the_history(self):
        source = self.worktree("amended")
        path = ".agent/tasks/amended.amendments.md"
        history = ("# Amendment history\r\n\r\n"
                   "## D-03 — Return value for zero\r\n\r\n**Disposition.** Pending a decision.\r\n\r\n"
                   "## Restart checkpoint\r\n\r\nD-03 remains pending.\r\n\r\n"
                   "## D-04 — Negative input\r\n\r\n**Disposition.** Pending clarification.\r\n\r\n")
        checkpoint = ("## Revision 2 checkpoint\r\n\r\n"
                      "- **D-03 Approved.** Incorporated in the revised plan.\r\n"
                      "- **D-04 declined for this delivery.** Preserve and exclude the proposed edit.\r\n")
        for note in (history, history + checkpoint + "- **D-03 declined.**\r\n",
                     history + checkpoint + "\r\n## D-05",
                     history + checkpoint.replace("D-04 declined", "D-05 declined")):
            self.write(source, path, note.replace("\r\n", "\n"))
            plan, report = self.evidence(source, "amended", revision="2")
            with self.assertRaisesRegex(ValueError, "pending or unconfirmed disposition"):
                delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, False)
        amendment = source / path
        retained = (history + checkpoint).encode()
        amendment.write_bytes(retained)
        plan, report = self.evidence(source, "amended", revision="2")
        declaration = delivery.declare(source, plan, report, "All blockers resolved.", ("verification",), {}, False)
        self.assertEqual(delivery.read(declaration)["amendments"], delivery.reference(source, path))
        self.assertEqual(self.selected("amended")["plans"][0]["revision"], "2")
        self.assertEqual(amendment.read_bytes(), retained)

        self.write(source, path, ("# Amendment history\n\n"
                                 "## First scope change\n\n**Status** Approved by the user.\n\n"
                                 "## Other scope change\n\n**Disposition.** Declined by the user.\n"))
        self.declare(source, "amended", revision="2")
        self.assertEqual(self.selected("amended")["plans"][0]["revision"], "2")

    def test_choice_retains_named_exact_tips_and_dependency_order(self):
        top = self.worktree("top")
        foundation = self.worktree("foundation")
        self.declare(foundation, "foundation", revision="2")
        self.declare(top, "top", deps={"foundation": "2"})
        offer = delivery.offer(self.root)
        self.assertEqual(offer["route_choice"], "combined-or-single")
        self.assertEqual(offer["base"], self.base)
        self.assertEqual([plan["task_id"] for plan in offer["plans"]], ["foundation", "top"])
        self.assertEqual([plan["source"]["tip"] for plan in offer["plans"]], [self.tip(foundation), self.tip(top)])
        single = delivery.select(self.root, offer, ("foundation",), "Declined combined delivery")
        self.assertEqual(single["route"], "verify")
        with self.assertRaisesRegex(ValueError, "needs selected or published"):
            delivery.select(self.root, offer, ("top",), "Choose dependent only")
        self.write(foundation, "foundation.txt", "Different choice\n")
        self.git(foundation, "add", "foundation.txt")
        self.commit(foundation, "Advance offered source")
        self.declare(foundation, "foundation", revision="2")
        with self.assertRaisesRegex(ValueError, "changed since the offer"):
            delivery.select(self.root, offer, ("foundation",), "Accept previous offer")

    def test_working_unconfirmed_closure_and_divergent_tips_require_full_route(self):
        location = self.worktree("task")
        plan, result = self.evidence(location, "task")

        def declare(remaining=("verification", "review"), *, working=False):
            return delivery.declare(location, plan, result, "All implementation blockers resolved.",
                                    remaining, {}, working)

        declare()
        confirmation = self.confirmation(location)
        self.assertEqual(self.selected("task")["route"], "verify")
        self.assertEqual(self.selected("task", confirmation=confirmation)["route"], "tested")
        declare(working=True)
        self.assertEqual(self.selected("task", confirmation=confirmation)["route"], "verify")
        declare(remaining=("closure-content", "verification"))
        self.assertEqual(self.selected("task", confirmation=confirmation)["route"], "verify")
        old = self.selected("task")
        self.write(self.root, "target.txt", "Master advances independently\n")
        self.git(self.root, "add", "target.txt")
        self.commit(self.root, "Advance master")
        self.assertEqual(self.selected("task")["route"], "merge")
        with self.assertRaisesRegex(ValueError, "Master changed"):
            delivery.revalidate(self.root, old)

    def test_separate_assembly_uses_original_sources_and_scoped_cleanup(self):
        first = self.worktree("first", shared_channel=True)
        second = self.worktree("second", shared_channel=True)
        paths = ["first.txt", "second.txt"]
        conflict = "Two clients use the same exclusive channel"
        check = ("from pathlib import Path\n"
                 "import sys\n"
                 "channels = [path.read_text().strip() for path in map(Path, sys.argv[1:]) if path.is_file()]\n"
                 "assert channels, 'At least one client must be configured'\n"
                 f"assert len(channels) == len(set(channels)), {conflict!r}\n")
        self.declare(first, "first")
        self.declare(second, "second", deps={"first": "1"})
        independent = {}
        for location, task in ((first, "first"), (second, "second")):
            record, _ = self.verify_candidate(location, task, paths, check=check)
            journal = (self.root / record["case_report"]["path"]).with_suffix(".jsonl")
            independent[task] = (journal, journal.read_bytes())
        script = [sys.executable, "-B", str(Path(delivery.__file__))]
        offer = self.root / ".agent/tmp/offer.json"
        selection = self.root / ".agent/tmp/selection.json"
        for arguments in (["discover", "--output", str(offer)],
                          ["select", "--offer", str(offer), "--task", "second", "--task", "first",
                           "--answer", "Proceed together", "--assembly-branch", "assembly",
                           "--cleanup", "refs/heads/second", "--output", str(selection)]):
            process = subprocess.run([*script, *arguments], cwd=self.root, check=True, capture_output=True)
            self.assertIn(b'"plans"', process.stdout)
        selected = delivery.read(selection)
        self.assertEqual(delivery.revalidate(self.root, selected), selected)
        assembly = Path(selected["assembly"]["root"])
        self.git(self.root, "worktree", "add", "-b", "assembly", assembly, selected["base"])
        self.assertEqual(delivery.revalidate(self.root, selected, provisioned=True), selected)
        manifest = assembly / ".agent/tmp/sources.json"
        subprocess.run([*script, "revalidate", str(selection), "--provisioned", "--sources-output", str(manifest)],
                       cwd=self.root, check=True, capture_output=True)
        self.assertEqual(delivery.read(manifest), selected["sources"])
        self.assertEqual(self.tip(assembly), self.base)
        for source in selected["sources"][1:]:
            self.git(assembly, "merge", "--no-ff", "--no-edit", source["tip"])
        self.assertEqual(self.git(assembly, "rev-list", "--count", "--merges", self.base + "..HEAD"), "2")
        self.assertEqual((self.tip(first), self.tip(second)), tuple(item["tip"] for item in selected["sources"][1:]))
        self.assertEqual(self.git(first, "status", "--porcelain"), "")
        self.assertEqual(self.git(second, "status", "--porcelain"), "")
        failed, _ = self.verify_candidate(assembly, "assembly", paths, check=check,
                                          expected_exit=1, diagnostic=conflict)
        self.assertEqual(failed["outcome"], "failed")
        self.assertEqual(failed["groups"]["Fixture check"]["status"], "failed")
        self.assertEqual(failed["head"], self.tip(assembly))
        before = {"master": self.tip(self.root), "first": self.tip(first), "second": self.tip(second)}
        directory = assembly / ".agent/tmp/verification/commit"
        frozen_path = directory / "integration.json"
        with self.assertRaisesRegex(ValueError, "Passing canonical commit evidence"):
            integration.freeze(assembly, directory, directory / "review.json",
                               directory / "partition.json", "master", manifest)
        self.assertFalse(frozen_path.exists())
        with patch.object(verify, "REPO_ROOT", assembly), redirect_stdout(io.StringIO()) as output:
            refusal = verify.main(["integrate", "publish", "record", str(frozen_path), "task", "assembly",
                                   "execution", "execution", "delivery", str(selection)])
        self.assertEqual(refusal, 1, output.getvalue())
        blocked = {"master": self.tip(self.root), "first": self.tip(first), "second": self.tip(second)}
        self.assertEqual(blocked, before)
        self.assertEqual(blocked["master"], selected["base"])
        self.assertFalse(list((self.root / delivery.INTEGRATIONS).glob("*/integration.json")))
        self.write(assembly, ".agent/tmp/failed-assembly.json", json.dumps({
            "canonical": failed, "publication_exit": refusal, "publication_output": output.getvalue(),
            "refs_before": before, "refs_after": blocked,
        }))

        self.write(assembly, "second.txt", "secondary\n")
        self.git(assembly, "add", "second.txt")
        self.commit(assembly, "Assign a separate channel to the second client")
        record, frozen = self.native_assembly_publication(assembly, selected, manifest, selection, check)
        self.assertNotEqual(frozen["evidence"]["attempt"], failed["attempt"])
        self.assertNotEqual(frozen["tree"], failed["tree"])
        self.assertEqual(cleanup_sources(frozen), [selected["sources"][2]])
        process = subprocess.run([*script, "status", str(record)], cwd=self.root, check=True, capture_output=True)
        report = json.loads(process.stdout)
        self.assertEqual([(plan["task_id"], plan["original_tip"], plan["cleanup"]) for plan in report["plans"]],
                         [("first", self.tip(first), "unselected"), ("second", self.tip(second), "pending")])
        self.assertEqual(report["assembly_cleanup"], "unselected")
        provenance = report["assembly_verification"]
        self.assertEqual(provenance["execution"]["task_id"], "assembly")
        self.assertEqual(provenance["sources"], [item["source_execution"] for item in report["plans"]])
        self.assertEqual([item["task_id"] for item in provenance["sources"]], ["first", "second"])
        self.assertEqual(delivery.reference(self.root, provenance["journal"]["path"]), provenance["journal"])
        assembly_journal = (self.root / provenance["journal"]["path"]).read_bytes()
        self.assertIn(provenance["attempt"].encode(), assembly_journal)
        for task, (journal, unchanged) in independent.items():
            self.assertEqual(journal.read_bytes(), unchanged)
            header = json.loads(unchanged.splitlines()[0])
            self.assertEqual((header["task_id"], header["execution_id"]), (task, "execution"))
            self.assertNotEqual(header["invocation_id"], provenance["case_report"]["invocation_id"])
        offer = delivery.offer(self.root)
        self.assertEqual(offer["plans"], [])
        self.assertIn("already records this plan", offer["excluded"]["second"])

    def test_incompatible_dependencies_and_changed_source_block_assembly(self):
        first = self.worktree("first")
        second = self.worktree("second")
        self.declare(first, "first")
        plan, result = self.evidence(second, "second")
        delivery.declare(second, plan, result, "All implementation blockers resolved.",
                         ("verification", "review"), {"first": "2"}, False)
        self.assertIn("revision 2", delivery.offer(self.root)["excluded"]["second"])
        delivery.declare(second, plan, result, "All implementation blockers resolved.",
                         ("verification", "review"), {"first": "1"}, False)
        selected = self.selected("first", "second", assembly_branch="assembly")
        path = self.write(second, "second.txt", "Pending source edit\n")
        with self.assertRaisesRegex(ValueError, "Selected source changed: second"):
            delivery.revalidate(self.root, selected)
        path.write_text("second\n", encoding="utf-8")
        assembly = Path(selected["assembly"]["root"])
        self.git(self.root, "worktree", "add", "-b", "assembly", assembly, self.base)
        self.git(assembly, "merge", "--no-ff", "--no-edit", self.tip(first))
        selected_path = self.write(self.root, ".agent/tmp/selection.json", json.dumps(selected))
        with self.assertRaisesRegex(ValueError, "missing from the accepted commit"):
            delivery.bind_publication(assembly, selected_path, self.frozen(assembly, selected))
        self.assertEqual(self.tip(self.root), self.base)

    def test_source_preservation_resumes_and_cleanup_retains_unselected_checkout(self):
        first = self.worktree("first")
        second = self.worktree("second")
        self.declare(first, "first")
        self.declare(second, "second")
        selected = self.selected("first", "second", assembly_branch="assembly",
                                 cleanup=("refs/heads/first", "refs/heads/second"))
        assembly = Path(selected["assembly"]["root"])
        self.git(self.root, "worktree", "add", "-b", "assembly", assembly, self.base)
        for source in selected["sources"][1:]:
            self.git(assembly, "merge", "--no-ff", "--no-edit", source["tip"])
        record, frozen = self.publish_fixture(assembly, selected)
        archive = self.root / ".agent/task-archives/assembly/execution"
        self.write(archive, "registration.json", json.dumps({
            "schema_version": 1, "task_id": "assembly", "execution_id": "execution",
            "git_common_dir": str(self.root / ".git"), "root_thread_id": "38e0b2b3-5101-4730-a1e0-6ea70126e200",
            "start_utc": "2026-11-19T10:00:00Z",
        }))
        frozen["capture"] = capture_binding(self.root, "assembly", "execution")
        delivery.save(record, frozen)
        path, closure = closure_record(frozen)
        closure.update(language={"state": "complete", "final_commit": frozen["commit"]},
                       graph={"commit": frozen["commit"]})
        delivery.save(path, closure)
        from preservation import SelectedFile, copy_selected
        files = {}
        pending_id = None
        for task, inputs in delivery.preservation_inputs(frozen).items():
            for number, source in enumerate(inputs):
                key = f"{task}-{number}"
                complete = task == "first" or number > 0
                if not complete:
                    pending_id = key
                files[key] = SelectedFile(Path(source), PurePosixPath(task) / Path(source).name, complete)
        self.assertIsNotNone(pending_id)

        def preserve(items):
            finish_preservation(self.root, record, "assembly", "execution",
                                lambda state, progress: copy_selected(self.root, archive, items, state, progress),
                                selected=items)

        with self.assertRaisesRegex(ValueError, "exact known inputs"):
            preserve({key: item for key, item in files.items() if key != pending_id})
        with self.assertRaisesRegex(ValueError, "producer is still active"):
            preserve(files)
        self.assertEqual([plan["preservation"]["state"] for plan in delivery.status(self.root, record)["plans"]],
                         ["preserved", "pending"])
        pending = files[pending_id]
        files[pending_id] = SelectedFile(pending.source, pending.destination, True)
        preserve(files)
        _, retained = closure_record(frozen)
        self.assertEqual(set(retained["preservation"]["plans"]), {"first", "second"})
        self.assertEqual([plan["preservation"]["state"] for plan in delivery.status(self.root, record)["plans"]],
                         ["preserved", "preserved"])

        self.write(self.root, ".git/info/exclude", "/tools/\n")
        worktree_tool = Path(__file__).resolve().parents[2] / "worktree.ps1"
        self.write(self.root, "tools/worktree.ps1", worktree_tool.read_text(encoding="utf-8"))
        self.write(second, ".agent/tasks/second.amendments.md", "Later plan work.\n")
        with self.assertRaisesRegex(ValueError, "amendment record changed"):
            cleanup(self.root, record)
        self.assertFalse(first.exists())
        self.assertTrue(second.exists())
        self.assertTrue(assembly.exists())
        report = delivery.status(self.root, record)
        self.assertEqual(report["plans"][0]["cleanup"]["branch"], "removed")
        self.assertEqual(report["plans"][1]["cleanup"]["branch"], "present")
        self.assertEqual(report["assembly_cleanup"], "unselected")
        self.assertEqual(self.tip(self.root), frozen["commit"])

    def test_local_only_and_ordinary_push_retries_use_only_authorized_master(self):
        location = self.worktree("task")
        self.declare(location, "task")
        remote = self.parent / "destination.git"
        other = self.parent / "other.git"
        empty_repository("bare", remote, bare=True)
        empty_repository("bare", other, bare=True)
        self.git(self.root, "remote", "add", "destination", remote)
        self.git(self.root, "remote", "add", "other", other)
        selected = self.selected("task", remote="destination")
        self.git(location, "tag", "-a", "fixture-tag", "-m", "Fixture tag")
        self.git(self.root, "config", "push.followTags", "true")
        record, frozen = self.publish_fixture(location, selected)
        local_only = {**frozen, "delivery": {**selected, "push": None}}
        delivery.save(record, local_only)
        with self.assertRaisesRegex(ValueError, "grants no remote push"):
            delivery.push(self.root, record)
        self.assertEqual(self.git(remote, "for-each-ref"), "")
        delivery.save(record, frozen)
        result = delivery.push(self.root, record)
        self.assertEqual(result["state"], "pushed")
        self.assertEqual(self.tip(remote, delivery.MASTER), frozen["commit"])
        self.assertEqual(self.git(remote, "show-ref"), frozen["commit"] + " " + delivery.MASTER)
        self.assertEqual(self.git(other, "for-each-ref"), "")
        self.assertEqual(delivery.push(self.root, record), result)
        self.assertEqual(delivery.status(self.root, record)["push"], result)

    def test_remote_rejection_and_changed_destination_leave_local_publication(self):
        location = self.worktree("task")
        self.declare(location, "task")
        remote = self.parent / "destination.git"
        empty_repository("bare", remote, bare=True)
        self.git(self.root, "remote", "add", "destination", remote)
        different = self.worktree("different")
        self.git(different, "push", "--no-verify", str(remote), "HEAD:" + delivery.MASTER)
        intervening = self.tip(remote, delivery.MASTER)
        record, frozen = self.publish_fixture(location, self.selected("task", remote="destination"))
        with self.assertRaisesRegex(ValueError, "Push rejected"):
            delivery.push(self.root, record)
        self.assertEqual(self.tip(remote, delivery.MASTER), intervening)
        self.assertEqual(self.tip(self.root), frozen["commit"])
        self.assertEqual(delivery.status(self.root, record)["push"]["state"], "rejected")
        self.git(self.root, "remote", "set-url", "destination", self.parent / "other.git")
        with self.assertRaisesRegex(ValueError, "destination changed"):
            delivery.push(self.root, record)
        self.assertEqual(self.tip(self.root), frozen["commit"])


if __name__ == "__main__":
    unittest.main()
