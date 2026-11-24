"""Offline workspace contracts for the on-demand public function probe."""

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
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from test_case_report import git_fixture, init_repository

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
from case_report import CONTEXT
import unused_public
from unused_public_syntax import Discovery
from unused_public_workspace import build_failures, check_report
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

    def verify(self, build_record, *, code=0):
        destination = self.root / "tools/verification"
        destination.mkdir(parents=True)
        for file in TOOLS.glob("*.py"):
            shutil.copyfile(file, destination / file.name)
        self.record_build_sources(destination, build_record)
        env = {key: value for key, value in os.environ.items() if key not in CONTEXT.values()}
        process = subprocess.run([sys.executable, "tools/verification/verify.py", "check", "unused",
                                  "public", "crate", "cpu", "--results-dir", str(self.results)],
                                 cwd=self.root, env=env, capture_output=True, text=True, encoding="utf-8",
                                 errors="replace", timeout=90)
        report = self.read_report()
        if process.returncode != code:
            diagnostics = [
                (name, phase, row["message"])
                for name, context in report["contexts"].items()
                for phase, build in (("baseline", context["baseline"]), ("batch", context["batch"]),
                                     ("retry", context.get("retry", {}).get("batch", {})))
                for row in build.get("diagnostics", [])
            ]
            self.fail(f"{process.stdout}{process.stderr}\nCompiler diagnostics: {diagnostics}")
        return report

    def record_build_sources(self, scripts, records):
        # Observe the bytes handed to Cargo at the process boundary in this fixture only.
        (scripts / "unused_public.py").rename(scripts / "unused_public_implementation.py")
        self.write("tools/verification/unused_public.py", f"""import json
from pathlib import Path
import unused_public_implementation as probe

real_build = probe.run_build

def record(workspace, environment, command, output, observation):
    files = {{path.relative_to(workspace).as_posix(): path.read_bytes().hex()
              for path in workspace.rglob("*.rs")}}
    with Path({str(records)!r}).open("a", encoding="utf-8") as log:
        log.write(json.dumps({{"files": files, "command": command}}) + "\\n")
    return real_build(workspace, environment, command, output, observation)

probe.run_build = record
raise SystemExit(probe.main())
""")

    def declarations(self, report):
        return {(item["module"], item["name"]): item
                for item in report["discovery"]["declarations"]}

    def comparisons(self, report, context="default-features"):
        return {key: item["comparisons"][context] for key, item in self.declarations(report).items()}

    def privacy_sites(self, report, key, context="default-features"):
        result = self.comparisons(report, context)[key]
        diagnostics = report["contexts"][context]["batch"]["diagnostics"]
        return {(site["kind"], site["span"]["path"], site["span"]["byte_start"], site["span"]["byte_end"],
                 diagnostics[observation["diagnostic_index"]]["code"])
                for observation in result["observations"] if observation["kind"] == "privacy"
                for site in observation["referring_sites"]}

    def assert_reference(self, report, key, context, kind, path, source, call, name, code):
        data = source.encode()
        start = data.index(call.encode()) + call.encode().rindex(name.encode())
        self.assertEqual(self.privacy_sites(report, key, context), {
            (kind, path, start, start + len(name.encode()), code),
        })

    def assert_builds(self, records, expected):
        builds = [json.loads(line) for line in records.read_text(encoding="utf-8").splitlines()]
        self.assertEqual(len(builds), len(expected))
        for build, (files, flags) in zip(builds, expected, strict=True):
            self.assertEqual(build["files"], {path: source.encode().hex() for path, source in files.items()})
            command = build["command"]
            self.assertEqual(command[:2], ["cargo", "check"])
            self.assertIn("--keep-going", command)
            self.assertIn("--all-targets", command)
            self.assertIn("--offline", command)
            self.assertEqual(set(command) & {"--all-features", "--no-default-features"}, set(flags))

    def assert_edits(self, context, path, source, tokens):
        actual = {(span["path"], span["byte_start"], span["byte_end"]) for span in context["edits"]}
        starts = [source.encode().index(token.encode()) for token in tokens]
        self.assertEqual(actual, {(path, start, start + 3) for start in starts})
        self.assertEqual(len(context["edits"]), len(tokens))

    def test_serde_field_inventory_keeps_original_token_and_reports_unknown_attributes(self):
        path = "cpu/src/lib.rs"
        source = ('#[derive(serde::Serialize, serde::Deserialize)]\n'
                  'pub struct Snapshot {\n'
                  '    #[serde(with = "codec")]\n'
                  '    pub value: u8,\n'
                  '    #[unknown]\n'
                  '    pub ambiguous_field: u8,\n'
                  '}\n'
                  '#[serde(with = "codec")]\n'
                  'pub fn ambiguous_item() {}\n')
        self.write(path, source)
        report = {"gaps": []}
        discovery = Discovery(self.root, "cpu", report)
        discovery.discover({"name": "cpu", "kind": ["lib"], "src_path": str(self.root / path)})
        edits = discovery.edits()

        self.assertEqual([item["name"] for item in report["declarations"]], ["Snapshot", "value"])
        token = source.encode().index(b"pub value")
        field = edits[(path, token)]
        self.assertEqual((field.kind, field.enclosing), ("field", ("Snapshot",)))
        self.assertEqual(field.visibility["line"], 4)
        self.assertEqual(field.visibility["byte_end"], token + 3)
        self.assertEqual(source.encode()[field.visibility["byte_start"]:field.visibility["byte_end"]], b"pub")
        self.assertEqual((self.root / path).read_bytes(), source.encode())
        self.assertEqual([gap["span"]["line"] for gap in report["gaps"]
                          if gap["reason"] == "An attribute can transform the item before compilation."], [5, 8])

    def test_spans_follow_original_file_bytes_on_long_crlf_source(self):
        path = "cpu/src/lib.rs"
        header = "// padding\r\n" * 260
        source = header + "/* ż */ pub fn sample() {}\r\n"
        self.write(path, source)
        report = {"gaps": []}
        discovery = Discovery(self.root, "cpu", report)
        discovery.discover({"name": "cpu", "kind": ["lib"], "src_path": str(self.root / path)})
        discovery.edits()

        item, = report["declarations"]
        data = source.encode("utf-8")
        for field, token, column in (
                ("span", b"pub fn sample() {}", 10),
                ("visibility_span", b"pub", 10),
                ("name_span", b"sample", 17)):
            with self.subTest(span=field):
                start = data.index(token)
                self.assertEqual(item[field], {
                    "path": path, "line": 261, "byte_column": column,
                    "byte_start": start, "byte_end": start + len(token),
                })

    def test_one_batch_edits_exact_function_tokens_and_prioritizes_external_references(self):
        path = "cpu/src/lib.rs"
        consumer_path = "client/src/lib.rs"
        header = ('#![deny(dead_code)]\n'
                  '#[cfg(not(public_probe_fixture))] compile_error!("lost the fixture compiler flag");\n'
                  '// ż pub fn comment() {}\n'
                  'const RAW: &str = r#"pub fn string() {}"#;\n')
        unchanged = ('mod api { pub fn kept() {} }\n'
                     'pub use api::kept as public_kept;\n'
                     'pub struct Point { pub x: u8 }\n')
        called = 'fn called() { let _ = RAW; pub const NEVER: u8 = 0; fn nested() {} } '
        source = (header + 'pub ' + called + 'pub fn exported() {} pub fn spare() {}\n'
                  'pub mod left { pub fn same() {} } pub mod right { pub fn same() { let _ = 1; } }\n'
                  + unchanged + 'impl Point { pub fn used_method(&self) {} '
                  'pub fn spare_method(&self) {} pub fn tail_method(&self) {} }\n')
        edited = (header + 'pub(crate) ' + called + 'pub(crate) fn exported() {} pub(crate) fn spare() {}\n'
                  'pub mod left { pub(crate) fn same() {} } pub mod right { pub(crate) fn same() { let _ = 1; } }\n'
                  + unchanged + 'impl Point { pub(crate) fn used_method(&self) {} '
                  'pub(crate) fn spare_method(&self) {} pub(crate) fn tail_method(&self) {} }\n')
        consumer = ('pub fn consumer() { cpu::called(); cpu::Point { x: 1 }.used_method(); }\n'
                    'pub use cpu::exported;\n'
                    'use cpu::left::same;\n'
                    'pub fn imported() { same(); }\n')
        self.write(path, source)
        self.write(consumer_path, consumer)
        self.write(".cargo/config.toml", "[target.'cfg(all())']\n"
                   "rustflags = ['--cfg', 'public_probe_fixture', '--check-cfg=cfg(public_probe_fixture)']\n")
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records)
        name = "default-features"
        pristine_files = {path: source, consumer_path: consumer}
        self.assert_builds(records, [(pristine_files, ()), ({**pristine_files, path: edited}, ())])
        context = report["contexts"][name]
        self.assert_edits(context, path, source, [
            "pub fn called", "pub fn exported", "pub fn spare()", "pub fn same() {}",
            "pub fn same() { let", "pub fn used_method", "pub fn spare_method", "pub fn tail_method",
        ])
        self.assertEqual(context["source_sha256"][path], {
            "pristine": sha256(source.encode()).hexdigest(), "probed": sha256(edited.encode()).hexdigest(),
        })
        self.assertEqual(context["baseline"]["state"], "succeeded")
        self.assertTrue(any(row["target"]["name"] == "cpu" and not row["test"]
                            for row in context["compiled_targets"]))
        self.assertTrue(any(row["target"]["name"] == "client" and row["state"] == "diagnostics emitted"
                            for row in context["unfinished_targets"]))
        self.assertEqual(report["state"], "completed")

        items = self.declarations(report)
        self.assertEqual(items["crate::api", "kept"]["state"], "re-exported")
        reexport = b"pub use api::kept as public_kept;"
        reexport_start = source.encode().index(reexport)
        locations = items["crate::api", "kept"]["producer_re_exports"]
        self.assertEqual([(span["path"], span["byte_start"], span["byte_end"]) for span in locations],
                         [(path, reexport_start, reexport_start + len(reexport))])
        for key in (("crate", "Point"), ("crate", "x"), ("crate", "left"), ("crate", "NEVER")):
            self.assertEqual(items[key]["state"], "outside function scan")
        self.assertNotIn(("crate", "nested"), items)

        for key, kind, call, symbol, code in [
            (("crate", "called"), "direct reference", "cpu::called()", "called", "E0603"),
            (("crate", "exported"), "re-export", "pub use cpu::exported", "exported", "E0603"),
            (("crate::left", "same"), "import", "use cpu::left::same", "same", "E0603"),
            (("crate", "used_method"), "direct reference", ".used_method()", "used_method", "E0624"),
        ]:
            self.assert_reference(report, key, name, kind, consumer_path, consumer, call, symbol, code)

        outcomes = self.comparisons(report)
        for key in (("crate", "called"), ("crate", "exported"), ("crate::left", "same"), ("crate", "used_method")):
            self.assertEqual({row["kind"] for row in outcomes[key]["observations"]}, {"dead_code", "privacy"})
        candidate_tokens = {
            ("crate", "spare"): b"pub fn spare() {}",
            ("crate::right", "same"): b"pub fn same() { let _ = 1; }",
            ("crate", "spare_method"): b"pub fn spare_method(&self) {}",
            ("crate", "tail_method"): b"pub fn tail_method(&self) {}",
        }
        candidates = {(item["module"], item["name"]): (
            item["span"]["path"], item["span"]["byte_start"], item["span"]["byte_end"],
        ) for item in report["candidates"]}
        expected_spans = {key: (path, source.encode().index(token), source.encode().index(token) + len(token))
                          for key, token in candidate_tokens.items()}
        self.assertEqual(candidates, expected_spans)
        self.assertEqual({(item["module"], item["name"]) for item in report["confirmed_references"]}, {
            ("crate", "called"), ("crate", "exported"), ("crate::left", "same"),
            ("crate", "used_method"), ("crate::api", "kept"),
        })

        diagnostics = context["batch"]["diagnostics"]
        self.assertFalse(any("NEVER" in row["message"] or "`nested`" in row["message"] for row in diagnostics))
        grouped = {index for index, row in enumerate(diagnostics) if row["code"] == "dead_code"
                   and all(f"`{method}`" in row["message"]
                           for method in ("used_method", "spare_method", "tail_method"))}
        if grouped:
            for method in ("used_method", "spare_method", "tail_method"):
                self.assertEqual({row["diagnostic_index"] for row in outcomes["crate", method]["observations"]
                                  if row["kind"] == "dead_code"}, grouped)

    def test_grouped_producer_re_exports_preserve_public_functions_and_reach_the_consumer(self):
        path = "cpu/src/lib.rs"
        consumer_path = "client/src/lib.rs"
        re_export = "pub use phase::{fill, /* exported alias */ evaluate as eval, Word,};"
        source = ('#![deny(dead_code)]\n'
                  'pub mod blitter {\n'
                  '    mod phase { pub fn fill() {} pub fn evaluate() {} pub type Word = u16; }\n'
                  '    pub mod sibling { pub fn fill() {} }\n'
                  f'    {re_export}\n'
                  '}\n')
        edited = ('#![deny(dead_code)]\n'
                  'pub mod blitter {\n'
                  '    mod phase { pub fn fill() {} pub fn evaluate() {} pub type Word = u16; }\n'
                  '    pub mod sibling { pub(crate) fn fill() {} }\n'
                  f'    {re_export}\n'
                  '}\n')
        consumer = "pub fn consume() { cpu::blitter::fill(); cpu::blitter::eval(); }\n"
        self.write(path, source)
        self.write(consumer_path, consumer)
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records)
        self.assert_builds(records, [
            ({path: source, consumer_path: consumer}, ()), ({path: edited, consumer_path: consumer}, ()),
        ])
        context = report["contexts"]["default-features"]
        self.assertEqual(context["batch"]["state"], "succeeded")
        self.assertTrue(any(row["target"]["name"] == "client" and not row["test"]
                            for row in context["compiled_targets"]))

        items = self.declarations(report)
        start = source.encode().index(re_export.encode())
        for name in ("fill", "evaluate"):
            function = items["crate::blitter::phase", name]
            spans = function["producer_re_exports"]
            self.assertEqual(function["state"], "re-exported")
            self.assertEqual([(span["path"], span["byte_start"], span["byte_end"]) for span in spans],
                             [(path, start, start + len(re_export.encode()))])
        self.assertEqual(items["crate::blitter::phase", "Word"]["state"], "outside function scan")
        self.assertEqual({(item["module"], item["name"]) for item in report["candidates"]},
                         {("crate::blitter::sibling", "fill")})

    def test_local_glob_privacy_identifies_grouped_imports_and_qualified_calls(self):
        path = "cpu/src/lib.rs"
        integration_path = "cpu/tests/glob_callers.rs"
        source = ('pub mod micro {\n'
                  '    pub mod types { pub fn data_fc() {} pub fn prog_fc() {} }\n'
                  '    pub use types::*;\n'
                  '}\n'
                  'pub mod mfm { pub fn encode() {} }\n'
                  'pub use mfm::*;\n'
                  'pub mod sibling { pub fn data_fc() {} }\n')
        integration = ('use cpu::micro::{data_fc, prog_fc};\n'
                       '#[test] fn read() { data_fc(); prog_fc(); cpu::encode(); }\n')
        self.write(path, source)
        self.write(integration_path, integration)
        report = self.probe()
        name = "default-features"
        context = report["contexts"][name]
        self.assertEqual(report["state"], "completed")
        self.assertEqual(context["batch"]["failure_kind"], "visibility")
        self.assertEqual(context["genuine_error_indices"], [])
        self.assert_edits(context, path, source, [
            "pub fn data_fc() {} pub fn prog_fc", "pub fn prog_fc",
            "pub fn encode", "pub fn data_fc() {} }\n",
        ])

        for module, item, site, call, glob_line in [
            ("crate::micro::types", "data_fc", "import", "use cpu::micro::{data_fc", 3),
            ("crate::micro::types", "prog_fc", "import", "use cpu::micro::{data_fc, prog_fc", 3),
            ("crate::mfm", "encode", "direct reference", "cpu::encode()", 6),
        ]:
            self.assert_reference(report, (module, item), name, site, integration_path, integration, call, item,
                                  "E0603")
            result = self.comparisons(report)[module, item]
            self.assertEqual({row["kind"] for row in result["observations"]}, {"dead_code", "privacy"})
            index, = [row["diagnostic_index"] for row in result["observations"] if row["kind"] == "privacy"]
            native = context["batch"]["diagnostics"][index]
            self.assertIn((path, glob_line), {
                (span["path"], span["line_start"]) for note in native["children"]
                if note["message"] == f"the function `{item}` is defined here" for span in note["spans"]
            })
        self.assertEqual({(item["module"], item["name"]) for item in report["candidates"]},
                         {("crate::sibling", "data_fc")})

    def test_glob_through_an_unresolved_module_alias_preserves_the_native_failure(self):
        self.write("cpu/src/lib.rs", "pub mod implementation { pub fn call() {} }\n"
                   "use self::implementation as bridge;\npub use bridge::*;\n"
                   "pub mod sibling { pub fn call() {} }\n")
        self.write("cpu/tests/glob_callers.rs", "#[test] fn read() { cpu::call(); }\n")
        report = self.probe(code=1)
        context = report["contexts"]["default-features"]
        self.assertEqual(report["state"], "partial")
        self.assertEqual(context["batch"]["failure_kind"], "genuine")
        privacy, = [index for index, row in enumerate(context["batch"]["diagnostics"]) if row["code"] == "E0603"]
        self.assertIn(privacy, context["genuine_error_indices"])
        self.assertIn(privacy, context["unresolved_diagnostic_indices"])
        self.assertEqual(report["confirmed_references"], [])

    def test_feature_contexts_keep_silent_results_and_report_integration_and_enabled_callers(self):
        self.write("cpu/Cargo.toml", '[package]\nname = "cpu"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[features]\nextra = []\n')
        self.write("client/Cargo.toml", '[package]\nname = "client"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[dependencies]\ncpu = { path = "../cpu" }\n[features]\nextra = ["cpu/extra"]\n')
        path = "cpu/src/lib.rs"
        consumer_path = "client/src/lib.rs"
        integration_path = "cpu/tests/use_export.rs"
        keep = ('mod bridge { pub fn internal() { super::silent(); } }\n'
                'pub use crate::bridge::internal;\n')
        feature = '#[cfg(feature = "extra")] '
        source = ('pub fn silent() {} pub fn integration_only() {} pub fn extra_call() {}\n'
                  + keep + feature + 'pub fn extra_only() {}\n')
        narrowed = ('pub(crate) fn silent() {} pub(crate) fn integration_only() {} pub(crate) fn extra_call() {}\n'
                    + keep)
        consumer = (feature + 'pub fn consumer() { cpu::extra_call(); }\n')
        integration = ('#[test] fn integration() { cpu::integration_only(); }\n'
                       'pub fn test_only() {}\n')
        self.write(path, source)
        self.write(consumer_path, consumer)
        self.write(integration_path, integration)
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records)
        pristine = {path: source, consumer_path: consumer, integration_path: integration}
        self.assert_builds(records, [
            (pristine, ()),
            ({**pristine, path: narrowed + feature + 'pub fn extra_only() {}\n'}, ()),
            (pristine, ("--all-features",)),
            ({**pristine, path: narrowed + feature + 'pub(crate) fn extra_only() {}\n'}, ("--all-features",)),
        ])
        default = self.comparisons(report)
        all_features = self.comparisons(report, "all-features")
        self.assertEqual(set(report["contexts"]), {"default-features", "all-features"})
        self.assertTrue(any("Target triples" in item for item in report["omitted_contexts"]))
        self.assertTrue(any("Individual feature combinations" in item for item in report["omitted_contexts"]))
        self.assertTrue(any("external consumers" in item for item in report["omitted_contexts"]))
        edit_tokens = ["pub fn silent", "pub fn integration_only", "pub fn extra_call"]
        for name, context in report["contexts"].items():
            self.assertEqual(context["baseline"]["state"], "succeeded")
            self.assertTrue(Path(context["baseline"]["log"]).is_file())
            self.assertTrue(Path(context["batch"]["log"]).is_file())
            tokens = edit_tokens if name == "default-features" else [*edit_tokens, "pub fn extra_only"]
            self.assert_edits(context, path, source, tokens)
            self.assertFalse(any(item["name"] == "test_only" for item in report["discovery"]["declarations"]))
        self.assertEqual(self.declarations(report)["crate::bridge", "internal"]["state"], "re-exported")
        self.assertEqual(default["crate", "silent"]["state"], "inconclusive")
        self.assertEqual(default["crate", "extra_only"]["state"], "inactive")
        self.assertEqual(default["crate", "silent"]["observations"], [])
        self.assertEqual(all_features["crate", "silent"]["state"], "inconclusive")
        self.assertEqual({row["kind"] for row in default["crate", "extra_call"]["observations"]}, {"dead_code"})
        self.assertEqual({row["kind"] for row in all_features["crate", "extra_only"]["observations"]},
                         {"dead_code"})
        self.assert_reference(report, ("crate", "integration_only"), "default-features", "direct reference",
                              integration_path, integration, "cpu::integration_only()", "integration_only", "E0603")
        self.assert_reference(report, ("crate", "extra_call"), "all-features", "direct reference",
                              consumer_path, consumer, "cpu::extra_call()", "extra_call", "E0603")
        self.assertEqual({(item["module"], item["name"]) for item in report["candidates"]},
                         {("crate", "extra_only")})
        self.assertEqual(report["candidates"][0]["incomplete_contexts"], ["default-features", "all-features"])

    def test_discovery_gaps_and_genuine_producer_error_keep_consumers_unreached(self):
        self.write("cpu/Cargo.toml", '[package]\nname = "cpu"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[features]\nextra = []\n')
        path = "cpu/src/lib.rs"
        shared_path = "cpu/src/shared.rs"
        mirrored_path = "cpu/src/mirrored.rs"
        client_path = "client/src/lib.rs"
        header = ('#![deny(dead_code)]\n'
                  'macro_rules! generate { () => { pub fn generated() {} } }\ngenerate!();\n'
                  '#[rustfmt::skip]\npub fn unknown() {}\n'
                  '#[path = "shared.rs"] mod left;\n'
                  '#[path = "shared.rs"] mod right;\n'
                  'pub mod mirrored;\n')
        reexport = 'use self::hidden as bridge;\npub use bridge::{blocked as exported};\n'
        tail = ('pub(crate) fn restricted() {}\n'
                '#[cfg(feature = "extra")] compile_error!("baseline fixture");\n')
        source = (header + 'mod hidden { pub fn blocked() { super::restricted(); } }\n'
                  + reexport + tail)
        edited = (header + 'mod hidden { pub(crate) fn blocked() { super::restricted(); } }\n'
                  + reexport + tail)
        pristine = {path: source, shared_path: "pub fn shared() {}\n",
                    mirrored_path: "pub fn compiled_in_test() {}\n",
                    "cpu/tests/uses_mirrored.rs": ('#[path = "../src/mirrored.rs"] mod mirrored;\n'
                                                   '#[test] fn reached() { mirrored::compiled_in_test(); }\n'),
                    client_path: "pub fn calls() { cpu::exported(); }\n"}
        for filename, text in pristine.items():
            self.write(filename, text)
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records, code=1)
        self.assert_builds(records, [
            (pristine, ()), ({**pristine, path: edited}, ()), (pristine, ("--all-features",)),
        ])
        default = report["contexts"]["default-features"]
        enabled = report["contexts"]["all-features"]
        self.assert_edits(default, path, source, ["pub fn blocked"])
        self.assertEqual(enabled["edits"], [])
        self.assertEqual(report["state"], "partial")
        self.assertEqual(enabled["baseline"]["state"], "failed")
        self.assertGreater(enabled["baseline"]["exit_code"], 0)
        self.assertIn("baseline fixture", Path(enabled["baseline"]["log"]).read_text(encoding="utf-8"))
        self.assertNotIn("log", enabled["batch"])
        self.assertEqual(self.comparisons(report, "all-features")["crate::hidden", "blocked"]["state"],
                         "baseline blocked")

        gaps = {(gap["span"]["path"], gap["span"].get("line"), gap["reason"])
                for gap in report["discovery"]["gaps"]}
        self.assertTrue(any(filename == path and line == 3 and reason.startswith("generate!")
                            for filename, line, reason in gaps))
        self.assertIn((path, 4, "An attribute can transform the item before compilation."), gaps)
        self.assertTrue(any(filename == shared_path and "logical module origin" in reason
                            for filename, _, reason in gaps))
        discovered = self.declarations(report)
        self.assertEqual({(module, name) for module, name in discovered if name == "shared"},
                         {("crate::left", "shared"), ("crate::right", "shared")})
        self.assertTrue(all(discovered[module, "shared"]["state"] == "unsupported"
                            for module in ("crate::left", "crate::right")))
        self.assertTrue(any(filename == mirrored_path for filename, _, _ in gaps))
        self.assertEqual(discovered["crate::mirrored", "compiled_in_test"]["state"], "unsupported")
        self.assertFalse({"generated", "unknown", "restricted"} & {name for _, name in discovered})
        self.assertEqual(default["batch"]["state"], "failed")
        self.assertGreater(default["batch"]["exit_code"], 0)
        native = default["batch"]["diagnostics"]
        self.assertIn("E0364", {native[index]["code"] for index in default["genuine_error_indices"]})
        self.assertIn("error[E0364]", Path(default["batch"]["log"]).read_text(encoding="utf-8"))
        self.assertEqual(report["candidates"], [])

        client_targets = [(row["target"], row["test"], row["state"])
                          for row in default["unfinished_targets"] if row["target"]["name"] == "client"]
        library = {"name": "client", "kind": ["lib"]}
        self.assertCountEqual(client_targets, [
            (library, False, "no compiler output observed"),
            (library, True, "no compiler output observed"),
        ])

    def test_expected_privacy_preserves_an_independent_failure_and_successful_target(self):
        path = "cpu/src/lib.rs"
        privacy_path = "client/src/bin/privacy.rs"
        good = "client/src/bin/good.rs"
        bad = "client/src/bin/bad.rs"
        consumer = "fn main() { cpu::direct(); }\n"
        source = "pub fn direct() {} pub fn from_glob() {}\n"
        self.write(path, source)
        self.write(privacy_path, consumer)
        self.write(good, "fn main() {}\n")
        self.write(bad, "use cpu::*;\nfn main() { from_glob(); }\n")
        report = self.probe(code=1)
        context = report["contexts"]["default-features"]
        self.assertEqual(context["baseline"]["state"], "succeeded")
        self.assertEqual(report["state"], "partial")
        self.assert_reference(report, ("crate", "direct"), "default-features", "direct reference",
                              privacy_path, consumer, "cpu::direct()", "direct", "E0603")
        diagnostics = context["batch"]["diagnostics"]
        true_errors = {diagnostics[index]["code"] for index in context["genuine_error_indices"]}
        self.assertIn("E0425", true_errors)
        self.assertNotIn("E0603", true_errors)
        self.assertTrue(any(row["target"]["name"] == "good" for row in context["compiled_targets"]))
        for target in ("bad", "privacy"):
            self.assertTrue(any(row["target"]["name"] == target and row["state"] == "diagnostics emitted"
                                for row in context["unfinished_targets"]))

    def test_associated_call_retry_finds_use_after_partial_eq_takes_over(self):
        path = "cpu/src/lib.rs"
        consumer_path = "cpu/tests/condition.rs"
        source = ('#[derive(PartialEq)]\n'
                  'pub struct Condition(u8, u8);\n'
                  'impl Condition { pub fn eq(left: u8, right: u8) -> Self { Self(left, right) } }\n'
                  'impl Condition { pub fn run(&self) -> bool { self.0 == self.1 } }\n'
                  'pub fn touch() {}\n'
                  'pub fn spare() {}\n')
        consumer = ('use cpu::Condition;\n'
                    '#[test] fn checks_condition() {\n'
                    '    let condition = Condition::eq(7, 9);\n'
                    '    assert!(!condition.run());\n'
                    '    cpu::touch();\n'
                    '}\n')
        self.write(path, source)
        self.write(consumer_path, consumer)
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records)
        pristine = {path: source, consumer_path: consumer, "client/src/lib.rs": ""}
        edited = source.replace("pub fn", "pub(crate) fn")
        restored = edited.replace("pub(crate) fn eq", "pub fn eq")
        self.assert_builds(records, [(pristine, ()), ({**pristine, path: edited}, ()),
                                     ({**pristine, path: restored}, ())])

        context = report["contexts"]["default-features"]
        retry = context["retry"]
        self.assertEqual(report["state"], "completed")
        self.assertEqual(retry["state"], "accepted")
        self.assertEqual(retry["resolved_error_indices"], context["genuine_error_indices"])
        self.assertEqual(retry["genuine_error_indices"], [])
        self.assertEqual(list(build_failures(report["contexts"])), [])
        self.assertEqual(retry["restored_visibility"], self.declarations(report)["crate", "eq"]["visibility_span"])
        first_log = Path(context["batch"]["log"]).read_text(encoding="utf-8")
        retry_log = Path(retry["batch"]["log"]).read_text(encoding="utf-8")
        self.assertIn("error[E0308]", first_log)
        self.assertIn("error[E0599]", first_log)
        self.assertNotIn("error[E0308]", retry_log)
        self.assertNotIn("error[E0599]", retry_log)
        self.assertIn("error[E0624]", retry_log)
        self.assertIn("error[E0603]", retry_log)

        used = {(item["module"], item["name"]): item for item in report["confirmed_references"]}
        self.assertEqual(set(used), {("crate", "eq"), ("crate", "run"), ("crate", "touch")})
        evidence, = used["crate", "eq"]["observations"]
        self.assertEqual(evidence["kind"], "visibility retry")
        diagnostic = context["batch"]["diagnostics"][evidence["diagnostic_index"]]
        self.assertEqual(diagnostic["code"], "E0308")
        primary, = (span for span in diagnostic["spans"] if span["is_primary"])
        self.assertEqual(primary["path"], consumer_path)
        self.assertEqual(consumer.encode()[primary["byte_start"]:primary["byte_end"]], b"Condition::eq")
        self.assertEqual({(item["module"], item["name"]) for item in report["candidates"]}, {("crate", "spare")})
        path, = self.results.glob("unused-public-*/report.json")
        with contextlib.redirect_stdout(io.StringIO()) as gate_output:
            self.assertFalse(check_report("cpu", path, 0))
        self.assertIn("candidate unused public function crate::spare", gate_output.getvalue())

    def test_associated_call_with_two_matching_definitions_stays_partial(self):
        path = "cpu/src/lib.rs"
        consumer_path = "cpu/tests/condition.rs"
        definition = ('#[derive(PartialEq)] pub struct Condition(u8, u8);\n'
                      'impl Condition { pub fn eq(left: u8, right: u8) -> Self { Self(left, right) } }\n')
        source = definition + "pub mod alternative {\n" + definition + "}\n"
        consumer = "use cpu::Condition;\n#[test] fn checks_condition() { let _ = Condition::eq(7, 9); }\n"
        self.write(path, source)
        self.write(consumer_path, consumer)
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records, code=1)
        pristine = {path: source, consumer_path: consumer, "client/src/lib.rs": ""}
        edited = source.replace("pub fn", "pub(crate) fn")
        self.assert_builds(records, [(pristine, ()), ({**pristine, path: edited}, ())])
        context = report["contexts"]["default-features"]
        self.assertEqual(report["state"], "partial")
        self.assertNotIn("retry", context)
        self.assertIn("E0308", {context["batch"]["diagnostics"][index]["code"]
                                for index in context["genuine_error_indices"]})
        self.assertEqual(report["confirmed_references"], [])

    def test_no_eligible_functions_skips_batch_after_pristine_baseline(self):
        path = "cpu/src/lib.rs"
        source = ('mod implementation { pub fn kept() {} }\n'
                  'pub use implementation::kept;\n'
                  'pub struct Point { pub x: u8 }\n'
                  'pub(crate) fn restricted() {}\n')
        self.write(path, source)
        records = self.root.parent / "cargo-inputs.jsonl"
        report = self.verify(records)
        self.assert_builds(records, [({path: source, "client/src/lib.rs": ""}, ())])
        context = report["contexts"]["default-features"]
        self.assertEqual(context["baseline"]["state"], "succeeded")
        self.assertEqual(context["batch"]["state"], "skipped")
        self.assertEqual(context["edits"], [])
        self.assertEqual(report["candidates"], [])

    def test_live_index_and_working_copy_survive_success_failure_and_interruptions(self):
        path = "cpu/src/lib.rs"
        extra_path = "cpu/src/untracked.rs"
        binary_path = "cpu/src/bin/independent.rs"
        staged = "pub fn index_only() {}\n"
        working = "pub fn working() { untracked::reached(); }\npub fn followup() {}\nmod untracked;\n"
        edited = ("pub(crate) fn working() { untracked::reached(); }\n"
                  "pub(crate) fn followup() {}\nmod untracked;\n")
        self.write("cpu/Cargo.toml", '[package]\nname = "cpu"\nversion = "0.0.0"\nedition = "2021"\n'
                   '[features]\nextra = []\n')
        self.write(path, staged)
        git_fixture(self.root, "add", path)
        self.write(path, working)
        self.write(extra_path, "pub fn reached() {}\n")
        self.write(binary_path, "fn main() {}\n")
        index = (self.root / ".git/index").read_bytes()
        source = (self.root / path).read_bytes()
        extra = (self.root / extra_path).read_bytes()

        def live_files():
            return {file.relative_to(self.root).as_posix(): file.read_bytes()
                    for file in self.root.rglob("*")
                    if file.is_file() and not file.is_relative_to(self.root / ".git")}

        original_files = live_files()

        def unchanged(consumer=""):
            self.assertEqual((self.root / ".git/index").read_bytes(), index)
            self.assertEqual((self.root / path).read_bytes(), source)
            self.assertEqual((self.root / extra_path).read_bytes(), extra)
            self.assertEqual(live_files(), {**original_files, "client/src/lib.rs": consumer.encode()})
            self.assertFalse((self.root / "target").exists())
            self.assertFalse((self.root / "Cargo.lock").exists())

        records = self.root.parent / "cargo-inputs.jsonl"
        real_build = unused_public.run_build

        def record(workspace, environment, command, output, observation):
            files = {file.relative_to(workspace).as_posix(): file.read_bytes().hex()
                     for file in workspace.rglob("*.rs")}
            with records.open("a", encoding="utf-8") as log:
                log.write(json.dumps({"files": files, "command": command}) + "\n")
            return real_build(workspace, environment, command, output, observation)

        with patch.object(unused_public, "run_build", new=record):
            report = self.probe()
        pristine = {path: working, extra_path: "pub fn reached() {}\n", "client/src/lib.rs": "",
                    binary_path: "fn main() {}\n"}
        batch = {**pristine, path: edited, extra_path: "pub(crate) fn reached() {}\n"}
        self.assert_builds(records, [
            (pristine, ()), (batch, ()), (pristine, ("--all-features",)), (batch, ("--all-features",)),
        ])
        self.assertEqual(set(self.declarations(report)), {("crate", "working"), ("crate", "followup"),
                                                          ("crate::untracked", "reached")})
        selections = {value["path"]: value for value in report["candidate"]["working_copy_selections"]}
        self.assertTrue(selections[path]["differs_from_index"])
        self.assertEqual(selections[path]["sha256"], sha256(source).hexdigest())
        self.assertTrue(selections[extra_path]["untracked"])
        unchanged()

        broken = 'compile_error!("baseline fixture");\n'
        self.write("client/src/lib.rs", broken)
        failure = self.probe(code=1)
        self.assertEqual(failure["state"], "partial")
        unchanged(broken)
        self.write("client/src/lib.rs", "")

        copies = []

        def interrupt_edits(workspace, *args):
            copies.append(workspace)
            raise KeyboardInterrupt

        with patch.object(unused_public, "visibility_edits", new=interrupt_edits):
            before_edits = self.probe(code=130)
        self.assertEqual(before_edits["active"]["phase"], "batch")
        self.assertEqual(before_edits["active"]["context"], "default-features")
        self.assertEqual(before_edits["contexts"]["default-features"]["baseline"]["state"], "succeeded")
        self.assertEqual(before_edits["contexts"]["all-features"]["baseline"]["state"], "pending")
        self.assertNotIn("log", before_edits["contexts"]["default-features"]["batch"])
        self.assertEqual(before_edits["candidates"], [])
        self.assertFalse(any(row.get("observations") for row in self.comparisons(before_edits).values()))
        self.assertEqual(len(copies), 1)
        self.assertFalse(copies[0].exists())
        unchanged()

        attempts = []
        original_start = WorkerProcesses.start

        def interrupt_wait(owner, command, log, environment, cwd):
            attempts.append(cwd)
            child = original_start(owner, command, log, environment, cwd)
            if len(attempts) != 4:
                return child
            isolated = Path(environment["CARGO_TARGET_DIR"])
            self.assertEqual(isolated.parent, cwd.parent)
            self.assertFalse(cwd.is_relative_to(self.root))
            original_wait = child.wait

            def wait_then_interrupt():
                # Let Cargo write useful evidence before the foreground wait is interrupted.
                code = original_wait()
                self.assertEqual(code, 0)
                raise KeyboardInterrupt

            return SimpleNamespace(wait=wait_then_interrupt)

        with patch("unused_public_cargo.WorkerProcesses.start", new=interrupt_wait):
            interrupted = self.probe(code=130)
        self.assertEqual(len(attempts), 4)
        self.assertEqual(interrupted["state"], "interrupted")
        self.assertEqual(interrupted["active"]["phase"], "batch")
        self.assertEqual(interrupted["active"]["context"], "all-features")
        self.assertEqual(interrupted["contexts"]["default-features"]["batch"]["state"], "succeeded")
        self.assertEqual(interrupted["contexts"]["all-features"]["batch"]["state"], "interrupted")
        self.assertEqual({row["kind"] for row in self.comparisons(interrupted, "all-features")[
            "crate", "followup"]["observations"]}, {"dead_code"})
        self.assertFalse(attempts[-1].exists())
        unchanged()

        original_discover = Discovery.discover
        targets = []

        def interrupt_next_target(discovery, target):
            targets.append(Path(target["src_path"]).relative_to(discovery.root).as_posix())
            if len(targets) == 2:
                copies.append(discovery.root)
                raise KeyboardInterrupt
            return original_discover(discovery, target)

        with patch.object(Discovery, "discover", new=interrupt_next_target):
            discovery = self.probe(code=130)
        self.assertCountEqual(targets, [path, binary_path])
        self.assertEqual(discovery["state"], "interrupted")
        self.assertEqual(discovery["discovery"]["state"], "interrupted")
        self.assertEqual(discovery["active"], {"phase": "discovery", "target": targets[1]})
        self.assertEqual(discovery["discovery"]["pending_targets"], [targets[1]])
        self.assertEqual(discovery["candidates"], [])
        self.assertTrue(all(context["baseline"]["state"] == "pending" for context in discovery["contexts"].values()))
        self.assertEqual(len(copies), 2)
        self.assertFalse(copies[1].exists())
        unchanged()


if __name__ == "__main__":
    unittest.main()
