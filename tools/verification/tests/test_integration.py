"""Publication and recovery contracts against real Git worktrees and processes."""
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

import psutil

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import commands
import execution
import integration
import integration_cleanup
import integration_state
import repository
import verify
from cli import parse_args
from evidence import digest


class IntegrationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.target = self.parent / "primary"
        self.target.mkdir()
        self.source = self.parent / "primary-task"
        self.write(self.target, ".gitignore", ".agent/\n.env\nignored*\ntarget/\n")
        self.write(self.target, "value.txt", "baseline\n")
        tool = Path(__file__).resolve().parents[2] / "worktree.ps1"
        self.write(self.target, "tools/worktree.ps1", tool.read_text(encoding="utf-8"))
        self.git(self.target, "init", "-q", "-b", "master")
        self.base = self.commit(self.target)
        self.git(self.target, "worktree", "add", "-q", "-b", "task", str(self.source), self.base)
        self.directory = self.source / ".agent/tmp/verification/commit"
        self.record = self.directory / "integration.json"
        self.native_selection = commands.select_phases
        self.enterContext(patch.object(integration, "execution_conditions", return_value="fixture"))
        self.enterContext(patch.object(verify, "execution_conditions", return_value="fixture"))
        self.phases = [("Preflight", [("Fixture check", [sys.executable, "-c", "print('verified fixture')"], {})])]
        self.enterContext(patch.object(commands, "select_phases", return_value=self.phases))
        self.enterContext(patch.object(verify, "select_phases", return_value=self.phases))

    def write(self, root, name, content):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    def git(self, root, *args):
        return repository.git(root, *args).decode("utf-8").strip()

    def commit(self, root, message="fixture", parents=None):
        self.git(root, "add", "-A")
        tree = self.git(root, "write-tree")
        old = subprocess.run(["git", "rev-parse", "--verify", "HEAD"], cwd=root, capture_output=True)
        if parents is None:
            parents = [old.stdout.decode().strip()] if old.returncode == 0 else []
        args = ["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit-tree", tree, "-m", message]
        for parent in parents:
            args += ["-p", parent]
        revision = self.git(root, *args)
        self.git(root, "update-ref", "HEAD", revision)
        return revision

    def prepare(self, pending=True, message="feat: candidate", original_source=None, sources=None):
        if pending:
            self.write(self.source, "value.txt", "candidate\n")
        with patch.object(verify, "REPO_ROOT", self.source), patch.object(execution, "REPO_ROOT", self.source), \
             patch.object(verify, "stable_update_notice", return_value=""), contextlib.redirect_stdout(io.StringIO()) as output:
            sentence = ["commit", "--integration-base", self.base]
            if original_source:
                sentence += ["--integration-source", original_source]
            result = verify.main(sentence)
        self.assertEqual(result, 0, output.getvalue())
        evidence = integration.read(self.directory / "commit.json")
        partition = [{"message": "feat: candidate", "tree": evidence["tree"]}] if repository.collect_changes(self.source) else []
        self.write(self.source, ".agent/tmp/partition.json", json.dumps(partition))
        review = {"verdict": "APPROVED", "reviewer": "independent fixture reviewer", "source": evidence["source"],
                  "integration_base": self.base, "partition": digest(partition)}
        sources_path = None
        if sources is not None:
            sources_path = self.write(self.source, ".agent/tmp/sources.json", json.dumps(sources))
            review["sources"] = digest(sources)
        self.write(self.source, ".agent/tmp/review.json", json.dumps(review))
        integration.freeze(self.source, self.directory, self.source / ".agent/tmp/review.json",
                           self.source / ".agent/tmp/partition.json", "master", sources_path)
        if partition:
            self.commit(self.source, message)
        integration.seal(self.source, self.record)
        return self.git(self.source, "rev-parse", "HEAD")

    def assemble_sources(self):
        self.write(self.source, "task.txt", "First task\n")
        first = self.commit(self.source, "feat: first task")
        sources = [{"ref": "refs/heads/task", "root": str(self.source), "tip": first}]
        for name in ("second", "third"):
            location = self.parent / ("primary-" + name)
            self.git(self.target, "worktree", "add", "-q", "-b", name, str(location), self.base)
            self.write(location, name + ".txt", name + " task\n")
            tip = self.commit(location, "feat: " + name + " task")
            sources.append({"ref": "refs/heads/" + name, "root": str(location), "tip": tip})
        self.write(self.target, "primary.txt", "Target work\n")
        self.base = self.commit(self.target, "feat: primary work")
        for tip in (self.base, *(item["tip"] for item in sources[1:])):
            parent = self.git(self.source, "rev-parse", "HEAD")
            self.git(self.source, "merge", "--no-ff", "--no-commit", tip)
            self.commit(self.source, "merge: awaiting combined verification", [parent, tip])
            Path(self.git(self.source, "rev-parse", "--path-format=absolute", "--git-path", "MERGE_HEAD")).unlink()
        return sources

    def test_batch_preserves_three_histories_and_publishes_one_combined_transition(self):
        sources = self.assemble_sources()
        assembled = self.git(self.source, "rev-parse", "HEAD")
        commit = self.prepare(sources=sources)
        evidence = integration.read(self.directory / "commit.json")
        self.assertEqual(set(evidence["scope"]), {"task.txt", "second.txt", "third.txt", "value.txt"})
        self.assertEqual(self.git(self.source, "rev-list", "--count", "--merges", sources[0]["tip"] + ".." + assembled), "3")
        self.assertEqual(integration.read(self.record)["sources"], sources)
        for item in sources:
            integration_state.ancestor(self.source, item["tip"], commit)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)
        integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)
        self.assertEqual(self.git(self.target, "reflog", "-1", "--format=%gs", "master"),
                         "local integration " + self.base + " -> " + commit)
        for name in ("task", "second", "third", "primary"):
            self.assertTrue((self.target / (name + ".txt")).is_file())

    def test_batch_rejects_unselected_history_missing_assembly_and_duplicate_sources(self):
        sources = self.assemble_sources()
        for selected, message in ((sources[1:], "include the assembly"),
                                  ([*sources, sources[1]], "unique local branches")):
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                self.prepare(pending=False, sources=selected)
        absent = {**sources[1], "tip": self.commit(Path(sources[1]["root"]), "feat: later history")}
        with self.assertRaisesRegex(ValueError, "absent from the assembled"):
            self.prepare(pending=False, sources=[sources[0], absent])
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_batch_source_approval_and_artifact_are_bound_to_freeze(self):
        sources = self.assemble_sources()
        self.prepare(sources=sources)
        frozen = integration.read(self.record)
        frozen["sources"] = sources[:1]
        with self.assertRaisesRegex(ValueError, "selected source approval"):
            integration.verify_seal(self.source, frozen)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_batch_failed_verification_preserves_target_and_all_sources(self):
        sources = self.assemble_sources()
        self.phases[0][1][0] = ("Fixture check", [sys.executable, "-c", "raise SystemExit(1)"], {})
        with self.assertRaises(AssertionError):
            self.prepare(sources=sources)
        self.assertFalse(self.record.exists())
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)
        for item in sources:
            self.assertTrue(Path(item["root"]).is_dir())
            self.assertTrue(self.git(self.target, "branch", "--list", item["ref"].removeprefix("refs/heads/")))

    def test_batch_dirty_source_and_advanced_tip_block_publication(self):
        sources = self.assemble_sources()
        self.prepare(sources=sources)
        other = Path(sources[1]["root"])
        later = self.write(other, "later.txt", "New work\n")
        with self.assertRaisesRegex(ValueError, "pending changes"):
            integration.publish(self.source, self.record)
        self.assertEqual(later.read_text(), "New work\n")
        self.commit(other, "feat: later work")
        with self.assertRaisesRegex(ValueError, "Selected source advanced"):
            integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_batch_publication_locks_every_selected_reference(self):
        sources = self.assemble_sources()
        commit = self.prepare(sources=sources)
        native_git = integration.git
        writers = []
        def during_checkout(root, *args):
            if args[0] == "read-tree":
                for item in sources:
                    writers.append(subprocess.run(["git", "update-ref", item["ref"], self.base],
                                                  cwd=self.target, capture_output=True).returncode)
            return native_git(root, *args)
        with patch.object(integration, "git", side_effect=during_checkout):
            integration.publish(self.source, self.record)
        self.assertEqual(len(writers), 3)
        self.assertTrue(all(code != 0 for code in writers))
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)

    def test_batch_source_writer_before_prepare_blocks_the_whole_publication(self):
        sources = self.assemble_sources()
        self.prepare(sources=sources)
        native = integration.RefTransaction
        def change_source(*args, **kwargs):
            self.git(self.target, "update-ref", sources[1]["ref"], self.base, sources[1]["tip"])
            return native(*args, **kwargs)
        with patch.object(integration, "RefTransaction", side_effect=change_source):
            with self.assertRaisesRegex(ValueError, "transaction refused"):
                integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)
        self.assertFalse((self.target / ".git/local-integration.json").exists())
        for item in sources:
            self.assertTrue(Path(item["root"]).exists())

    def test_batch_partial_cleanup_survives_assembly_removal_and_retries(self):
        sources = self.assemble_sources()
        commit = self.prepare(sources=sources)
        integration.publish(self.source, self.record)
        durable = integration.cleanup_record(integration.read(self.record))
        other = Path(sources[1]["root"])
        later = self.write(other, "later.txt", "Preserve later work\n")
        with patch.object(integration_cleanup, "stopped"), self.assertRaisesRegex(ValueError, "pending changes"):
            integration_cleanup.cleanup(self.target, self.record)
        self.assertFalse(self.source.exists())
        self.assertFalse(Path(sources[2]["root"]).exists())
        self.assertEqual(later.read_text(), "Preserve later work\n")
        self.assertTrue(durable.is_file())
        outcome = integration.read(durable)["cleanup"]
        self.assertEqual(outcome["refs/heads/task"]["branch"], "removed")
        self.assertEqual(outcome["refs/heads/second"]["directory"], "present")
        later.unlink()  # Resolve only the deliberate fixture obstruction.
        with patch.object(integration_cleanup, "stopped"):
            integration_cleanup.cleanup(self.target, durable)
            integration_cleanup.cleanup(self.target, durable)
        for item in sources:
            self.assertFalse(Path(item["root"]).exists())
            self.assertEqual(self.git(self.target, "branch", "--list", item["ref"].removeprefix("refs/heads/")), "")
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)

    def test_committed_feature_and_pending_rename_have_complete_scope(self):
        self.write(self.source, "committed.txt", "feature\n")
        self.commit(self.source)
        self.git(self.source, "mv", "value.txt", "renamed.txt")
        self.write(self.source, "new.txt", "pending\n")
        self.assertEqual(repository.collect_changes(self.source, self.base),
                         {"committed.txt", "value.txt", "renamed.txt", "new.txt"})
        self.assertNotIn("committed.txt", repository.collect_changes(self.source))

    def test_publication_installs_exact_tree_under_prepared_reference_locks(self):
        commit = self.prepare()
        integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)
        self.assertEqual((self.target / "value.txt").read_text(), "candidate\n")
        self.assertEqual(self.git(self.target, "status", "--porcelain"), "")
        self.assertIn("local integration", self.git(self.target, "reflog", "-1", "--format=%gs", "master"))
        self.assertEqual(integration.read(self.record)["state"], "published")

    def test_distinct_source_selections_at_the_same_commit_retain_both_cleanup_records(self):
        commit = self.prepare()
        integration.publish(self.source, self.record)
        first = integration.cleanup_record(integration.read(self.record))
        original = first.read_bytes()
        other = self.parent / "primary-other"
        self.git(self.target, "worktree", "add", "-q", "-b", "other", str(other), commit)
        confirmation = self.write(other, ".agent/tmp/confirmation.json", json.dumps({
            "base": commit, "commit": commit, "tree": self.git(other, "rev-parse", "HEAD^{tree}"),
            "plan_approved": True, "tested": True, "confirmation": "Fixture approval of a second source at the same commit."}))
        second = integration.accept_tested(other, other / ".agent/tmp/verification/commit", confirmation, "master")
        integration.publish(other, second)
        second_control = integration.cleanup_record(integration.read(second))
        self.assertNotEqual(first, second_control)
        self.assertEqual(first.read_bytes(), original)
        self.assertEqual(integration.read(second_control)["source_ref"], "refs/heads/other")
        with patch.object(integration_cleanup, "stopped"):
            integration_cleanup.cleanup(self.target, first)
            integration_cleanup.cleanup(self.target, second_control)
        self.assertFalse(self.source.exists())
        self.assertFalse(other.exists())
        self.assertTrue(first.is_file())
        self.assertTrue(second_control.is_file())

    def test_clean_committed_feature_requires_no_extra_commit(self):
        self.write(self.source, "value.txt", "committed feature\n")
        feature = self.commit(self.source)
        self.assertEqual(self.prepare(pending=False), feature)
        integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), feature)

    def confirmation(self, **overrides):
        record = {"base": self.base, "commit": self.git(self.source, "rev-parse", "HEAD"),
                  "tree": self.git(self.source, "rev-parse", "HEAD^{tree}"),
                  "plan_approved": True, "tested": True,
                  "confirmation": "User approved this exact fast-forward and confirmed the tip was tested."}
        record.update(overrides)
        return self.write(self.source, ".agent/tmp/confirmation.json", json.dumps(record))

    def test_confirmed_fast_forward_runs_no_profile_and_supports_cleanup(self):
        self.write(self.source, "value.txt", "already tested\n")
        feature = self.commit(self.source)
        self.confirmation()
        with patch.object(verify, "REPO_ROOT", self.source), \
             patch.object(integration, "validate_evidence", side_effect=AssertionError("Unexpected verification")), \
             contextlib.redirect_stdout(io.StringIO()) as output:
            result = verify.main(["integrate", "accept-tested", "confirmation", ".agent/tmp/confirmation.json",
                                  "target", "master"])
        self.assertEqual(result, 0, output.getvalue())
        self.assertIn("no new verification was run", output.getvalue())
        record = integration.read(self.record)
        self.assertEqual(record["verification"], "confirmed-fast-forward")
        self.assertNotIn("evidence", record)
        self.assertFalse((self.directory / "commit.json").exists())
        integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), feature)
        self.assertEqual((self.target / "value.txt").read_text(), "already tested\n")
        with patch.object(integration_cleanup, "stopped"):
            integration_cleanup.cleanup(self.target, self.record)
        self.assertFalse(self.source.exists())
        self.assertEqual(self.git(self.target, "branch", "--list", "task"), "")

    def test_fast_forward_requires_affirmative_plan_and_test_confirmation(self):
        for changed in ({"tested": False}, {"tested": None}, {"tested": 1},
                        {"plan_approved": False}, {"confirmation": " "}):
            with self.subTest(changed=changed), self.assertRaisesRegex(ValueError, "Explicit plan approval"):
                integration.accept_tested(self.source, self.directory, self.confirmation(**changed), "master")
        self.assertFalse(self.record.exists())
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_fast_forward_confirmation_rejects_pending_work_and_divergence(self):
        self.write(self.source, "value.txt", "uncommitted\n")
        with self.assertRaisesRegex(ValueError, "pending changes"):
            integration.accept_tested(self.source, self.directory, self.confirmation(), "master")
        self.commit(self.source)
        self.write(self.target, "primary.txt", "divergent target\n")
        self.base = self.commit(self.target)
        with self.assertRaisesRegex(ValueError, "not an ancestor"):
            integration.accept_tested(self.source, self.directory, self.confirmation(), "master")
        self.assertFalse(self.record.exists())

    def test_confirmed_publication_rejects_changed_confirmation_and_source(self):
        self.write(self.source, "value.txt", "tested\n")
        feature = self.commit(self.source)
        confirmation = self.confirmation()
        integration.accept_tested(self.source, self.directory, confirmation, "master")
        saved = confirmation.read_bytes()
        confirmation.write_text("{}")
        with self.assertRaisesRegex(ValueError, "confirmation artifact changed"):
            integration.publish(self.source, self.record)
        confirmation.write_bytes(saved)
        self.write(self.source, "value.txt", "later edit\n")
        with self.assertRaisesRegex(ValueError, "pending changes"):
            integration.publish(self.source, self.record)
        self.commit(self.source)
        with self.assertRaisesRegex(ValueError, "advanced after sealing"):
            integration.publish(self.source, self.record)
        self.assertNotEqual(self.git(self.source, "rev-parse", "HEAD"), feature)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_confirmed_publication_refuses_a_new_target_baseline(self):
        self.write(self.source, "value.txt", "tested\n")
        self.commit(self.source)
        confirmation = self.confirmation()
        integration.accept_tested(self.source, self.directory, confirmation, "master")
        self.write(self.target, "primary.txt", "new target work\n")
        other = self.commit(self.target)
        with self.assertRaisesRegex(ValueError, "Target advanced"):
            integration.publish(self.source, self.record)
        with self.assertRaisesRegex(ValueError, "Target advanced"):
            integration.accept_tested(self.source, self.directory, confirmation, "master")
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), other)

    def test_interrupted_confirmed_publication_uses_existing_recovery(self):
        self.write(self.source, "value.txt", "tested\n")
        self.commit(self.source)
        integration.accept_tested(self.source, self.directory, self.confirmation(), "master")
        with patch.object(integration.RefTransaction, "commit", side_effect=ValueError("interruption")):
            with self.assertRaisesRegex(ValueError, "interruption"):
                integration.publish(self.source, self.record)
        integration.recover(self.target)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)
        self.assertEqual((self.target / "value.txt").read_text(), "baseline\n")
        integration.publish(self.source, self.record)
        self.assertEqual((self.target / "value.txt").read_text(), "tested\n")

    def test_force_added_ignored_file_remains_in_the_reviewed_tree(self):
        self.write(self.source, "ignored-feature", "explicit candidate\n")
        self.git(self.source, "add", "-f", "ignored-feature")
        # The final content may differ from the force-added index version.
        self.write(self.source, "ignored-feature", "final candidate\n")
        commit = self.prepare()
        integration.publish(self.source, self.record)
        self.assertEqual((self.target / "ignored-feature").read_text(), "final candidate\n")
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)

    def test_wrong_mechanical_message_cannot_be_sealed(self):
        with self.assertRaisesRegex(ValueError, "frozen tree, parents or message"):
            self.prepare(message="feat: unreviewed message")
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_empty_partition_cannot_bridge_a_rewind_to_a_same_tree_ancestor(self):
        same_tree_commit = self.commit(self.source, "feat: same tree history")
        self.prepare(pending=False)
        self.git(self.source, "update-ref", "refs/heads/task", self.base, same_tree_commit)
        self.assertEqual(self.git(self.source, "rev-parse", "HEAD^{tree}"), self.git(self.source, "rev-parse", same_tree_commit + "^{tree}"))
        with self.assertRaisesRegex(ValueError, "frozen history"):
            integration.verify_seal(self.source, integration.read(self.record))

    def test_ref_writer_between_comparison_and_prepare_cannot_be_overwritten(self):
        self.prepare()
        self.write(self.target, "concurrent.txt", "other publisher\n")
        other = self.commit(self.target)
        self.git(self.target, "read-tree", "-m", "-u", other, self.base)
        self.git(self.target, "update-ref", "refs/heads/master", self.base, other)
        native = integration.RefTransaction
        def advance(*args, **kwargs):
            self.git(self.target, "read-tree", "-m", "-u", self.base, other)
            self.git(self.target, "update-ref", "refs/heads/master", other, self.base)
            return native(*args, **kwargs)
        with patch.object(integration, "RefTransaction", side_effect=advance), self.assertRaisesRegex(ValueError, "transaction refused"):
            integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), other)
        self.assertTrue((self.target / "concurrent.txt").exists())
        self.assertFalse((self.target / ".git/local-integration.json").exists())

    def test_dirty_staged_operation_head_and_ignored_collisions_are_preserved(self):
        self.prepare()
        self.write(self.target, "value.txt", "local edit\n")
        for staged in (False, True):
            if staged:
                self.git(self.target, "add", "value.txt")
            with self.assertRaisesRegex(ValueError, "pending changes"):
                integration.publish(self.source, self.record)
            self.assertEqual((self.target / "value.txt").read_text(), "local edit\n")
        self.git(self.target, "read-tree", self.base)
        self.write(self.target, "value.txt", "baseline\n")
        marker = self.target / ".git/MERGE_HEAD"
        marker.write_text(self.base)
        with self.assertRaisesRegex(ValueError, "operation"):
            integration.publish(self.source, self.record)
        marker.unlink()
        self.git(self.target, "symbolic-ref", "HEAD", "refs/heads/unexpected")
        with self.assertRaisesRegex(ValueError, "checkout"):
            integration.publish(self.source, self.record)
        self.git(self.target, "symbolic-ref", "HEAD", "refs/heads/master")
        self.write(self.target, "ignored-new", "unique ignored file\n")
        # The source introduces a tracked file with the same ignored name.
        self.write(self.source, "ignored-new", "candidate\n")
        self.git(self.source, "add", "-f", "ignored-new")
        self.commit(self.source)
        with self.assertRaisesRegex(ValueError, "advanced"):
            integration.publish(self.source, self.record)
        with self.assertRaisesRegex(ValueError, "overwritten"):
            integration_state.collisions(self.target, self.git(self.source, "rev-parse", "HEAD"))
        self.assertEqual((self.target / "ignored-new").read_text(), "unique ignored file\n")

    def test_frozen_review_tree_and_artifacts_block_stale_publication(self):
        self.prepare()
        artifacts = integration.read(self.directory / "commit.json")["groups"]["Fixture check"]["artifacts"]
        artifact = self.directory / next(name for name in artifacts if name.endswith(".log"))
        artifact.write_text("tampered\n")
        with self.assertRaisesRegex(ValueError, "artifact changed"):
            integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_changed_review_or_partition_blocks_sealed_publication(self):
        self.prepare()
        review_path = self.source / ".agent/tmp/review.json"
        review = integration.read(review_path)
        review["verdict"] = "CHANGES REQUESTED"
        review_path.write_text(json.dumps(review))
        with self.assertRaisesRegex(ValueError, "review or partition artifact"):
            integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)

    def test_divergent_merge_preserves_both_parents_and_exact_result(self):
        self.write(self.source, "feature.txt", "task branch\n")
        feature = self.commit(self.source)
        self.write(self.target, "primary.txt", "primary branch\n")
        self.base = self.commit(self.target)
        self.git(self.source, "merge", "--no-ff", "--no-commit", self.base)
        self.assertTrue(repository.divergent_integration(self.source, self.base))
        native_commit = self.commit
        def merge_commit(root, message="fixture", parents=None):
            result = native_commit(root, message, [feature, self.base])
            Path(self.git(root, "rev-parse", "--path-format=absolute", "--git-path", "MERGE_HEAD")).unlink()
            return result
        with patch.object(self, "commit", side_effect=merge_commit):
            commit = self.prepare()
        integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "show", "-s", "--format=%P", commit).split(), [feature, self.base])
        self.assertTrue((self.target / "feature.txt").is_file())
        self.assertTrue((self.target / "primary.txt").is_file())

    def test_immutable_base_and_stale_completion_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "immutable"):
            repository.resolve_base(self.source, "master")
        self.prepare()
        record = integration.read(self.directory / "commit.json")
        record["required"] = []
        (self.directory / "commit.json").write_text(json.dumps(record))
        # A fresh freeze must reject even apparently passing truncated records.
        with self.assertRaises(ValueError):
            integration.freeze(self.source, self.directory, self.source / ".agent/tmp/review.json",
                               self.source / ".agent/tmp/partition.json", "master")

    def test_rebased_divergent_source_retains_full_selection_through_freeze(self):
        self.write(self.source, "docs/task.md", "Task documentation\n")
        original = self.commit(self.source)
        self.write(self.target, "primary.txt", "New primary work\n")
        self.base = self.commit(self.target)
        self.git(self.source, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "rebase", self.base)
        self.assertFalse(repository.divergent_integration(self.source, self.base))
        self.assertTrue(repository.divergent_integration(self.source, self.base, original))
        self.assertFalse(repository.divergent_integration(self.source, self.base, self.base))
        seen = []

        def select(args, changes, *extra):
            seen.append(args.integration_divergent)
            return self.phases

        with patch.object(verify, "select_phases", side_effect=select), \
             patch.object(commands, "select_phases", side_effect=select):
            commit = self.prepare(pending=False, original_source=original)
        self.assertEqual(seen, [True, True])  # Execution and independent freeze revalidation.
        self.assertEqual(integration.read(self.record)["evidence"]["integration_source"], original)

        _, args = parse_args(["commit", "--integration-base", self.base, "--integration-source", original])
        args.integration_divergent = repository.divergent_integration(self.source, self.base, original)
        phases = self.native_selection(args, {"docs/task.md"})
        self.assertIn("Coverage", [name for name, _ in phases])
        self.assertTrue(any(title == "All standard harness tests" for _, items in phases for title, _, _ in items))
        integration.publish(self.source, self.record)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)
        self.assertTrue((self.target / "docs/task.md").is_file())

    def test_original_source_requires_an_immutable_integration_context(self):
        with patch.object(verify, "REPO_ROOT", self.source), contextlib.redirect_stderr(io.StringIO()):
            for sentence in (["commit", "--integration-source", self.base, "--plan"],
                             ["commit", "--integration-base", self.base, "--integration-source", "task", "--plan"]):
                with self.subTest(sentence=sentence), self.assertRaises(SystemExit) as error:
                    verify.main(sentence)
                self.assertEqual(error.exception.code, 2)

    def child_code(self, extra=""):
        module = str(Path(integration.__file__).parent)
        return (f"import sys; sys.path.insert(0, {module!r})\n"
                "import integration, integration_state, time, os\nfrom pathlib import Path\n"
                "integration.execution_conditions = lambda root: 'fixture'\n"
                f"source=Path({str(self.source)!r}); record=Path({str(self.record)!r})\n" + extra +
                "\nintegration.publish(source, record)\n")

    def test_two_real_publishers_only_one_succeeds(self):
        commit = self.prepare()
        barrier = self.parent / "start"
        code = self.child_code(f"while not Path({str(barrier)!r}).exists(): time.sleep(0.01)\n")
        processes = [subprocess.Popen([sys.executable, "-c", code], stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(2)]
        barrier.touch()
        results = [process.communicate(timeout=20) for process in processes]
        self.assertEqual(sorted(process.returncode for process in processes), [0, 1], results)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)
        self.assertEqual(self.git(self.target, "status", "--porcelain"), "")

    def test_recovery_locks_out_a_normal_reference_writer_during_rollback(self):
        commit = self.prepare()
        with patch.object(integration.RefTransaction, "commit", side_effect=ValueError("interruption")):
            with self.assertRaisesRegex(ValueError, "interruption"):
                integration.publish(self.source, self.record)
        native_git = integration.git
        writers = []
        def during_rollback(root, *args):
            if args[0] == "read-tree":
                writers.append(subprocess.run(["git", "update-ref", "refs/heads/master", commit, self.base],
                                              cwd=self.target, capture_output=True).returncode)
            return native_git(root, *args)
        with patch.object(integration, "git", side_effect=during_rollback):
            integration.recover(self.target)
        self.assertEqual(len(writers), 1)
        self.assertNotEqual(writers[0], 0)
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), self.base)
        self.assertEqual(self.git(self.target, "status", "--porcelain"), "")
        self.assertFalse((self.target / ".git/local-integration.json").exists())

    def test_real_termination_and_recovery_at_publication_boundaries(self):
        commit = self.prepare()
        sealed = self.record.read_bytes()
        for stage in ("before_prepare", "after_prepare", "during_checkout", "before_commit", "after_commit"):
            with self.subTest(stage=stage):
                marker = self.parent / (stage + ".ready")
                extra = f"""
stage={stage!r}
def stop():
    Path({str(marker)!r}).touch()
    time.sleep(60)
native_send=integration_state.RefTransaction.send
def send(self, command):
    if command == 'prepare' and stage == 'before_prepare': stop()
    if command == 'commit' and stage == 'before_commit': stop()
    result=native_send(self, command)
    if command == 'prepare' and stage == 'after_prepare': stop()
    if command == 'commit' and stage == 'after_commit': stop()
    return result
integration_state.RefTransaction.send=send
native_git=integration.git
def git(root, *args):
    if args[0] == 'read-tree' and stage == 'during_checkout':
        (root / 'value.txt').write_text('interrupted edit\\n')
        stop()
    return native_git(root, *args)
integration.git=git
"""
                process = subprocess.Popen([sys.executable, "-c", self.child_code(extra)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                try:
                    deadline = time.monotonic() + 15
                    while not marker.exists() and process.poll() is None and time.monotonic() < deadline:
                        time.sleep(0.02)
                    self.assertTrue(marker.exists(), process.poll())
                finally:
                    process.terminate()
                    process.communicate(timeout=10)
                # EOF releases the orphaned native transaction's prepared locks.
                deadline = time.monotonic() + 5
                while (self.target / ".git/refs/heads/master.lock").exists() and time.monotonic() < deadline:
                    time.sleep(0.02)
                phase = self.target / ".git/local-integration.json"
                if stage == "during_checkout":
                    with self.assertRaisesRegex(ValueError, "unexpected edits"):
                        integration.recover(self.target)
                    self.assertEqual((self.target / "value.txt").read_text(), "interrupted edit\n")
                    # Resolve the deliberate fixture edit; recovery can now abort.
                    self.write(self.target, "value.txt", "baseline\n")
                    integration.recover(self.target)
                elif phase.exists():
                    integration.recover(self.target)
                expected = commit if stage == "after_commit" else self.base
                self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), expected)
                self.assertEqual(self.git(self.target, "status", "--porcelain"), "")
                self.assertTrue(self.source.exists())
                self.assertFalse(phase.exists())
                if expected == commit:
                    self.git(self.target, "read-tree", "-m", "-u", commit, self.base)
                    self.git(self.target, "update-ref", "refs/heads/master", self.base, commit)
                self.record.write_bytes(sealed)

    def test_cleanup_discards_all_source_ignored_files_and_removes_only_source(self):
        commit = self.prepare()
        integration.publish(self.source, self.record)
        self.write(self.source, ".agent/tasks/plan.md", "active evidence\n")
        self.write(self.source, ".env", "unique local configuration\n")
        self.write(self.source, "target/build-output", "generated output\n")
        self.write(self.target, ".env", "primary local configuration\n")
        with patch.object(integration_cleanup, "stopped"):
            integration_cleanup.cleanup(self.target, self.record)
        self.assertFalse(self.source.exists())
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)
        self.assertFalse(self.record.exists(), "Source verification evidence is discarded too")
        self.assertFalse((self.target / ".agent/tmp/integration-archives").exists())
        self.assertEqual((self.target / ".env").read_text(), "primary local configuration\n")
        self.assertEqual(self.git(self.target, "branch", "--list", "task"), "")

    def test_cleanup_refuses_later_work_and_an_active_command(self):
        self.prepare()
        integration.publish(self.source, self.record)
        self.write(self.source, "new-work.txt", "later task\n")
        with self.assertRaisesRegex(ValueError, "pending changes"):
            integration_cleanup.cleanup(self.target, self.record)
        (self.source / "new-work.txt").unlink()
        process = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"], cwd=self.source)
        try:
            with self.assertRaisesRegex(ValueError, "process|ownership"):
                integration_cleanup.cleanup(self.target, self.record)
        finally:
            process.terminate()
            process.wait(timeout=10)
        self.assertTrue(self.source.exists())

    def test_cleanup_ignores_inaccessible_system_processes_but_blocks_source_commands(self):
        system = Mock(pid=288)
        system.cwd.side_effect = psutil.AccessDenied(pid=288)
        system.username.side_effect = AssertionError("Process owners are not needed")
        command = Mock(pid=123)
        command.cwd.return_value = str(self.source)
        with patch.object(psutil, "process_iter", return_value=[system]):
            integration_cleanup.stopped(self.source)
        with patch.object(psutil, "process_iter", return_value=[system, command]):
            with self.assertRaisesRegex(ValueError, "source checkout"):
                integration_cleanup.stopped(self.source)

    def test_failed_cleanup_retains_publication_and_can_retry_original_record(self):
        commit = self.prepare()
        integration.publish(self.source, self.record)
        native_run = subprocess.run
        def fail_removal(command, *args, **kwargs):
            if command[0] in {"powershell", "pwsh"}:
                return subprocess.CompletedProcess(command, 1)
            return native_run(command, *args, **kwargs)
        with patch.object(integration_cleanup, "stopped"), patch.object(subprocess, "run", side_effect=fail_removal):
            with self.assertRaisesRegex(ValueError, "Integration succeeded; worktree cleanup failed"):
                integration_cleanup.cleanup(self.target, self.record)
        self.assertTrue(self.source.exists())
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)
        self.assertTrue(self.record.exists())
        self.assertFalse((self.target / ".agent/tmp/integration-archives").exists())
        with patch.object(integration_cleanup, "stopped"):
            integration_cleanup.cleanup(self.target, self.record)
        self.assertFalse(self.source.exists())
        self.assertEqual(self.git(self.target, "branch", "--list", "task"), "")

    def test_cleanup_can_finish_branch_removal_with_an_external_input_record(self):
        commit = self.prepare()
        integration.publish(self.source, self.record)
        record = self.parent / "published-input.json"
        record.write_bytes(self.record.read_bytes())
        self.git(self.target, "worktree", "remove", str(self.source))
        integration_cleanup.cleanup(self.target, record)
        self.assertEqual(self.git(self.target, "branch", "--list", "task"), "")
        self.assertEqual(self.git(self.target, "rev-parse", "HEAD"), commit)
        self.assertEqual(integration.read(record)["state"], "published")
        integration_cleanup.cleanup(self.target, record)


if __name__ == "__main__":
    unittest.main()
