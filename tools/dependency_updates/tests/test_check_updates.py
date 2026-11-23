"""Fixture checks for the manual dependency report."""

from collections import Counter
from contextlib import redirect_stdout
import ctypes
from io import StringIO
import json
import os
from pathlib import Path
import shutil
import sys
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
import check_updates
from dependency_updates import browsers, cargo, common, docker_tools, node_packages, python_packages, setup_tools
from dependency_updates.common import Area, Finding, ProbeError, Probes, Report


def rendered(area):
    out = StringIO()
    Report(out).write_area(area)
    return out.getvalue()


class FlushedOutput(StringIO):
    flushed = ""

    def flush(self):
        super().flush()
        self.flushed = self.getvalue()


class TerminalOutput(StringIO):
    def isatty(self):
        return True

    def fileno(self):
        return 1


class UpdateFixtures(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(TemporaryDirectory()))
        self.probe = Probes(self.root)
        self.tracked = set()

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        self.tracked.add(name)
        return path

    def write_json(self, name, data):
        return self.write(name, json.dumps(data))

    def test_cargo_reports_only_changes_tied_to_direct_declarations_by_locked_version(self):
        self.write("Cargo.toml", """
[workspace]
members = ["crates/*"]
[workspace.dependencies]
shared = { package = "lemma", version = "1.2", optional = true }
""")
        self.write("crates/a/Cargo.toml", """
[package]
name = "member-a"
[target.'cfg(windows)'.build-dependencies]
shared.workspace = true
""")
        self.write("crates/b/Cargo.toml", """
[package]
name = "member-b"
[dev-dependencies]
lemma = "2.1"
""")
        lock = self.write("Cargo.lock", """
version = 4
[[package]]
name = "member-a"
version = "0.1.0"
dependencies = ["lemma 1.2.0"]
[[package]]
name = "member-b"
version = "0.1.0"
dependencies = ["lemma 2.1.0"]
[[package]]
name = "lemma"
version = "1.2.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
[[package]]
name = "lemma"
version = "2.1.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
[[package]]
name = "transitive"
version = "1.9.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
""")
        before = lock.read_bytes()

        def http(url, accept):
            self.assertEqual((url, accept), ("https://index.crates.io/le/mm/lemma", "text/plain"))
            return "\n".join(json.dumps({"name": "lemma", "vers": version, "yanked": yanked})
                             for version, yanked in [("1.3.0", False), ("2.2.0", False),
                                                    ("3.0.0", False), ("4.0.0", True), ("4.0.1-rc.1", False)])

        def resolver(command, *, timeout, combine, env):
            self.assertEqual(command, ["cargo", "+stable", "update", "--dry-run", "--color", "never"])
            self.assertTrue(combine)
            self.assertLessEqual(timeout, 60)
            self.assertTrue(Path(env["CARGO_HOME"]).is_relative_to(self.root / ".agent/tmp"))
            return ("Locking 4 packages to latest compatible versions\n"
                    "    Updating lemma v1.2.0 -> v1.3.0 (available: v3.0.0)\n"
                    "    Updating lemma v2.1.0 -> v2.2.0 (available: v3.0.0)\n"
                    "    Updating transitive v1.9.0 -> v1.9.1\n"
                    "      Adding lemma v3.0.0\n"
                    "warning: aborting update due to dry run")

        area = Area("Cargo")
        with patch.object(self.probe, "http", side_effect=http) as requests, \
             patch.object(self.probe, "run", side_effect=resolver), \
             patch.object(cargo, "cargo_configuration", return_value=False):
            cargo.check_crates(area, self.probe, self.tracked)
        requests.assert_called_once()
        self.assertEqual(lock.read_bytes(), before)
        self.assertEqual(area.counts, {"update": 2, "pin review": 2})
        self.assertCountEqual([
            (row.status, row.item, row.observed, row.declared, row.candidate, row.context)
            for row in area.findings
        ], [
            ("update", "lemma", "1.2.0", "1.2", "1.3.0", None),
            ("update", "lemma", "2.1.0", "2.1", "2.2.0", None),
            ("pin review", "lemma", "1.2.0", "1.2", "3.0.0", "member-a"),
            ("pin review", "lemma", "2.1.0", "2.1", "3.0.0", "member-b"),
        ])
        result = rendered(area)
        self.assertIn("Updates within current requirements\n    lemma 1.2.0 -> 1.3.0", result)
        self.assertIn("lemma (member-a) 1.2.0 -> 3.0.0", result)
        self.assertIn("lemma (member-b) 2.1.0 -> 3.0.0", result)
        self.assertNotIn("transitive", result)
        self.assertNotIn("resolver addition", result)

    def test_cargo_home_publish_age_keeps_direct_release_comparison(self):
        home = self.root / "home"
        config = self.write("home/config.toml", '[registry]\nglobal-min-publish-age = "7 days"\n')
        self.write("Cargo.toml", """
[package]
name = "top"
[dependencies]
lemma = "1.0"
""")
        self.write("Cargo.lock", """
[[package]]
name = "top"
version = "0.1.0"
dependencies = ["lemma"]
[[package]]
name = "lemma"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
""")
        original = config.read_bytes()
        area = Area("Cargo")
        environment = {key: value for key, value in os.environ.items() if not key.startswith("CARGO_")}
        environment["CARGO_HOME"] = str(home)
        with patch.dict(os.environ, environment, clear=True), \
             patch.object(self.probe, "http", return_value='{"name":"lemma","vers":"2.0.0","yanked":false}') as http, \
             patch.object(self.probe, "run", return_value="Locking 0 packages") as run:
            cargo.check_crates(area, self.probe, self.tracked)
        run.assert_not_called()
        http.assert_called_once_with("https://index.crates.io/le/mm/lemma", "text/plain")
        self.assertEqual(area.counts, {"pin review": 1, "inconclusive": 1})
        self.assertIn(Finding("pin review", "lemma", observed="1.0.0", declared="1.0", candidate="2.0.0",
                              source=cargo.CRATES_INDEX, context="top"), area.findings)
        self.assertIn("Could not check\n    resolved updates (the isolated cache would omit Cargo home configuration)",
                      rendered(area))
        self.assertEqual(config.read_bytes(), original)

    def test_nested_python_requirements_keep_pins_and_query_once_across_files(self):
        self.write("first/requirements.txt", "-r nested/include.txt\n")
        self.write("first/nested/include.txt", "-r ../packages.txt\n")
        self.write("first/packages.txt", "example[extra]==1.2\nmissing==1.0\n")
        self.write("second/requirements.txt", "example>=1,<2\n")
        installed = {
            "example": SimpleNamespace(version="1.2", requires=['transit>=1; extra == "extra"']),
            "transit": SimpleNamespace(version="1.1", requires=[]),
        }
        versions = {"example": ["2.0rc1", "1.3", "1.2"], "missing": ["1.1", "1.0"],
                    "transit": ["1.2", "1.1"], "pip": ["25.0"]}

        def distribution(name):
            if name in installed:
                return installed[name]
            raise python_packages.metadata.PackageNotFoundError(name)

        def run(command, **_options):
            if command[-1] == "--version":
                return "pip 25.0 from installed pip"
            self.assertEqual(command[:4], [sys.executable, "-B", "-m", "pip"])
            self.assertIn("https://pypi.org/simple", command)
            self.assertIn("--no-cache-dir", command)
            return json.dumps({"versions": versions[command[command.index("versions") + 1]]})

        area = Area("Python")
        with patch.object(self.probe, "run", side_effect=run) as commands, \
             patch.object(python_packages.metadata, "distribution", side_effect=distribution) as metadata:
            python_packages.check_python_packages(area, self.probe, self.tracked)
        self.assertCountEqual([call.args[0] for call in metadata.call_args_list], ["example", "missing"])
        package_calls = [call.args[0][call.args[0].index("versions") + 1] for call in commands.call_args_list
                         if "versions" in call.args[0]]
        self.assertEqual(Counter(package_calls), {"example": 1, "missing": 1, "pip": 1})
        example = [(row.status, row.context, row.observed, row.declared, row.candidate)
                   for row in area.findings if row.item == "example"]
        self.assertEqual(example, [
            ("pin review", "first, via packages.txt", "1.2", "example[extra]==1.2", "1.3"),
            ("update", "second", "1.2", "example<2,>=1", "1.3"),
        ])
        result = rendered(area)
        self.assertIn("example (first, via packages.txt) 1.2 -> 1.3", result)
        self.assertIn("example (second) 1.2 -> 1.3", result)
        self.assertIn("Missing\n    missing not installed", result)
        self.assertNotIn("transit", result)
        self.assertNotIn("2.0rc1", result)

    def test_pip_keeps_only_the_public_index_when_falling_back_to_text(self):
        _, _, Version = python_packages.packaging_api()
        responses = [
            ProbeError("fixture pip does not recognize --json"),
            "example (2.0)\nAvailable versions: 2.0, 1.5, 1.0",
        ]
        configured = {
            "PIP_EXTRA_INDEX_URL": "https://extra.example.invalid/simple",
            "PIP_FIND_LINKS": "https://links.example.invalid/",
            "PIP_CONFIG_FILE": str(self.root / "pip.ini"),
        }
        with patch.dict(os.environ, configured), patch.object(self.probe, "run", side_effect=responses) as run:
            versions = python_packages.pip_index(self.probe, "example", Version)
        self.assertEqual([str(version) for version in versions], ["2.0", "1.5", "1.0"])
        self.assertEqual(run.call_count, 2)
        for call, json_requested in zip(run.call_args_list, (True, False)):
            command = call.args[0]
            self.assertEqual("--json" in command, json_requested)
            self.assertEqual(command[command.index("--index-url") + 1], "https://pypi.org/simple")
            env = call.kwargs.get("env") or {}
            self.assertEqual({key: env[key] for key in configured if key in env}, {
                "PIP_EXTRA_INDEX_URL": None, "PIP_FIND_LINKS": None, "PIP_CONFIG_FILE": os.devnull,
            })

    def test_windows_npm_uses_the_resolved_launcher_and_its_own_node_version(self):
        launcher = self.root / "launcher/npm.cmd"
        runtime = self.write("launcher/node.exe", "")
        cli = self.write("prefix/node_modules/npm/bin/npm-cli.js", "")
        self.write_json("prefix/node_modules/npm/package.json", {"name": "npm", "version": "10.0.0"})
        metadata = {"name": "npm", "versions": {
            "11.0.0": {"engines": {"node": ">=18"}},
            "12.0.0": {"engines": {"node": ">=22"}},
        }}
        area = Area("Node.js and npm")

        def which(name):
            self.assertEqual(name, "npm.cmd")
            return str(launcher)

        def run(command, **options):
            if command == ["node", "--version"]:
                return "v24.1.0"
            if command == [str(launcher), "version", "--json"]:
                environment = options["env"]
                cache = Path(environment["npm_config_cache"])
                self.assertTrue(cache.is_relative_to(self.root / ".agent/tmp"))
                self.assertEqual(environment["npm_config_logs_dir"], str(cache))
                self.assertIsNone(environment["NODE_OPTIONS"])
                return '{"npm":"10.0.0","node":"18.0.0"}'
            if command == [str(launcher), "root", "--global"]:
                return str(self.root / "prefix/node_modules")
            self.assertEqual(command[:2], [str(runtime), "-e"])
            self.assertEqual(command[-1], str(cli))
            self.assertEqual(json.loads(options["input_text"]), [{
                "version": "10.0.0", "ranges": [],
                "candidates": [["11.0.0", ">=18"], ["12.0.0", ">=22"]],
                "engine": "18.0.0",
            }])
            return '[{"latest":"12.0.0","allowed":"11.0.0","matches":true}]'

        with patch.object(sys, "platform", "win32"), \
             patch.object(node_packages.shutil, "which", side_effect=which), \
             patch.object(self.probe, "http", return_value=metadata), patch.object(self.probe, "run", side_effect=run), \
             patch.object(node_packages, "check_node") as check_node:
            node_packages.check_node_packages(area, self.probe, set())
        check_node.assert_called_once_with(area, self.probe, "v24.1.0")
        self.assertEqual(area.counts, {"update": 1, "review": 1})
        self.assertEqual([row.candidate for row in area.findings], ["11.0.0", "12.0.0"])
        self.assertIn("npm uses Node.js 18.0.0.", rendered(area))

    def test_npm_with_unknown_package_location_reports_version_but_leaves_comparison_open(self):
        launcher = self.root / "npm-wrapper"
        area = Area("Node.js and npm")
        payload = {"name": "npm", "versions": {"11.0.0": {"engines": {"node": ">=18"}}}}
        self.write("node_modules/npm/bin/npm-cli.js", "")
        self.write_json("node_modules/npm/package.json", {"name": "npm", "version": "9.0.0"})
        responses = ["v24.1.0", '{"npm":"10.0.0","node":"18.0.0"}', str(self.root / "node_modules")]
        with patch.object(node_packages.shutil, "which", return_value=str(launcher)), \
             patch.object(self.probe, "run", side_effect=responses) as run, \
             patch.object(self.probe, "http", return_value=payload), patch.object(node_packages, "check_node"):
            node_packages.check_node_packages(area, self.probe, set())
        self.assertEqual(run.call_count, 3)
        self.assertEqual(area.counts, {"inconclusive": 1})
        self.assertEqual((area.findings[0].item, area.findings[0].observed, area.findings[0].source),
                         ("npm", "10.0.0", node_packages.NPM_REGISTRY))
        self.assertIn("Could not check\n    npm 10.0.0 (could not identify the installed npm package", rendered(area))

    def test_failed_npm_release_lookup_keeps_the_observed_version(self):
        area = Area("Node.js and npm")
        with (patch.object(self.probe, "run", return_value="v24.1.0"),
              patch.object(node_packages, "check_node"),
              patch.object(node_packages, "npm_runtime", return_value=("10.0.0", "24.1.0", "node", "npm-cli.js")),
              patch.object(self.probe, "http", side_effect=ProbeError("fixture registry unavailable"))):
            node_packages.check_node_packages(area, self.probe, set())
        self.assertIn("Could not check\n    npm 10.0.0 (fixture registry unavailable)", rendered(area))
        self.assertNotIn("npm uses Node.js", rendered(area))

    def test_npm_queries_only_packages_declared_in_manifest_without_node_modules(self):
        if not shutil.which("npm.cmd" if sys.platform == "win32" else "npm"):
            self.skipTest("npm is required to execute its semver rules")
        self.write_json("tools/book/package.json", {"dependencies": {"katex": "0.18.2"}})
        self.write_json("tools/book/package-lock.json", {"packages": {
            "": {"dependencies": {"katex": "0.18.2"}},
            "node_modules/commander": {"version": "8.3.0",
                                       "resolved": "https://registry.npmjs.org/commander/-/commander-8.3.0.tgz"},
            "node_modules/katex": {"version": "0.18.2", "dependencies": {"commander": "^8.3.0"},
                                   "resolved": "https://registry.npmjs.org/katex/-/katex-0.18.2.tgz"},
        }})
        index = {
            "katex": {"0.18.2": {}, "0.19.1": {}},
            "commander": {"8.3.0": {}, "8.4.0": {}, "15.1.0": {}, "16.0.0-beta.1": {}},
            "npm": {"90.0.0": {"engines": {"node": ">=10"}},
                    "100.0.0": {"engines": {"node": ">=100"}}},
        }

        def http(url, accept):
            self.assertEqual(accept, "application/vnd.npm.install-v1+json")
            name = url.rsplit("/", 1)[-1]
            return {"name": name, "versions": index[name]}

        area = Area("Node.js and npm")
        self.assertFalse((self.root / "tools/book/node_modules").exists())
        with patch.object(self.probe, "http", side_effect=http) as requests, patch.object(node_packages, "check_node"):
            node_packages.check_node_packages(area, self.probe, self.tracked)
        self.assertCountEqual([call.args[0].rsplit("/", 1)[-1] for call in requests.call_args_list], ["npm", "katex"])
        self.assertCountEqual([
            (row.status, row.item, row.observed, row.declared, row.candidate, row.context)
            for row in area.findings if row.item != "npm"
        ], [
            ("pin review", "katex", "0.18.2", "0.18.2", "0.19.1", "book"),
        ])
        report = rendered(area)
        self.assertNotIn("commander", report)
        self.assertRegex(report, r"npm [^\n]+ -> 90\.0\.0")
        self.assertRegex(report, r"npm [^\n]+ -> 100\.0\.0")
        self.assertNotIn("16.0.0-beta", report)
        self.assertFalse((self.root / "tools/book/node_modules").exists())

    def test_current_edge_is_sufficient_without_a_bundled_browser_or_its_metadata(self):
        edge = self.write("Edge/msedge.exe", "")
        playwright = SimpleNamespace(locate_file=lambda path: self.root / path)
        releases = [{"Product": "Stable", "Releases": [
            {"Platform": "Windows", "Architecture": "x64", "ProductVersion": "150.0.1.2"},
        ]}]
        area = Area("Book Review browsers")
        with (patch.object(browsers.metadata, "distribution", return_value=playwright),
              patch.object(self.probe, "run", return_value=json.dumps(str(edge))) as commands,
              patch.object(browsers, "edge_installed_version", return_value=((150, 0, 1, 2), "windows", "x64"))
              as read_version,
              patch.object(self.probe, "http", return_value=releases) as requests):
            browsers.check_browsers(area, self.probe, self.tracked)
        commands.assert_called_once()
        read_version.assert_called_once_with(self.probe, edge)
        requests.assert_called_once_with(browsers.EDGE_RELEASES)
        self.assertEqual(area.counts, {"current": 1})
        self.assertEqual(rendered(area), "  No findings.\n")

    def test_missing_edge_is_reported_as_the_default_book_review_browser(self):
        playwright = SimpleNamespace(locate_file=lambda path: self.root / path)
        for executable in (None, str(self.root / "Edge/msedge.exe")):
            with self.subTest(executable=executable):
                area = Area("Book Review browsers")
                with (patch.object(browsers.metadata, "distribution", return_value=playwright),
                      patch.object(self.probe, "run", return_value=json.dumps(executable)),
                      patch.object(self.probe, "http") as requests):
                    browsers.check_browsers(area, self.probe, self.tracked)
                requests.assert_not_called()
                self.assertEqual(area.counts, {"missing": 1})
                self.assertIn("Missing\n    Microsoft Edge Stable not found (default Book Review browser)",
                              rendered(area))

    def test_failed_edge_release_lookup_keeps_the_observed_version(self):
        edge = self.write("Edge/msedge.exe", "")
        area = Area("Book Review browsers")
        with (patch.object(browsers, "edge_installed_version", return_value=((158, 0, 4370, 33), "windows", "x64")),
              patch.object(self.probe, "http", side_effect=ProbeError("fixture release feed unavailable"))):
            browsers.check_edge(area, self.probe, edge)
        self.assertIn("Could not check\n    Microsoft Edge Stable 158.0.4370.33 (fixture release feed unavailable)",
                      rendered(area))

    def test_rust_channels_compare_stable_versions_and_nightly_dates(self):
        area = Area("Rust")
        stable = "stable-x86_64-pc-windows-msvc"
        nightly = "nightly-x86_64-pc-windows-msvc"
        stable_source = "https://static.rust-lang.org/dist/channel-rust-stable.toml"
        nightly_source = "https://static.rust-lang.org/dist/channel-rust-nightly.toml"
        local = ["rustc 1.99.0 (abc123 2026-10-01)", "rustc 1.102.0-nightly (aabbcc 2026-11-13)"]
        remote = ['[pkg.rust]\nversion = "1.100.0 (def456 2026-11-09)"\n',
                  '[pkg.rust]\nversion = "1.102.0-nightly (ddeeff 2026-11-15)"\n']
        with (patch.object(self.probe, "run", side_effect=local) as run,
              patch.object(self.probe, "http", side_effect=remote) as http,
              patch.object(setup_tools, "rust_components")):
            setup_tools.rust_one_channel(area, self.probe, "stable", stable, "Stable setup fixture")
            setup_tools.rust_one_channel(area, self.probe, "nightly", nightly, "Nightly setup fixture")
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["rustup", "run", stable, "rustc", "--version"],
            ["rustup", "run", nightly, "rustc", "--version"],
        ])
        self.assertEqual([call.args for call in http.call_args_list], [
            (stable_source, "text/plain"), (nightly_source, "text/plain"),
        ])
        self.assertEqual(area.counts, {"update": 2})
        self.assertEqual([(row.item, row.observed, row.candidate) for row in area.findings], [
            ("Rust Stable", "1.99.0", "1.100.0"),
            ("Rust Nightly", "1.102.0-nightly (2026-11-13)", "1.102.0-nightly (2026-11-15)"),
        ])
        self.assertEqual(rendered(area).count("Rust Stable 1.99.0"), 1)

    def test_rust_stable_counts_current_only_after_a_comparable_release_response(self):
        stable = "stable-x86_64-pc-windows-msvc"
        release = '[pkg.rust]\nversion = "1.100.0"\n'
        for label, installed, response, status in (
            ("equal", "rustc 1.100.0", release, "current"),
            ("local newer", "rustc 1.101.0", release, "current"),
            ("prerelease", "rustc 1.100.0", '[pkg.rust]\nversion = "1.102.0-nightly"\n', "inconclusive"),
            ("malformed", "rustc 1.100.0", "not valid TOML", "inconclusive"),
            ("offline", "rustc 1.100.0", ProbeError("fixture source unavailable"), "inconclusive"),
        ):
            with (self.subTest(label=label),
                  patch.object(self.probe, "run", return_value=installed),
                  patch.object(self.probe, "http", side_effect=[response]),
                  patch.object(setup_tools, "rust_components") as components):
                area = Area("Rust")
                setup_tools.rust_one_channel(area, self.probe, "stable", stable, "Stable setup fixture")
                self.assertEqual(area.counts, {status: 1})
                if status == "inconclusive":
                    self.assertIn("Could not check\n    Rust Stable 1.100.0", rendered(area))
                components.assert_called_once_with(area, self.probe, "stable", stable, "Stable setup fixture")

    def test_windows_checks_availability_and_compares_separate_pwsh_release(self):
        area = Area("Setup")
        release = {"draft": False, "prerelease": False, "tag_name": "v7.6.0"}
        with patch.object(sys, "platform", "win32"), patch.object(setup_tools.shutil, "which", return_value="pwsh"), \
             patch.object(self.probe, "run", side_effect=["5.1.26100.1", "PowerShell 7.5.0"]) as run, \
             patch.object(self.probe, "http", return_value=release) as http:
            setup_tools.check_powershell(area, self.probe)
        self.assertEqual([call.args[0][0] for call in run.call_args_list], ["powershell", "pwsh"])
        http.assert_called_once_with("https://api.github.com/repos/PowerShell/PowerShell/releases/latest")
        self.assertEqual(area.counts, {"current": 1, "update": 1})
        self.assertIn("PowerShell (pwsh) 7.5.0 -> 7.6.0", rendered(area))
        self.assertNotIn("Windows PowerShell", rendered(area))

    def test_windows_powershell_availability_needs_no_release_check(self):
        inputs = (
            ("5.1.26100.1", None),
            (ProbeError("powershell is unavailable"), "powershell is unavailable"),
            ("", "the executable did not report a version"),
        )
        for version, issue in inputs:
            with (self.subTest(version=version), patch.object(sys, "platform", "win32"),
                  patch.object(setup_tools.shutil, "which", return_value=None),
                  patch.object(self.probe, "run", side_effect=[version]) as run,
                  patch.object(self.probe, "http") as http):
                area = Area("Other project tools")
                setup_tools.check_powershell(area, self.probe)
                run.assert_called_once_with(["powershell", "-NoProfile", "-NonInteractive", "-Command",
                                             "$PSVersionTable.PSVersion.ToString()"])
                http.assert_not_called()
                out = StringIO()
                report = Report(out)
                report.write_area(area)
                report.finish()
                result = out.getvalue()
                if issue is None:
                    self.assertEqual(area.counts, {"current": 1})
                    self.assertIn("Report complete.", result)
                    self.assertNotIn("Windows PowerShell", result)
                else:
                    self.assertIn(f"Could not check\n    Windows PowerShell ({issue})", result)
                    self.assertIn("Partial report.", result)

    def test_qdrant_compares_the_running_image_separately_from_the_local_tag(self):
        active = "sha256:" + "a" * 64
        local = "sha256:" + "b" * 64
        linux_amd64 = "sha256:" + "c" * 64
        linux_arm64 = "sha256:" + "d" * 64
        config = {"Os": "linux", "Architecture": "amd64"}
        commands = {
            ("ps", "--format", "{{json .}}"): {"ID": "e" * 12, "Image": "qdrant/qdrant:latest"},
            ("container", "inspect", "--format", "{{.Image}}", "e" * 12): active,
            ("image", "ls", "--format", "{{json .}}", "qdrant/qdrant"): {
                "Repository": "qdrant/qdrant", "Tag": "latest",
            },
            ("image", "inspect", "--format", "{{.Id}}", "qdrant/qdrant:latest"): local,
            ("image", "inspect", "--format", "{{json .}}", active): {**config, "Id": active},
            ("image", "inspect", "--format", "{{json .}}", local): {**config, "Id": local},
            ("manifest", "inspect", "qdrant/qdrant:latest"): {"manifests": [
                {"platform": {"os": "linux", "architecture": "arm64"}, "digest": linux_arm64},
                {"platform": {"os": "linux", "architecture": "amd64"}, "digest": linux_amd64},
            ]},
            ("manifest", "inspect", f"qdrant/qdrant@{linux_amd64}"): {"config": {"digest": local}},
        }

        def run(command, **_options):
            self.assertEqual(command[0], "docker")
            response = commands[tuple(command[1:])]
            return response if isinstance(response, str) else json.dumps(response)

        area = Area("Qdrant")
        with patch.object(self.probe, "run", side_effect=run) as invocations:
            docker_tools.check_qdrant(area, self.probe)
        self.assertEqual(area.counts, {"current": 1, "update": 1})
        self.assertIn("(running container)", area.findings[0].item)
        self.assertEqual((area.findings[0].observed, area.findings[0].candidate), (active, local))
        self.assertIn("Qdrant:latest (running container) aaaaaaaaaaaa -> bbbbbbbbbbbb", rendered(area))
        self.assertNotIn(active, rendered(area))
        self.assertEqual(sum(call.args[0][1:3] == ["manifest", "inspect"]
                             for call in invocations.call_args_list), 2)

    def test_report_continues_after_failed_crates_source_without_claiming_complete(self):
        self.write("Cargo.toml", """
[package]
name = "top"
[dependencies]
lemma = "1.0"
""")
        self.write("Cargo.lock", """
[[package]]
name = "top"
version = "0.1.0"
dependencies = ["lemma"]
[[package]]
name = "lemma"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
""")
        summary = FlushedOutput()
        major, minor, patch_version = sys.version_info[:3]
        python_candidate = f"{major}.{minor}.{patch_version + 1}"

        def http(url, *_args):
            if url == python_packages.PYTHON_RELEASES:
                self.assertIn("lemma 1.0.0 -> 1.0.1", summary.flushed)
                self.assertIn("Checking Python and pip...", summary.flushed)
                return [{"name": f"Python {python_candidate}", "pre_release": False, "is_published": True}]
            if url.startswith(cargo.CRATES_INDEX):
                self.assertIn("Checking Cargo packages...", summary.flushed)
                self.assertIn("Checking direct crates on crates.io...", summary.flushed)
            raise ProbeError("fixture source returned HTTP 503")

        def run(command, **_options):
            if command == ["git", "ls-files", "-z"]:
                self.assertEqual(summary.flushed, "Finding project dependency files...\n")
                return "\0".join(sorted(self.tracked)) + "\0"
            if command == ["rustup", "show", "active-toolchain"]:
                self.assertIn("Checking Rust toolchains and Cargo tools...", summary.flushed)
            if command[:4] == ["cargo", "+stable", "update", "--dry-run"]:
                self.assertIn("Checking what Cargo can resolve with current manifests...", summary.flushed)
                return "Locking 1 package to latest compatible version\nUpdating lemma v1.0.0 -> v1.0.1"
            if command == [sys.executable, "-B", "-m", "pip", "--version"]:
                return "pip 25.0 from installed pip"
            if command[:4] == [sys.executable, "-B", "-m", "pip"]:
                self.assertIn("Checking Python package releases on PyPI...", summary.flushed)
                return '{"versions": ["25.0"]}'
            raise ProbeError("fixture command unavailable")

        with patch.object(check_updates, "ROOT", self.root), patch.object(check_updates, "Probes", return_value=self.probe), \
             patch.object(self.probe, "http", side_effect=http), patch.object(self.probe, "run", side_effect=run), \
             patch.object(cargo, "cargo_configuration", return_value=False), redirect_stdout(summary):
            self.assertEqual(check_updates.main([]), 0)
        report = summary.getvalue()
        self.assertIn("Could not check\n    lemma (top) (fixture source returned HTTP 503)", report)
        self.assertIn("Updates within current requirements\n    lemma 1.0.0 -> 1.0.1", report)
        self.assertIn(f" -> {python_candidate} (selected release series)", report)
        self.assertIn("Partial report", report)
        self.assertNotIn("Report complete", report)
        self.assertNotIn("Coverage", report)
        self.assertNotIn("\x1b[", report)

    def test_report_deduplicates_findings_and_keeps_missing_and_failed_checks_visible(self):
        private = "https://fixture-user:fixture-password@example.invalid/simple"

        def check(area, *_):
            for context in ("book review", "documentation"):
                area.record("pin review", "playwright", observed="1.63.0", candidate="1.65.2",
                            context=context, declared="playwright==1.63.0", source=private)
            area.record("missing", "unavailable-package", observed="not installed")
            area.incomplete("unknown-package", private, "registry timed out", declared=f"other @ {private}")
            area.incomplete("invalid-version", private, f"invalid source {private}", observed=private)

        with patch.multiple(check_updates, Probes=Mock(return_value=self.probe), check_rust=Mock(),
                            check_crates=Mock(), check_python=check, check_browsers=Mock(),
                            check_node_packages=Mock(), check_setup=Mock(), check_docker=Mock()), \
             patch.object(self.probe, "tracked_files", return_value=set()), redirect_stdout(StringIO()) as stream:
            self.assertEqual(check_updates.main([]), 0)
        result = stream.getvalue()
        self.assertIn("Requires changing project requirements\n    playwright 1.63.0 -> 1.65.2", result)
        self.assertEqual(result.count("playwright 1.63.0 -> 1.65.2"), 1)
        self.assertIn("Missing\n    unavailable-package not installed", result)
        self.assertIn("Could not check\n    unknown-package (registry timed out)", result)
        self.assertIn("Partial report", result)
        for hidden in ("fixture-user", "fixture-password", "playwright==", "https://", "Coverage"):
            self.assertNotIn(hidden, result)

    def test_terminal_color_and_redirected_output_keep_the_same_readable_headings(self):
        area = Area("Python and pip")
        area.notes.append("Selected Python environment: base environment.")
        area.record("update", "pip", observed="25.0", candidate="25.1")
        plain, terminal, disabled = StringIO(), TerminalOutput(), TerminalOutput()

        def render(out):
            report = Report(out)
            report.start_area(area.name)
            report.write_area(area)
            report.finish()

        with patch.dict(os.environ, {}, clear=True), patch.object(common.os, "name", "posix"):
            render(plain)
            render(terminal)
            with patch.dict(os.environ, {"NO_COLOR": "1"}):
                render(disabled)
        self.assertIn("\x1b[1;36mChecking Python and pip\x1b[0m", terminal.getvalue())
        self.assertIn("\x1b[32mUpdates available\x1b[0m", terminal.getvalue())
        self.assertEqual(disabled.getvalue(), plain.getvalue())
        self.assertNotIn("\x1b[", plain.getvalue())
        self.assertIn("Selected Python environment: base environment.", plain.getvalue())
        self.assertEqual(plain.getvalue().count("pip 25.0"), 1)
        self.assertIn("Report complete.", plain.getvalue())

    def test_windows_terminal_enables_vt_output_when_console_supports_it(self):
        kernel = SimpleNamespace(GetConsoleMode=Mock(return_value=1), SetConsoleMode=Mock(return_value=1))
        msvcrt = SimpleNamespace(get_osfhandle=Mock(return_value=42))
        with patch.dict(os.environ, {}, clear=True), patch.object(common.os, "name", "nt"), \
             patch.dict(sys.modules, {"msvcrt": msvcrt}), \
             patch.object(ctypes, "windll", SimpleNamespace(kernel32=kernel), create=True):
            terminal = TerminalOutput()
            Report(terminal).start_area("Rust toolchains and Cargo tools")
            kernel.SetConsoleMode.assert_called_once()
            self.assertEqual(kernel.SetConsoleMode.call_args.args[1], 4)
            msvcrt.get_osfhandle.assert_called_once_with(1)
        self.assertIn("\x1b[1;36mChecking Rust toolchains and Cargo tools\x1b[0m", terminal.getvalue())

    def test_failed_docker_desktop_release_lookup_keeps_the_observed_version(self):
        area = Area("Docker and Qdrant")
        server = {"Version": "29.0.0", "Platform": {"Name": "Docker Desktop 4.60.0"}}
        with (patch.object(docker_tools.shutil, "which", return_value="docker"),
              patch.object(docker_tools.platform, "machine", return_value="AMD64"),
              patch.object(sys, "platform", "win32"),
              patch.object(self.probe, "run", side_effect=["Docker version 29.0.0", json.dumps({"Server": server}),
                                                           "npipe://fixture-daemon"]),
              patch.object(self.probe, "http", side_effect=ProbeError("fixture release notes unavailable")),
              patch.object(docker_tools, "check_qdrant")):
            docker_tools.check_docker(area, self.probe, self.tracked)
        self.assertIn("Could not check\n    Docker distribution 4.60.0 (fixture release notes unavailable)",
                      rendered(area))

    def test_current_rust_channel_versions_are_hidden_with_or_without_an_update(self):
        stable = "stable-x86_64-pc-windows-msvc"
        nightly = "nightly-x86_64-pc-windows-msvc"
        local_nightly = "1.102.0-nightly (aabbcc 2026-11-13)"
        for remote_nightly in (local_nightly, "1.102.0-nightly (ddeeff 2026-11-15)"):
            with self.subTest(remote_nightly=remote_nightly):
                area = Area("Rust toolchains and Cargo tools")
                local = ["rustc 1.100.0", f"rustc {local_nightly}"]
                remote = ['[pkg.rust]\nversion = "1.100.0"\n', f'[pkg.rust]\nversion = "{remote_nightly}"\n']
                with (patch.object(self.probe, "run", side_effect=local),
                      patch.object(self.probe, "http", side_effect=remote),
                      patch.object(setup_tools, "rust_components")):
                    setup_tools.rust_one_channel(area, self.probe, "stable", stable, "Stable setup fixture")
                    setup_tools.rust_one_channel(area, self.probe, "nightly", nightly, "Nightly setup fixture")
                out = StringIO()
                report = Report(out)
                report.start_area(area.name)
                report.write_area(area)
                report.finish()
                result = out.getvalue()
                self.assertIn("Checking Rust toolchains and Cargo tools", result)
                self.assertNotIn("Rust Stable", result)
                self.assertNotIn("Installed:", result)
                self.assertNotIn("Coverage", result)
                if remote_nightly == local_nightly:
                    self.assertNotIn("Rust Nightly", result)
                    self.assertIn("No findings.", result)
                    self.assertIn("Report complete. No updates or installation differences found.", result)
                else:
                    self.assertIn("Rust Nightly 1.102.0-nightly (2026-11-13) -> "
                                  "1.102.0-nightly (2026-11-15)", result)
                    self.assertIn("Report complete.", result)


if __name__ == "__main__":
    unittest.main()
