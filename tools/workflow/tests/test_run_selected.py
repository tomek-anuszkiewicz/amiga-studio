"""Planned launches bind the original registration before starting the native command."""

import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
import unittest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
from run_selected import selected_environment


class SelectedCommandTests(unittest.TestCase):
    def setUp(self):
        temp = TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name).resolve()
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("AMIGA_TEST_")}

    def register(self, task, execution, **extra):
        receipt = self.root / ".agent/task-archives" / task / execution / "registration.json"
        receipt.parent.mkdir(parents=True)
        receipt.write_text(json.dumps(dict(task_id=task, execution_id=execution, root_thread_id="fixture",
                                           start_utc="2026-01-01T00:00:00Z", **extra)))
        return receipt

    def test_recovers_pair_and_keeps_simultaneous_commands_independent(self):
        first = self.register("plan-a", "active-a")
        second = self.register("plan-b", "active-b")
        one = selected_environment(first, self.root, self.env, "review", 2)
        two = selected_environment(second, self.root, self.env, "repair", 1)
        self.assertEqual([one["AMIGA_TEST_TASK_ID"], one["AMIGA_TEST_EXECUTION_ID"],
                          one["AMIGA_TEST_WORK_ITEM_ID"], one["AMIGA_TEST_ROUND"]],
                         ["plan-a", "active-a", "review", "2"])
        self.assertEqual(two["AMIGA_TEST_EXECUTION_ID"], "active-b")
        self.assertNotIn("AMIGA_TEST_EXECUTION_ID", self.env)

    def test_checks_identity_before_launch_and_preserves_native_exit(self):
        receipt = self.register("p", "e")
        target = self.root / "started"
        command = [sys.executable, str(TOOLS / "run_selected.py"), "--registration", str(receipt), "--",
                   sys.executable, "-c", "import pathlib; pathlib.Path('started').touch(); raise SystemExit(13)"]
        failed = subprocess.run(command, cwd=self.root, env={**self.env, "AMIGA_TEST_TASK_ID": "other"},
                                capture_output=True)
        self.assertEqual(failed.returncode, 2)
        self.assertFalse(target.exists())
        self.assertEqual(subprocess.run(command, cwd=self.root, env=self.env).returncode, 13)
        self.assertTrue(target.exists())
        with self.assertRaises(ValueError):
            selected_environment(self.register("p", "ended", end_utc="2026-02-01"), self.root, self.env)
