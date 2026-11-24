"""Offline workspace contracts for the on-demand public declaration probe."""

import contextlib
from hashlib import sha256
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from test_case_report import git_fixture, init_repository

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
from case_report import CONTEXT
import unused_public
from worker_processes import WorkerProcesses


class PublicProbeTests(unittest.TestCase):
    def setUp(self):
        temporary_root = TOOLS.parents[1] / ".agent/tmp"
        temporary_root.mkdir(parents=True, exist_ok=True)
        directory = self.enterContext(tempfile.TemporaryDirectory(prefix="public-fixture-", dir=temporary_root))
        self.root = Path(directory) / "live"
        self.results = Path(directory) / "results"
        self.root.mkdir()
        init_repository(self.root)
        self.write(".gitignore", "target/\n__pycache__/\n.agent/\n")
        self.write("Cargo.toml", '[workspace]\nmembers = ["cpu", "client"]\nresolver = "2"\n')
        self.write("cpu/Cargo.toml", '[package]\nname = "cpu"\nversion = "0.0.0"\nedition = "2021"\n')
        self.write("client/Cargo.toml", '[package]\nname = "client"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[dependencies]\ncpu = { path = "../cpu" }\n')
        self.write("client/src/lib.rs", "")

    def write(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data.encode("utf-8"))

    def read_report(self):
        reports = list(self.results.glob("unused-public-*/report.json"))
        latest = set(reports) - getattr(self, "seen", set())
        self.seen = set(reports)
        report, = latest
        return json.loads(report.read_text(encoding="utf-8"))

    def probe(self, code=0):
        with contextlib.redirect_stdout(io.StringIO()) as log:
            actual = unused_public.run_probe(self.root, "cpu", self.results)
        self.assertEqual(actual, code, log.getvalue())
        return self.read_report()

    def verify(self, build_record=None):
        destination = self.root / "tools/verification"
        destination.mkdir(parents=True)
        for file in TOOLS.glob("*.py"):
            shutil.copyfile(file, destination / file.name)
        if build_record:
            self.record_build_sources(destination, build_record)
        env = {key: value for key, value in os.environ.items() if key not in CONTEXT.values()}
        process = subprocess.run([sys.executable, "tools/verification/verify.py", "check", "unused",
                                  "public", "crate", "cpu", "--results-dir", str(self.results)],
                                 cwd=self.root, env=env, capture_output=True, text=True, encoding="utf-8",
                                 errors="replace", timeout=90)
        self.assertEqual(process.returncode, 0, process.stdout + process.stderr)
        return self.read_report()

    def record_build_sources(self, scripts, records):
        # Observe the bytes handed to Cargo at the process boundary in this fixture only.
        (scripts / "unused_public.py").rename(scripts / "unused_public_implementation.py")
        self.write("tools/verification/unused_public.py", f"""import json
from pathlib import Path
import unused_public_implementation as probe

real_build = probe.run_build

def record(workspace, environment, command, output, observation):
    files = {{path.relative_to(workspace).as_posix(): path.read_bytes().hex()
              for path in workspace.glob("*/*/*.rs")}}
    with Path({str(records)!r}).open("a", encoding="utf-8") as log:
        log.write(json.dumps(files) + "\\n")
    return real_build(workspace, environment, command, output, observation)

probe.run_build = record
raise SystemExit(probe.main())
""")

    def publications(self, report, context="default-features"):
        return {(item["module"], item["name"]): item["comparisons"][context]
                for item in report["discovery"]["declarations"]}

    def privacy_sites(self, result):
        return {(site["kind"], site["span"]["path"], site["span"]["line_start"],
                 observation["diagnostic"]["code"])
                for observation in result["observations"] if observation["kind"] == "privacy"
                for site in observation["referring_sites"]}

    def test_package_discovery_edits_one_token_and_reports_direct_and_reexport_sites(self):
        source = ('// pub fn comment() {}\n'
                  'const RAW: &str = r#"pub fn string() {}"#;\n'
                  'pub fn called() { let _ = RAW; } pub fn exported() {}\n'
                  'pub mod group { pub fn helper() {} }\n'
                  'pub mod left { pub fn same() {} } pub mod right { pub fn same() {} }\n'
                  'pub struct Point { pub x: u8 }\n')
        consumer = ('pub fn consumer() { cpu::called(); let p = cpu::Point { x: 1 }; let _ = p.x; }\n'
                    'pub use cpu::exported;\n')
        integration = '#[test] fn smoke() { cpu::left::same(); }\npub fn test_only() {}\n'
        self.write("cpu/src/lib.rs", source)
        self.write("client/src/lib.rs", consumer)
        self.write("cpu/tests/smoke.rs", integration)
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records)
        tool_inputs = {"cpu/src/lib.rs": source, "client/src/lib.rs": consumer,
                       "cpu/tests/smoke.rs": integration}
        expected = [
            ("crate", "called", "pub fn called"), ("crate", "exported", "pub fn exported"),
            ("crate", "group", "pub mod group"), ("crate::group", "helper", "pub fn helper"),
            ("crate", "left", "pub mod left"), ("crate::left", "same", "pub fn same"),
            ("crate", "right", "pub mod right"), ("crate::right", "same", "pub fn same"),
            ("crate", "Point", "pub struct Point"), ("crate", "x", "pub x"),
        ]
        declarations = report["discovery"]["declarations"]
        self.assertEqual([(item["module"], item["name"]) for item in declarations],
                         [(module, name) for module, name, _ in expected])
        starts = [index for index in range(len(source)) if source.startswith("pub fn same", index)]
        positions = [starts.pop(0) if token == "pub fn same" else source.index(token)
                     for _, _, token in expected]
        builds = [json.loads(line) for line in records.read_text(encoding="utf-8").splitlines()]
        self.assertEqual(len(builds), len(expected) + 1)
        for offset, build in zip([None, *positions], builds):
            files = dict(tool_inputs)
            if offset is not None:
                files["cpu/src/lib.rs"] = source[:offset] + "pub(crate)" + source[offset + 3:]
            self.assertEqual(build, {name: data.encode().hex() for name, data in files.items()})
        self.assertEqual([item["visibility_span"]["byte_start"] for item in declarations], positions)
        outcomes = self.publications(report)
        self.assertEqual(self.privacy_sites(outcomes["crate", "called"]),
                         {("direct reference", "client/src/lib.rs", 1, "E0603")})
        self.assertEqual(self.privacy_sites(outcomes["crate", "exported"]),
                         {("re-export", "client/src/lib.rs", 2, "E0603")})
        self.assertEqual(self.privacy_sites(outcomes["crate", "x"]),
                         {("direct reference", "client/src/lib.rs", 1, "E0616")})
        self.assertEqual({(row["item"]["module"], row["item"]["name"], row["diagnostic"]["code"])
                          for row in outcomes["crate", "group"]["observations"]},
                         {("crate::group", "helper", "dead_code")})

    def test_default_and_enabled_features_report_inactive_silent_and_consumers(self):
        self.write("cpu/Cargo.toml", '[package]\nname = "cpu"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[features]\nextra = []\n')
        self.write("client/Cargo.toml", '[package]\nname = "client"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[dependencies]\ncpu = { path = "../cpu" }\n[features]\nextra = ["cpu/extra"]\n')
        self.write("cpu/src/lib.rs", 'pub fn silent() {} pub fn integration_only() {} pub fn extra_call() {}\n'
                   'pub fn internal() { silent(); integration_only(); extra_call(); }\n'
                   '#[cfg(feature = "extra")] pub fn extra_only() {}\n')
        self.write("client/src/lib.rs", '#[cfg(feature = "extra")] pub fn consumer() { cpu::extra_call(); }\n')
        self.write("cpu/tests/use_export.rs", '#[test] fn integration() { cpu::integration_only(); }\n')
        report = self.verify()
        default = self.publications(report)
        all_features = self.publications(report, "all-features")
        self.assertEqual(set(report["contexts"]), {"default-features", "all-features"})
        self.assertTrue(report["omitted_contexts"])
        for name, context in report["contexts"].items():
            self.assertEqual(context["baseline"]["state"], "succeeded")
            self.assertTrue(Path(context["baseline"]["log"]).is_file())
            self.assertEqual(len(list(Path(context["baseline"]["log"]).parent.glob(f"{name}-baseline*"))), 1)
            for item in report["discovery"]["declarations"]:
                self.assertEqual((item["original_visibility"], item["probed_visibility"]), ("pub", "pub(crate)"))
                self.assertEqual(item["comparisons"][name]["baseline_context"], name)
        self.assertEqual(default["crate", "silent"]["state"], "inconclusive")
        self.assertEqual(default["crate", "extra_call"]["state"], "inconclusive")
        self.assertEqual(default["crate", "extra_only"]["state"], "inactive")
        self.assertNotIn("build", default["crate", "extra_only"])
        self.assertEqual(all_features["crate", "extra_only"]["state"], "observed")
        self.assertEqual(self.privacy_sites(default["crate", "integration_only"]),
                         {("direct reference", "cpu/tests/use_export.rs", 1, "E0603")})
        self.assertEqual(self.privacy_sites(all_features["crate", "extra_call"]),
                         {("direct reference", "client/src/lib.rs", 1, "E0603")})

    def test_macro_gap_baseline_failure_and_probe_error_preserve_separate_outcomes(self):
        self.write("cpu/Cargo.toml", '[package]\nname = "cpu"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[features]\nextra = []\n')
        self.write("cpu/src/lib.rs", '#![deny(dead_code)]\n'
                   'macro_rules! generate { () => { pub fn generated() {} } }\ngenerate!();\n'
                   'pub fn stops() {} pub fn usable() { let _ = restricted(); }\n'
                   'pub(crate) fn restricted() {}\n'
                   '#[cfg(feature = "extra")] compile_error!("baseline fixture");\n')
        self.write("client/src/lib.rs", "pub fn calls() { cpu::stops(); }\n")
        report = self.probe(code=1)
        discovered = report["discovery"]["declarations"]
        self.assertEqual([item["name"] for item in discovered], ["stops", "usable"])
        self.assertEqual({item["visibility_span"]["line"] for item in discovered}, {4})
        self.assertEqual({item["visibility_span"]["byte_column"] for item in discovered}, {1, 19})
        self.assertIn((3, "generate!"), {(gap["span"]["line"], gap["reason"].split()[0])
                                        for gap in report["discovery"]["gaps"]})
        self.assertEqual(report["contexts"]["all-features"]["baseline"]["state"], "failed")
        self.assertTrue(all(item["comparisons"]["all-features"]["state"] == "baseline blocked"
                            for item in discovered))
        stops = self.publications(report)["crate", "stops"]
        self.assertEqual(stops["state"], "observed")
        self.assertEqual(stops["build"]["state"], "failed")
        self.assertTrue(any(row["kind"] == "dead_code" and row["diagnostic"]["level"] == "error"
                            for row in stops["observations"]))
        baseline_log = Path(report["contexts"]["default-features"]["baseline"]["log"])
        with baseline_log.open(encoding="utf-8") as log:
            messages = (json.loads(line) for line in log if line.startswith("{"))
            client_ids = {message["package_id"] for message in messages
                          if message["reason"] == "compiler-artifact" and message["target"]["name"] == "client"}
        self.assertEqual(len(client_ids), 1, client_ids)
        client_targets = [(target["target"], target["test"], target["state"])
                          for target in stops["unfinished_targets"] if target["package_id"] in client_ids]
        library = {"name": "client", "kind": ["lib"]}
        self.assertCountEqual(client_targets, [
            (library, False, "no compiler output observed"),
            (library, True, "no compiler output observed"),
        ])

    def test_live_index_and_working_copy_survive_success_failure_and_interruptions(self):
        path = "cpu/src/lib.rs"
        staged = "pub fn index_only() {}\n"
        working = "pub fn working() { untracked::reached(); }\npub fn followup() {}\nmod untracked;\n"
        self.write(path, staged)
        git_fixture(self.root, "add", path)
        self.write(path, working)
        self.write("cpu/src/untracked.rs", "pub fn reached() {}\n")
        index = (self.root / ".git/index").read_bytes()
        source = (self.root / path).read_bytes()
        extra = (self.root / "cpu/src/untracked.rs").read_bytes()

        def unchanged():
            self.assertEqual((self.root / ".git/index").read_bytes(), index)
            self.assertEqual((self.root / path).read_bytes(), source)
            self.assertEqual((self.root / "cpu/src/untracked.rs").read_bytes(), extra)
            self.assertFalse((self.root / "target").exists())
            self.assertFalse((self.root / "Cargo.lock").exists())

        report = self.probe()
        self.assertEqual(set(self.publications(report)), {("crate", "working"), ("crate", "followup"),
                                                          ("crate::untracked", "reached")})
        selections = {value["path"]: value for value in report["candidate"]["working_copy_selections"]}
        self.assertTrue(selections[path]["differs_from_index"])
        self.assertEqual(selections[path]["sha256"], sha256(source).hexdigest())
        self.assertTrue(selections["cpu/src/untracked.rs"]["untracked"])
        unchanged()

        self.write("client/src/lib.rs", 'compile_error!("baseline fixture");\n')
        failure = self.probe(code=1)
        self.assertEqual(failure["state"], "partial")
        unchanged()
        self.write("client/src/lib.rs", "")

        # Interrupt the real build launcher at the selected attempt, leaving other builds to Cargo.
        original_start = WorkerProcesses.start
        for stop_at, phase, states in [(1, "baseline", ["pending", "pending", "pending"]),
                                       (3, "comparison", ["observed", "interrupted", "pending"])]:
            with self.subTest(phase=phase):
                attempts = []

                def interrupt(owner, command, log, environment, cwd):
                    attempts.append(cwd)
                    if len(attempts) == stop_at:
                        isolated = Path(environment["CARGO_TARGET_DIR"])
                        self.assertEqual(isolated.parent, cwd.parent)
                        self.assertFalse(cwd.is_relative_to(self.root))
                        raise KeyboardInterrupt
                    return original_start(owner, command, log, environment, cwd)

                with patch("unused_public_cargo.WorkerProcesses.start", new=interrupt):
                    interrupted = self.probe(code=130)
                self.assertEqual(interrupted["active"]["phase"], phase)
                self.assertEqual([outcome["state"] for outcome in self.publications(interrupted).values()], states)
                self.assertFalse(attempts[-1].exists())
                unchanged()


if __name__ == "__main__":
    unittest.main()
