"""Cargo manifest diagnostics through the manual and coverage entry points."""

import contextlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch

from test_case_report import TASK_CONTEXT, init_repository

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

import commands
import execution
import unused_dependencies
from case_report import CONTEXT


WARNINGS = (
    "warning: unused dependency `bitflags`\n"
    "  --> crates\\cpu\\Cargo.toml:15:1\n"
    "   |\n15 | bitflags = { workspace = true }\n   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n"
    "   |\n   = note: `cargo::unused_dependencies` is set to `warn` by default\n"
    "help: consider removing the dependency on `bitflags`\n"
    "warning: `cpu` (manifest) generated 1 warning\n"
    "warning: unused dependency `serde_json`\n"
    "  --> crates\\physical_memory\\Cargo.toml:14:1\n"
    "   |\n14 | serde_json = { workspace = true }\n   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n"
    "   |\n   = note: `cargo::unused_dependencies` is set to `warn` by default\n"
    "help: consider removing the dependency on `serde_json`\n"
    "help: to still use for development builds, move to `dev-dependencies`\n"
    "warning: `physical_memory` (manifest) generated 1 warning\n"
)

FINDING = "[FAIL] Unused dependency "
VERIFY_DEPENDENCIES = ["check", "unused", "dependencies"]
VERIFY_COVERAGE = ["test", "coverage", "workspace"]


def findings(output):
    return [line.removeprefix(FINDING) for line in output.splitlines() if line.startswith(FINDING)]


class RecordedDiagnosticsTests(unittest.TestCase):
    def test_coverage_keeps_both_manifest_findings_at_every_display_level(self):
        command = commands.select_coverage_commands("coverage")[2][1]
        self.assertEqual(command[:3], ["cargo", "llvm-cov", "nextest"])
        self.assertEqual(command[command.index("--workspace") + 1:][:2], ["--exclude", "test_runner"])
        expected = ["`bitflags` at crates/cpu/Cargo.toml:15",
                    "`serde_json` at crates/physical_memory/Cargo.toml:14"]
        excluded = "warning: unused dependency `test_fixture`\n  --> crates/test_runner/Cargo.toml:8:1\n"
        duplicate = "warning: unused dependency `bitflags`\n  --> crates/cpu/Cargo.toml:15:1\n"
        runtime = ("    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s\n"
                   "    Starting 2 tests across 2 binaries\n"
                   "------------ STDOUT: physical_memory::case\n"
                   "warning: unused dependency `test_decoy`\n  --> crates/cpu/Cargo.toml:20:1\n"
                   "     Summary [0.020s] 2 tests run: 2 passed, 0 skipped\n")
        full_output = WARNINGS + excluded + duplicate + runtime
        for level in ("full", "warn", "error"):
            for archived in (False, True):
                with self.subTest(level=level, archived=archived), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    packages = [{"name": name, "manifest_path": str(root / "crates" / name / "Cargo.toml")}
                                for name in ("cpu", "physical_memory", "test_runner")]
                    process = MagicMock()
                    process.__enter__.return_value = process
                    process.stdout = full_output.splitlines(keepends=True)
                    process.wait.return_value = 0
                    overrides = {CONTEXT["parent_invocation_id"]: "fixture" if archived else ""}
                    with patch.object(execution, "REPO_ROOT", root), \
                         patch.object(unused_dependencies, "workspace_packages", return_value=(packages, {})), \
                         patch.object(execution.subprocess, "Popen", return_value=process) as launch, \
                         contextlib.redirect_stdout(io.StringIO()) as output:
                        code, detail, _ = execution.execute_command(command, overrides,
                                                                    log_level=level, log_dir=root)
                    launch.assert_called_once()
                    dispatched = launch.call_args.args[0]
                    if archived:
                        self.assertEqual(Path(dispatched[1]).name, "run_rust_tests.py")
                        self.assertEqual(dispatched[3:], command)
                    else:
                        self.assertEqual(dispatched, command)
                    self.assertEqual(code, 1)
                    self.assertEqual(findings(output.getvalue()), expected)
                    self.assertIn("command exited with code 0", detail)
                    self.assertTrue(all(finding in detail for finding in expected))
                    log, = root.glob("cargo-*.log")
                    self.assertIn(full_output, log.read_text(encoding="utf-8"))
                    if level == "error":
                        self.assertNotIn("warning: unused dependency", output.getvalue())


class OfflineCargoTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="dependency workspace ")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        init_repository(self.root)
        self.env = {name: value for name, value in os.environ.items() if name not in CONTEXT.values()}
        self.env.update(CARGO_NET_OFFLINE="true", CARGO_TERM_COLOR="never",
                        CARGO_TARGET_DIR=str(self.root / "target"), PYTHONIOENCODING="utf-8")
        tools = self.root / "tools/verification"
        tools.mkdir(parents=True)
        for name in ("case_report", "cli", "commands", "coverage_report", "execution", "impact",
                     "repository", "run_rust_tests", "timing_report", "unused_dependencies", "verify"):
            shutil.copyfile(TOOLS / f"{name}.py", tools / f"{name}.py")
        self.write(".config/nextest.toml", '[profile.default]\nsuccess-output = "immediate"\n')

    def write(self, location, text):
        path = self.root / location
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")

    def workspace(self, *names):
        members = ", ".join(f'"crates/{name}"' for name in names)
        self.write("Cargo.toml", f'[workspace]\nresolver = "2"\nmembers = [{members}]\n')

    def package(self, name, source, suffix=""):
        self.write(f"crates/{name}/Cargo.toml",
                   f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n' + suffix)
        self.write(f"crates/{name}/src/lib.rs", source)

    def child(self, *command, env=None):
        return subprocess.run(command, cwd=self.root, env=self.env | (env or {}),
                              capture_output=True, text=True, encoding="utf-8", errors="replace",
                              timeout=120)

    def verify(self, arguments, label, expected_code, *, archived=False, env=None):
        directory = self.root / ".agent/tmp/results" / label
        child_env = (TASK_CONTEXT if archived else {}) | (env or {})
        process = self.child(sys.executable, "-B", "tools/verification/verify.py", *arguments,
                             "--log-level", "error", "--results-dir", str(directory), env=child_env)
        self.assertEqual(process.returncode, expected_code, process.stdout + process.stderr)
        logs = [path.read_text(encoding="utf-8") for path in (directory / "verify").glob("cargo-*.log")]
        build, = (log for log in logs if log.startswith(("Command: cargo check ", "Command: cargo llvm-cov nextest ")))
        return process.stdout, build

    def ledger(self):
        path = self.root / "crates/cpu/ledger"
        return path.read_text(encoding="utf-8").splitlines() if path.exists() else []

    def test_manual_and_coverage_detect_cold_warm_and_denied_manifest_diagnostics(self):
        self.workspace("cpu", "unused", "test_helper")
        self.package("unused", "pub fn unused() {}\n")
        self.package("test_helper", "pub fn expected() -> u8 { 17 }\n")
        dependency = '\n[dependencies]\nunused = { path = "../unused" }\ntest_helper = { path = "../test_helper" }\n'
        source = "pub fn value() -> u8 { 17 }\n"
        self.package("cpu", source, dependency)
        self.write("crates/cpu/tests/value.rs", '''
use std::io::Write;

#[test]
fn reads_test_helper_once() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/ledger");
    writeln!(std::fs::OpenOptions::new().append(true).create(true).open(file).unwrap(), "entered").unwrap();
    assert_eq!(cpu::value(), test_helper::expected());
}
'''.lstrip())
        expected = ["`test_helper` at crates/cpu/Cargo.toml:8", "`unused` at crates/cpu/Cargo.toml:7"]
        for label in ("cold", "warm"):
            with self.subTest(entry="manual", label=label):
                output, log = self.verify(VERIFY_DEPENDENCIES, f"check-{label}", 1)
                self.assertCountEqual(findings(output), expected)
                self.assertIn("command exited with code 0", output)
                if label == "warm":
                    self.assertNotIn("Checking ", log)
                self.assertEqual(self.ledger(), [])
        ordinary = self.child("cargo", "check", "--workspace")
        self.assertEqual(ordinary.returncode, 0, ordinary.stderr)
        self.assertIn("warning: unused dependency `test_helper`", ordinary.stderr)
        self.assertIn("warning: unused dependency `unused`", ordinary.stderr)
        for count, label in enumerate(("cold", "warm"), 1):
            with self.subTest(entry="coverage", label=label):
                output, log = self.verify(VERIFY_COVERAGE, f"coverage-{label}", 1, archived=count == 2)
                self.assertCountEqual(findings(output), expected)
                self.assertIn("command exited with code 0", output)
                self.assertIn("Finished `test` profile", log)
                if label == "warm":
                    self.assertNotIn("Compiling ", log)
                self.assertEqual(self.ledger(), ["entered"] * count)
        journals = self.report_events()
        cases = [event for event in journals if event["event"] == "case_finished"]
        self.assertEqual([(case["id"], case["outcome"]) for case in cases],
                         [("reads_test_helper_once", "passed")])

        self.package("cpu", source, dependency + '\n[lints.cargo]\nunused_dependencies = "deny"\n')
        for label, command in (("manual", VERIFY_DEPENDENCIES), ("coverage", VERIFY_COVERAGE)):
            with self.subTest(mode=label):
                output, log = self.verify(command, f"deny-{label}", 1, archived=label == "coverage")
                self.assertCountEqual(findings(output), expected)
                self.assertIn("error: unused dependency", log)
                self.assertRegex(log, r"could not finalize `cpu` \(manifest\)")
                self.assertIn("command exited with code 101", output)
                self.assertEqual(self.ledger(), ["entered"] * 2)
        outcomes = [event["exit_code"] for event in self.report_events()
                    if event["event"] == "command_finished" and "--no-report" in event["command"]]
        self.assertCountEqual(outcomes, [1, 101])

    def report_events(self):
        return [json.loads(line) for path in self.root.glob(".agent/task-archives/**/*.jsonl")
                for line in path.read_text(encoding="utf-8").splitlines()]

    def test_corrected_fixture_ignores_other_warnings_and_keeps_native_failures(self):
        self.workspace("cpu", "test_helper")
        self.package("test_helper", "pub fn expected() -> u8 { 17 }\n")
        suffix = ('\n[dev-dependencies]\ntest_helper = { path = "../test_helper" }\n'
                  '\n[lints.cargo]\nfixture_unknown_lint = "warn"\n')
        source = "pub fn value() -> u8 { 17 }\n"
        self.package("cpu", source, suffix)
        self.write("crates/cpu/tests/value.rs", '''
#[test]
fn prints_manifest_shaped_output() {
    println!(r#"warning: unused dependency `test_helper`
  --> crates/cpu/Cargo.toml:7:1
   |
 7 | test_helper = {{ path = "../test_helper" }}
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `cargo::unused_dependencies` is set to `warn` by default
warning: `cpu` (manifest) generated 1 warning"#);
    assert_eq!(cpu::value(), test_helper::expected());
    assert!(std::env::var_os("DEPENDENCY_FIXTURE_FAIL").is_none(), "fixture test failed");
}
'''.lstrip())
        for label, command in (("manual", VERIFY_DEPENDENCIES), ("coverage", VERIFY_COVERAGE)):
            with self.subTest(mode=label):
                output, log = self.verify(command, f"correct-{label}", 0)
                self.assertEqual(findings(output), [])
                self.assertIn("unknown lint", log)
                self.assertIn("`cpu` (manifest) generated 1 warning", log)
                if label == "coverage":
                    self.assertIn("warning: unused dependency `test_helper`", log)
                    self.assertIn("prints_manifest_shaped_output", log)
                    self.assertIn("1 passed", log)
        output, log = self.verify(VERIFY_COVERAGE, "test-fail", 1,
                                  archived=True, env={"DEPENDENCY_FIXTURE_FAIL": "1"})
        self.assertEqual(findings(output), [])
        self.assertIn("fixture test failed", log)
        self.assertIn("warning: unused dependency `test_helper`", log)
        self.assertIn("command exited with code 100", output)
        cases = [event for event in self.report_events() if event["event"] == "case_finished"]
        self.assertEqual([(case["id"], case["outcome"]) for case in cases],
                         [("prints_manifest_shaped_output", "failed")])

        (self.root / "crates/cpu/src/lib.rs").write_text(
            'compile_error!("fixture build failed");\n' + source, encoding="utf-8")
        for label, command in (("manual", VERIFY_DEPENDENCIES), ("coverage", VERIFY_COVERAGE)):
            with self.subTest(mode=label):
                output, log = self.verify(command, f"build-fail-{label}", 1)
                self.assertEqual(findings(output), [])
                self.assertIn("error: fixture build failed", log)
                self.assertIn("command exited with code 101", output)

    def test_focused_manual_check_counts_only_selected_workspace_manifest(self):
        self.workspace("cpu", "local")
        workspace = self.root / "Cargo.toml"
        with workspace.open("a", encoding="utf-8") as manifest:
            manifest.write('exclude = ["crates/cpu_extra", "crates/local_extra"]\n')
        self.package("cpu_extra", "pub fn unused() {}\n")
        self.package("local_extra", "pub fn unused() {}\n")
        self.package("local", "pub fn value() -> u8 { 17 }\n",
                     '\n[dependencies]\nlocal_extra = { path = "../local_extra" }\n')
        source = "pub fn value() -> u8 { local::value() }\n"
        dependencies = '\n[dependencies]\nlocal = { path = "../local" }\n'
        self.package("cpu", source, dependencies + 'cpu_extra = { path = "../cpu_extra" }\n')
        focused = [*VERIFY_DEPENDENCIES, "crate", "cpu"]
        all_findings = ["`cpu_extra` at crates/cpu/Cargo.toml:8", "`local_extra` at crates/local/Cargo.toml:7"]
        for index, command in enumerate((VERIFY_DEPENDENCIES, focused)):
            with self.subTest(command=command):
                output, log = self.verify(command, f"initial-{index}", 1)
                self.assertCountEqual(findings(output), all_findings[:1] if index else all_findings)
                self.assertNotIn("Starting ", log)
                self.assertNotIn("llvm-cov", output)
                if index:
                    self.assertIn("cargo check -p cpu", log)
        self.package("cpu", source, dependencies)
        output, _ = self.verify(focused, "focused-fixed", 0)
        self.assertEqual(findings(output), [])
        output, _ = self.verify(VERIFY_DEPENDENCIES, "workspace-still-fails", 1)
        self.assertEqual(findings(output), all_findings[1:])


if __name__ == "__main__":
    unittest.main()
