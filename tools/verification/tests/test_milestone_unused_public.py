"""Exercise the milestone command in a disposable Cargo and Git workspace."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from test_case_report import case_report, init_repository


TOOLS = Path(__file__).resolve().parents[1]


class MilestonePublicProbeTests(unittest.TestCase):
    def setUp(self):
        directory = self.enterContext(tempfile.TemporaryDirectory(prefix="milestone-probe-"))
        self.root = Path(directory).resolve()
        init_repository(self.root)
        self.write(".gitignore", ".agent/\n__pycache__/\ntarget/\n")
        self.write("Cargo.toml", '[workspace]\nmembers = ["empty", "service"]\n'
                   'default-members = ["empty"]\nresolver = "2"\n')
        for name in ("empty", "service"):
            self.write(f"{name}/Cargo.toml", f'[package]\nname = "{name}"\nversion = "0.0.0"\nedition = "2021"\n')
        self.write("empty/Cargo.toml", '[dependencies]\nservice = { path = "../service" }\n', append=True)
        self.write("empty/src/lib.rs", "fn local() { service::called(); }\n")
        self.write("service/src/lib.rs", "pub fn called() {}\n")
        self.tools = self.root / "tools/verification"
        self.tools.mkdir(parents=True)
        for script in TOOLS.glob("*.py"):
            shutil.copyfile(script, self.tools / script.name)
        self.write("tools/verification/run_singlestep.py", """
import json
import os
from pathlib import Path
import sys

results = Path(os.environ["SINGLESTEP_RESULTS_DIR"])
reports = [json.loads(path.read_text(encoding="utf-8"))
           for path in results.parent.glob("unused-public-workspace-*/*/report.json")]
with Path(".agent/singlestep.jsonl").open("a", encoding="utf-8") as log:
    log.write(json.dumps({"args": sys.argv[1:], "full": os.environ["SINGLESTEP_FULL"],
                          "limit": os.environ["SINGLESTEP_LIMIT"], "results": str(results),
                          "completed_packages": sorted(report["package"] for report in reports
                                                       if report["state"] == "completed")}) + "\\n")
raise SystemExit(int(os.environ["FIXTURE_SINGLESTEP_STATUS"]))
""")
        self.results = self.root / ".agent/results with spaces"
        self.singlestep = self.root / ".agent/singlestep.jsonl"

    def write(self, name, data, *, append=False):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a" if append else "w", encoding="utf-8") as stream:
            stream.write(data)

    def milestone(self, *, results=True, status=0):
        args = [sys.executable, "tools/verification/verify.py", "milestone"]
        if results:
            args.extend(["--results-dir", str(self.results)])
        env = {key: value for key, value in os.environ.items() if key not in case_report.CONTEXT.values()}
        env["FIXTURE_SINGLESTEP_STATUS"] = str(status)
        return subprocess.run(args, cwd=self.root, env=env, capture_output=True, text=True,
                              encoding="utf-8", errors="replace", timeout=90)

    def read_reports(self, directory):
        return {path: json.loads(path.read_text(encoding="utf-8"))
                for path in directory.glob("unused-public-workspace-*/*/report.json")}

    def test_sweep_passes_for_expected_visibility_failure_and_no_eligible_functions(self):
        run = self.milestone()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        reports = self.read_reports(self.results)
        self.assertCountEqual([report["package"] for report in reports.values()], ["empty", "service"])
        by_package = {report["package"]: report for report in reports.values()}
        self.assertEqual(by_package["empty"]["discovery"]["declarations"], [])
        batches = {package: report["contexts"]["default-features"]["batch"]
                   for package, report in by_package.items()}
        self.assertEqual(batches["empty"]["state"], "skipped")
        self.assertEqual(batches["service"]["failure_kind"], "visibility")
        self.assertTrue(all(report["candidates"] == [] for report in reports.values()))
        calls = [json.loads(line) for line in self.singlestep.read_text(encoding="utf-8").splitlines()]
        self.assertEqual(calls, [{"args": ["--profile", "verification"], "full": "1", "limit": "all",
                                  "results": str(self.results / "singlestep"),
                                  "completed_packages": ["empty", "service"]}])
        self.assertLess(run.stdout.index("[PASS] Workspace unused public Rust functions"),
                        run.stdout.index("[RUN] Full SingleStepTests"))

    def test_sweep_prints_candidates_from_successful_probes_and_stops_singlestep(self):
        self.write("empty/src/lib.rs", "pub fn empty_candidate() {}\n", append=True)
        self.write("service/src/lib.rs", "\npub fn service_candidate() {}\n", append=True)
        run = self.milestone(results=False)
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertNotIn("[RUN] Full SingleStepTests", run.stdout)
        self.assertFalse(self.singlestep.exists())
        reports = self.read_reports(self.root / ".agent/tmp/verify")
        self.assertCountEqual([report["package"] for report in reports.values()], ["empty", "service"])
        for path, report in reports.items():
            self.assertEqual(report["state"], "completed")
            package = report["package"]
            candidate, = report["candidates"]
            line = 2 if package == "empty" else 3
            self.assertEqual(candidate["name"], f"{package}_candidate")
            self.assertIn(f"{package}: candidate unused public function crate::{package}_candidate "
                          f"at {package}/src/lib.rs:{line}", run.stdout)
            self.assertIn(f"[INFO] Candidate report: {path}", run.stdout)
        self.assertIn("Check callers and re-exports for each candidate and remove confirmed unused code", run.stdout)

    def substitute_probe(self):
        # Replace probe results to exercise Verify's failure paths without breaking the host compiler.
        (self.tools / "unused_public_workspace.py").rename(self.tools / "workspace_probe_implementation.py")
        self.write("tools/verification/unused_public_workspace.py", """
import json
import workspace_probe_implementation as sweep

def probe(root, package, output, *, excluded_output):
    mode = (root / ".agent/mode").read_text(encoding="utf-8") if package == "empty" else "pass"
    report = {"package": package, "state": "completed", "candidates": [],
              "candidate": {"state": "captured"},
              "discovery": {"state": "completed", "pending_targets": [], "pending_consumer_targets": []},
              "contexts": {"default-features": {"baseline": {"state": "succeeded"},
                                               "batch": {"state": "skipped"}}}}
    path = output / "report.json"
    if mode == "absent":
        return 0
    if mode == "invalid":
        del report["candidates"]
    if mode == "partial":
        report["state"] = "partial"
        log = output / "default-features-batch.log"
        log.write_text("A genuine compiler error.", encoding="utf-8")
        report["contexts"]["default-features"]["batch"] = {"state": "failed", "failure_kind": "genuine",
                                                          "log": str(log)}
    if mode == "interrupt":
        report.update(state="interrupted", reason="Interrupted by the user.")
    path.write_text(json.dumps(report), encoding="utf-8")
    return {"interrupt": 130, "nonzero": 7}.get(mode, 0)

sweep.probe_package = probe
raise SystemExit(sweep.main())
""")

    def test_incomplete_sweep_stops_and_a_fresh_success_runs_singlestep(self):
        self.substitute_probe()
        self.write(".agent/mode", "pass")
        run = self.milestone(status=9)
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertIn("[PASS] Workspace unused public Rust functions", run.stdout)
        self.assertIn("[FAIL] Full SingleStepTests", run.stdout)
        self.assertIn("command exited with code 9", run.stdout)
        original_reports = self.read_reports(self.results)
        self.assertCountEqual([report["package"] for report in original_reports.values()], ["empty", "service"])

        failures = {"partial": "default-features batch failed", "interrupt": "Interrupted by the user",
                    "absent": "cannot use public probe report", "invalid": "missing or mismatched package",
                    "nonzero": "public probe completed (exit code 7)"}
        for mode, error in failures.items():
            with self.subTest(mode=mode):
                self.write(".agent/mode", mode)
                existing = set(self.results.glob("unused-public-workspace-*"))
                run = self.milestone()
                self.assertNotEqual(run.returncode, 0, run.stdout + run.stderr)
                self.assertIn(error, run.stdout)
                self.assertNotIn("[RUN] Full SingleStepTests", run.stdout)
                fresh, = set(self.results.glob("unused-public-workspace-*")) - existing
                self.assertEqual(len(list(fresh.glob("service-*/report.json"))), int(mode != "interrupt"))
                if mode == "partial":
                    log, = fresh.glob("empty-*/*-batch.log")
                    self.assertIn(f"[INFO] Diagnostic log: {log}", run.stdout)

        self.assertEqual(len(self.singlestep.read_text(encoding="utf-8").splitlines()), 1)
        self.write(".agent/mode", "pass")
        run = self.milestone()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertIn("[PASS] Full SingleStepTests", run.stdout)
        self.assertEqual(len(self.singlestep.read_text(encoding="utf-8").splitlines()), 2)
        for path, report in original_reports.items():
            self.assertEqual(json.loads(path.read_text(encoding="utf-8")), report)
