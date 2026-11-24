"""Exercise the current environment inventory with local inputs and controlled probes."""

from argparse import Namespace
from contextlib import redirect_stderr
from hashlib import sha256
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import environment

from capture_fixtures import case_directory


class EnvironmentInventoryTests(unittest.TestCase):
    def setUp(self):
        self.root = case_directory(self)

    def write(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path

    def test_capture_round_trip_compares_client_tools_and_dependencies(self):
        commit = "a" * 40
        requirements = "tools/verification/requirements.txt"
        original = b"pytest==8.0\n"
        update = b"pytest==8.1\n"
        executable = self.root / "git.exe"
        turn = "11111111-1111-4111-8111-111111111111"

        def rollout(model, effort):
            events = [{"type": "session_meta", "payload": {"cli_version": "1.2.3", "source": "cli"}},
                      {"type": "turn_context", "payload": {"turn_id": turn, "model": model, "effort": effort}}]
            return self.write("rollout.jsonl", b"\n".join(json.dumps(event).encode() for event in events))

        def git_bytes(operation, *arguments):
            if operation == "rev-parse":
                self.assertEqual(arguments[-1], "HEAD^{commit}")
                return commit.encode() + b"\n"
            if operation == "cat-file" and arguments == ("blob", f"{commit}:{requirements}"):
                return original
            return None

        def snapshot(name, session_log, tool_version):
            args = Namespace(governing_commit="HEAD", docker_image=[], node_project=[], shell="PowerShell",
                             session_log=session_log, turn_id=turn)
            tool = subprocess.CompletedProcess([str(executable), "--version"], 0,
                                               stdout=f"git version {tool_version}\n".encode())
            with (patch.object(environment, "ROOT", self.root), patch.object(environment, "git_bytes", git_bytes),
                  patch.object(environment, "COMMANDS", (("git", "--version"), ("node", "--version"))),
                  patch.object(environment.shutil, "which", side_effect=lambda name: str(executable) if name == "git" else None),
                  patch.object(environment, "run", return_value=tool),
                  patch.object(environment, "distributions", return_value=[])):
                captured = environment.capture(args)
            return environment.load_inventory(self.write(name, json.dumps(captured).encode()))

        saved = snapshot("saved.json", self.root / "rollout.jsonl", "2.48.0")
        self.assertEqual(saved["format_version"], 2)
        self.assertEqual(saved["governing_commit"], commit)
        self.assertIsNone(saved["role_settings"])
        self.assertEqual(saved["declared_mcp_servers"], ["fixture"])
        self.assertEqual(saved["client"]["turn_id"], turn)
        self.assertEqual(saved["repository_input_git_blob_sha256"][requirements], sha256(original).hexdigest())
        unchanged = environment.compare(saved, saved)
        self.assertEqual(unchanged["changed"], [])
        self.assertEqual(unchanged["missing"], [])
        self.assertIn("probes/node --version", unchanged["unknown"])

        self.write(requirements, update)
        self.write("git.exe", b"updated tool")
        fresh = snapshot("fresh.json", rollout("updated-model", "high"), "2.49.0")
        changes = environment.compare(saved, fresh)
        self.assertEqual(changes["missing"], [])
        differences = {entry["item"]: (entry["saved"], entry["fresh"]) for entry in changes["changed"]}
        self.assertEqual(differences, {
            "client/effort": ("low", "high"),
            "client/model": ("fixture-model", "updated-model"),
            "executables/git --version/sha256": (sha256(b"original tool").hexdigest(),
                                                  sha256(b"updated tool").hexdigest()),
            "probes/git --version": ("git version 2.48.0", "git version 2.49.0"),
            f"python_requirements/{requirements}/requirements/pytest/specifier": ("==8.0", "==8.1"),
            f"repository_input_checkout_sha256/{requirements}": (sha256(original).hexdigest(),
                                                                  sha256(update).hexdigest()),
        })

    def test_compare_reports_required_format_for_an_unsupported_inventory(self):
        content = b'{"format_version": 1, "probes": []}'
        saved = self.root / "old.json"
        arguments = ["environment.py", "compare", str(saved), str(saved)]
        with (patch.object(sys, "argv", arguments), redirect_stderr(io.StringIO()) as errors,
              self.assertRaises(SystemExit) as exit_status):
            environment.main()
        self.assertEqual(exit_status.exception.code, 2)
        self.assertIn("format 2", errors.getvalue())
        self.assertEqual(saved.read_bytes(), content)
