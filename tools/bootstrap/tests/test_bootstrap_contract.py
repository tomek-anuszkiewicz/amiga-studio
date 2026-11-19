"""Regression coverage for the shared RAG bootstrap entry points."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


BOOTSTRAP_DIRECTORY = Path(__file__).resolve().parents[1]


class BootstrapRagContractTests(unittest.TestCase):
    def setUp(self):
        temporary_root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.repo_root = temporary_root / "Amiga fixture"
        self.script_root = self.repo_root / "tools" / "bootstrap"
        self.script_root.mkdir(parents=True)
        for script in ("bootstrap.ps1", "bootstrap_rag.ps1", "ensure_qdrant.ps1"):
            shutil.copyfile(BOOTSTRAP_DIRECTORY / script, self.script_root / script)

        self.tier_log = temporary_root / "tiers.log"
        for tier in ("sources", "graphify", "documentation", "assembler"):
            (self.script_root / f"bootstrap_{tier}.ps1").write_text(
                f'Add-Content -LiteralPath $env:BOOTSTRAP_TEST_TIER_LOG -Value "{tier}"\n'
                "exit 0\n",
                encoding="utf-8",
            )

        self.rag_log = temporary_root / "rag.jsonl"
        self.docker_log = temporary_root / "docker.log"
        self.index_state = temporary_root / "shared [state].json"
        self.environment = os.environ.copy()
        self.environment.pop("RAG_INDEX_JSON", None)
        self.environment.pop("RAG_SERVICE_MODE", None)
        self.environment.update(
            PATH=f"{temporary_root}{os.pathsep}{self.environment['PATH']}",
            BOOTSTRAP_TEST_MODE="compose",
            BOOTSTRAP_TEST_MODE_EXIT="0",
            BOOTSTRAP_TEST_RAG_LOG=str(self.rag_log),
            BOOTSTRAP_TEST_DOCKER_LOG=str(self.docker_log),
            BOOTSTRAP_TEST_TIER_LOG=str(self.tier_log),
            BOOTSTRAP_TEST_START_EXIT="0",
            BOOTSTRAP_TEST_STATUS_EXIT="0",
        )

        fake_rag = temporary_root / "fake_rag.py"
        fake_rag.write_text(
            "import json, os, sys\n"
            "args = sys.argv[1:]\n"
            "with open(os.environ['BOOTSTRAP_TEST_RAG_LOG'], 'a', encoding='utf-8') as log:\n"
            "    print(json.dumps(args), file=log)\n"
            "if args == ['service', 'mode']:\n"
            "    mode = os.environ.get('BOOTSTRAP_TEST_MODE')\n"
            "    if mode:\n"
            "        print(mode)\n"
            "    sys.exit(int(os.environ['BOOTSTRAP_TEST_MODE_EXIT']))\n"
            "if args == ['service', 'start']:\n"
            "    sys.exit(int(os.environ['BOOTSTRAP_TEST_START_EXIT']))\n"
            "if args == ['--status']:\n"
            "    print('Shared RAG status')\n"
            "    sys.exit(int(os.environ['BOOTSTRAP_TEST_STATUS_EXIT']))\n",
            encoding="utf-8",
        )
        (temporary_root / "rag_qdrant.cmd").write_text(
            f'@echo off\n"{sys.executable}" "%~dp0fake_rag.py" %*\nexit /b %errorlevel%\n',
            encoding="utf-8",
        )
        (temporary_root / "docker.cmd").write_text(
            '@echo off\necho %*>>"%BOOTSTRAP_TEST_DOCKER_LOG%"\n'
            "@echo true\nexit /b 0\n",
            encoding="utf-8",
        )

    def run_bootstrap(self, script, *arguments, **environment):
        completed = subprocess.run(
            [
                "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass",
                "-File", str(self.script_root / script), *arguments,
            ],
            cwd=self.repo_root,
            env={**self.environment, **environment},
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        self.assertFalse(self.docker_log.exists(), "Amiga bootstrap called Docker directly")
        return completed

    def take_rag_calls(self):
        if not self.rag_log.exists():
            return []
        calls = [json.loads(line) for line in self.rag_log.read_text(encoding="utf-8").splitlines()]
        self.rag_log.unlink()
        return calls

    def test_all_only_starts_and_checks_shared_rag(self):
        self.index_state.write_bytes(b'{"unchanged": true}\n')
        completed = self.run_bootstrap("bootstrap.ps1", "-All", RAG_INDEX_JSON=str(self.index_state))

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("Shared RAG status", completed.stdout)
        self.assertEqual(self.take_rag_calls(), [["service", "mode"], ["service", "start"], ["--status"]])
        self.assertEqual(self.index_state.read_bytes(), b'{"unchanged": true}\n')
        self.assertEqual(
            self.tier_log.read_text(encoding="utf-8").splitlines(),
            ["sources", "graphify", "documentation", "assembler"],
        )

    def test_check_only_does_not_require_index_state(self):
        completed = self.run_bootstrap("bootstrap_rag.ps1", "-CheckOnly")

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("Shared RAG status", completed.stdout)
        self.assertEqual(self.take_rag_calls(), [["service", "mode"], ["service", "start"], ["--status"]])

    def test_explicit_index_routes_use_amiga_scopes_and_canonical_state(self):
        for script, argument in (("bootstrap.ps1", "-Rag"), ("bootstrap_rag.ps1", "-Index")):
            with self.subTest(script=script, argument=argument):
                completed = self.run_bootstrap(script, argument, RAG_INDEX_JSON=str(self.index_state))

                self.assertEqual(completed.returncode, 0, completed.stderr)
                index_flags = [
                    "--repo-root", str(self.repo_root), "--source", "amiga",
                    "--index-json", str(self.index_state),
                ]
                self.assertEqual(
                    self.take_rag_calls(),
                    [
                        ["service", "mode"],
                        ["service", "start"],
                        [str(self.repo_root / "docs"), *index_flags],
                        [str(self.repo_root / "Obsidian/Amiga/Design"), *index_flags],
                        [str(self.repo_root / "Obsidian/Amiga/Reference"), *index_flags],
                        ["--status"],
                    ],
                )

    def test_no_arguments_print_usage_without_running_tiers_or_rag(self):
        for script in ("bootstrap.ps1", "bootstrap_rag.ps1"):
            with self.subTest(script=script):
                completed = self.run_bootstrap(script)

                self.assertEqual(completed.returncode, 0, completed.stderr)
                self.assertIn("Usage:", completed.stdout)
                self.assertEqual(self.take_rag_calls(), [])
                self.assertFalse(self.tier_log.exists())

    def test_invalid_rag_inputs_fail_before_invoking_cli(self):
        cases = (
            (("-Index", "-CheckOnly"), {}, "Choose one action"),
            (("-Index",), {}, "RAG_INDEX_JSON"),
        )
        for arguments, environment, diagnostic in cases:
            with self.subTest(arguments=arguments, environment=environment):
                completed = self.run_bootstrap("bootstrap_rag.ps1", *arguments, **environment)

                self.assertNotEqual(completed.returncode, 0)
                self.assertIn(diagnostic, completed.stderr)
                self.assertEqual(self.take_rag_calls(), [])

    def test_mode_probe_refuses_startup_and_indexing_without_compose_confirmation(self):
        for mode, exit_code in (
            ("host", "0"),
            ("unknown", "0"),
            ("", "0"),
            ("compose\nextra output", "0"),
            ("compose", "19"),
        ):
            for argument in ("-CheckOnly", "-Index"):
                with self.subTest(mode=mode, exit_code=exit_code, action=argument):
                    completed = self.run_bootstrap(
                        "bootstrap_rag.ps1", argument,
                        BOOTSTRAP_TEST_MODE=mode,
                        BOOTSTRAP_TEST_MODE_EXIT=exit_code,
                        RAG_INDEX_JSON=str(self.index_state),
                    )

                    self.assertNotEqual(completed.returncode, 0)
                    diagnostic = " ".join(completed.stderr.split())
                    self.assertIn("rag_qdrant service mode must report compose", diagnostic)
                    self.assertEqual(self.take_rag_calls(), [["service", "mode"]])
                    self.assertFalse(self.index_state.exists())

    def test_service_failures_stop_the_coordinator_with_the_cli_exit_code(self):
        for start_exit, status_exit, expected_calls in (
            (23, 0, [["service", "mode"], ["service", "start"]]),
            (0, 24, [["service", "mode"], ["service", "start"], ["--status"]]),
        ):
            with self.subTest(start_exit=start_exit, status_exit=status_exit):
                completed = self.run_bootstrap(
                    "bootstrap.ps1", "-All",
                    BOOTSTRAP_TEST_START_EXIT=str(start_exit),
                    BOOTSTRAP_TEST_STATUS_EXIT=str(status_exit),
                )

                self.assertEqual(completed.returncode, start_exit or status_exit, completed.stderr)
                self.assertEqual(self.take_rag_calls(), expected_calls)
                self.assertEqual(
                    self.tier_log.read_text(encoding="utf-8").splitlines(),
                    ["sources", "graphify", "documentation"],
                )
                self.tier_log.unlink()


if __name__ == "__main__":
    unittest.main()
