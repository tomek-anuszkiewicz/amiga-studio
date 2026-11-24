"""Fixture checks for the manual dependency report."""

from collections import Counter
from contextlib import contextmanager, redirect_stdout
import ctypes
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from io import StringIO
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from threading import Event, Thread
from time import monotonic
from types import SimpleNamespace
import unittest
from unittest.mock import ANY, Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
import check_updates
from dependency_updates import browsers, cargo, common, docker_tools, node_packages, python_packages, setup_tools
from dependency_updates.common import Area, CommandFailed, Finding, ProbeError, Probes, Report


def rendered(area):
    out = StringIO()
    Report(out).write_area(area)
    return out.getvalue()


@contextmanager
def serve_http(handler):
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = Thread(target=server.serve_forever, kwargs={"poll_interval": 0.05})
        thread.start()
        try:
            with patch.dict(os.environ, {"no_proxy": "127.0.0.1"}):
                yield f"http://127.0.0.1:{server.server_port}"
        finally:
            server.shutdown()
            thread.join()


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

    def test_http_probe_limits_total_time_when_body_keeps_arriving(self):
        sent = Event()
        stop = Event()

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                if self.path == "/redirect":
                    self.send_response(302)
                    self.send_header("Location", "/slow")
                    self.end_headers()
                    return
                self.send_response(200)
                self.send_header("Content-Length", "12")
                self.end_headers()
                try:
                    for index in range(12):
                        self.wfile.write(b"a")
                        if index == 1:
                            sent.set()
                        if stop.wait(0.2):
                            return
                except OSError:
                    pass

            def log_message(self, *_args):
                pass

        with serve_http(Handler) as url:
            try:
                with patch.object(common, "HTTP_DEADLINE", 1.2, create=True):
                    with self.assertRaisesRegex(ProbeError, "timed out"):
                        self.probe.http(f"{url}/redirect", "text/plain")
                self.assertTrue(sent.is_set(), "the redirected response must start before the deadline")
            finally:
                stop.set()

    def test_http_probe_caches_successes_and_reports_only_safe_http_error_details(self):
        requests = []

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                requests.append((self.path, self.headers["Accept"]))
                if self.path == "/releases":
                    self.send_response(200)
                    self.end_headers()
                    self.wfile.write(b'{"version": "1.2.3"}')
                else:
                    self.send_error(503, "fixture-password")

            def log_message(self, *_args):
                pass

        with serve_http(Handler) as url:
            for _ in range(2):
                self.assertEqual(self.probe.http(f"{url}/releases"), {"version": "1.2.3"})
            with self.assertRaisesRegex(ProbeError, "^source returned HTTP 503$"):
                self.probe.http(f"{url}/unavailable", "text/plain")
        self.assertEqual(requests, [("/releases", "application/json"), ("/unavailable", "text/plain")])

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

        def http(url, accept, *, batch):
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
        http.assert_called_once_with("https://index.crates.io/le/mm/lemma", "text/plain", batch=ANY)
        self.assertEqual(area.counts, {"pin review": 1, "inconclusive": 1})
        self.assertIn(Finding("pin review", "lemma", observed="1.0.0", declared="1.0", candidate="2.0.0",
                              source=cargo.CRATES_INDEX, context="top"), area.findings)
        self.assertIn("Could not check\n    resolved updates (the isolated cache would omit Cargo home configuration)",
                      rendered(area))
        self.assertEqual(config.read_bytes(), original)

    def test_failed_crates_index_keeps_each_direct_lock_version(self):
        self.write("Cargo.toml", '[workspace]\nmembers = ["apps/*"]\n')
        self.write("apps/gui/Cargo.toml", '[package]\nname = "gui"\n[dependencies]\negui = "0.32"\n')
        self.write("apps/preview/Cargo.toml", '[package]\nname = "preview"\n[dependencies]\negui = "0.31"\n')
        self.write("Cargo.lock", """
[[package]]
name = "gui"
version = "0.1.0"
dependencies = ["egui 0.32.3"]
[[package]]
name = "preview"
version = "0.1.0"
dependencies = ["egui 0.31.1"]
[[package]]
name = "egui"
version = "0.32.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
[[package]]
name = "egui"
version = "0.31.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
""")
        area = Area("Cargo")
        with (patch.object(cargo, "cargo_configuration", return_value=False),
              patch.object(self.probe, "http", side_effect=ProbeError("fixture index unavailable")) as http,
              patch.object(self.probe, "run", return_value="Locking 0 packages") as run):
            cargo.check_crates(area, self.probe, self.tracked)
        http.assert_called_once_with("https://index.crates.io/eg/ui/egui", "text/plain", batch=ANY)
        self.assertEqual(run.call_args.args[0][:4], ["cargo", "+stable", "update", "--dry-run"])
        self.assertEqual(area.counts, {"inconclusive": 2, "current": 2})
        self.assertIn("egui (gui) 0.32.3 (fixture index unavailable)", rendered(area))
        self.assertIn("egui (preview) 0.31.1 (fixture index unavailable)", rendered(area))

    def test_incomplete_cargo_comparison_keeps_the_exact_linked_prerelease(self):
        self.write("Cargo.toml", '[package]\nname = "top"\n[dependencies]\nlemma = "=1.2.3-rc.1"\n')
        self.write("Cargo.lock", """
[[package]]
name = "top"
version = "0.1.0"
dependencies = ["lemma 1.2.3-rc.1"]
[[package]]
name = "lemma"
version = "1.2.3-rc.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
[[package]]
name = "lemma"
version = "1.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
""")
        index = json.dumps({"name": "lemma", "vers": "1.2.3", "yanked": False})
        unavailable = "fixture index unavailable"
        for response, reason in ((ProbeError(unavailable), unavailable),
                                 (index, "no matching stable registry version could be tied to this declaration")):
            with self.subTest(reason=reason):
                area = Area("Cargo")
                with (patch.object(cargo, "cargo_configuration", return_value=False),
                      patch.object(self.probe, "http", side_effect=[response]) as http,
                      patch.object(self.probe, "run", return_value="Locking 0 packages")):
                    cargo.check_crates(area, self.probe, self.tracked)
                http.assert_called_once_with("https://index.crates.io/le/mm/lemma", "text/plain", batch=ANY)
                self.assertEqual(area.counts, {"inconclusive": 1})
                self.assertEqual([(row.item, row.context, row.observed, row.detail) for row in area.findings],
                                 [("lemma", "top", "1.2.3-rc.1", reason)])
                self.assertIn(f"lemma 1.2.3-rc.1 ({reason})", rendered(area))

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

    def test_python_package_pin_review_uses_the_pin_when_installation_is_newer(self):
        self.write("book/requirements.txt", "playwright==1.63.0\n")
        _, _, Version = python_packages.packaging_api()

        def releases(_probe, name, _version):
            return [Version("1.64.0"), Version("1.63.0")] if name == "playwright" else [Version("25.0")]

        area = Area("Python")
        with (patch.object(python_packages.metadata, "distribution", return_value=SimpleNamespace(version="1.65.0")),
              patch.object(python_packages, "pip_index", side_effect=releases),
              patch.object(self.probe, "run", return_value="pip 25.0 from installed pip")):
            python_packages.check_python_packages(area, self.probe, self.tracked)
        result = rendered(area)
        self.assertIn("Requires changing project requirements\n    playwright 1.63.0 -> 1.64.0 (project pin)", result)
        self.assertIn("Installation or lockfile differences\n    playwright 1.65.0", result)
        self.assertEqual(area.counts, {"pin review": 1, "drift": 1, "current": 1})

    def test_graphify_pin_review_uses_the_pin_when_installation_is_newer(self):
        self.write("tools/graphify/refresh.py", 'GRAPHIFY_VERSION = "0.9.74"\n')
        _, _, Version = python_packages.packaging_api()
        area = Area("Setup")
        with (patch.object(setup_tools.metadata, "version", return_value="0.9.76"),
              patch.object(setup_tools, "pip_index", return_value=[Version("0.9.75")])):
            setup_tools.check_graphify(area, self.probe)
        result = rendered(area)
        self.assertIn("Requires changing project requirements\n    Graphify 0.9.74 -> 0.9.75 (project pin)", result)
        self.assertIn("Installation or lockfile differences\n    Graphify 0.9.76", result)
        self.assertEqual(area.counts, {"pin review": 1, "drift": 1})

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
              patch.object(node_packages, "npm_runtime", return_value=("10.0.0", "24.1.0", "node", "npm-cli.js", "npm")),
              patch.object(self.probe, "http", side_effect=ProbeError("fixture registry unavailable"))):
            node_packages.check_node_packages(area, self.probe, set())
        self.assertIn("Could not check\n    npm 10.0.0 (fixture registry unavailable)", rendered(area))
        self.assertNotIn("npm uses Node.js", rendered(area))

    def npm_fixture_environment(self):
        if not shutil.which("npm.cmd" if sys.platform == "win32" else "npm"):
            self.skipTest("npm is required to read its configuration and interpret package specs")
        userconfig = self.write("user.npmrc", "")
        globalconfig = self.write("global.npmrc", "")
        env = {key: value for key, value in os.environ.items() if not key.lower().startswith("npm_config_")}
        return {**env, "npm_config_userconfig": str(userconfig), "npm_config_globalconfig": str(globalconfig)}

    def write_book_npm_inputs(self):
        directory = "tools/bootstrap/book-to-markdown"
        for name in ("package.json", "package-lock.json"):
            path = f"{directory}/{name}"
            self.write(path, (check_updates.ROOT / path).read_text(encoding="utf-8"))
        return directory

    def test_npm_does_not_query_public_package_when_config_replaces_the_lock_registry(self):
        env = self.npm_fixture_environment()
        directory = self.write_book_npm_inputs()
        package = "https://registry.npmjs.org/katex"
        private = "https://mirror.example.invalid/npm/"
        for setting in ("project npmrc", "environment", "protected credentials"):
            with self.subTest(setting=setting):
                registry = ("https://fixture-user:fixture-password@mirror.example.invalid/npm/"
                            if setting == "protected credentials" else private)
                configured = dict(env)
                if setting != "project npmrc":
                    configured["npm_config_registry"] = registry
                self.write(f"{directory}/.npmrc", f"registry={private}\n" if setting == "project npmrc" else "")
                area = Area("Node.js and npm")
                with (patch.dict(os.environ, configured, clear=True),
                      patch.object(node_packages, "check_node"),
                      patch.object(self.probe, "http", return_value={"name": "npm", "versions": {"99.0.0": {}}})
                      as http):
                    node_packages.check_node_packages(area, self.probe, self.tracked)
                self.assertNotIn(package, [call.args[0] for call in http.call_args_list])
                http.assert_called_once_with("https://registry.npmjs.org/npm", "application/vnd.npm.install-v1+json")
                expected = "Could not check\n    katex 0.18.2 ("
                if setting != "protected credentials":
                    expected += "the configured npm registry needs a separate check)"
                self.assertIn(expected, rendered(area))
                self.assertNotIn("fixture-password", rendered(area))

    def test_npm_does_not_query_public_package_for_new_direct_sources_with_the_old_lock(self):
        env = self.npm_fixture_environment()
        directory = self.write_book_npm_inputs()
        for spec in ("https://fixture-user:fixture-password@packages.example.invalid/katex.tgz",
                     "git+https://fixture-user:fixture-password@git.example.invalid/katex.git#main", "1.2.3-fixture.tgz"):
            with self.subTest(scheme=spec.split(":", 1)[0]):
                self.write_json(f"{directory}/package.json", {"dependencies": {"katex": spec}})
                area = Area("Node.js and npm")
                with (patch.dict(os.environ, env, clear=True), patch.object(node_packages, "check_node"),
                      patch.object(self.probe, "http", return_value={"name": "npm", "versions": {"99.0.0": {}}})
                      as http):
                    node_packages.check_node_packages(area, self.probe, self.tracked)
                http.assert_called_once_with("https://registry.npmjs.org/npm", "application/vnd.npm.install-v1+json")
                self.assertIn("Could not check\n    katex 0.18.2 (the version or dependency source needs a separate check)",
                              rendered(area))
                self.assertIsNone(next(row for row in area.findings if row.item == "katex").declared)
                self.assertNotIn("fixture-password", rendered(area))

    def test_npm_treats_the_default_https_port_as_the_public_registry(self):
        env = self.npm_fixture_environment()
        directory = self.write_book_npm_inputs()
        lockfile = self.root / directory / "package-lock.json"
        original_lock = lockfile.read_text(encoding="utf-8")
        public = "https://registry.npmjs.org/"
        with_port = "https://registry.npmjs.org:443/"

        def http(url, _accept):
            name = url.rsplit("/", 1)[-1]
            return {"name": name, "versions": {"99.0.0": {}}}

        for registry, locked_registry in ((with_port, public), (public, with_port)):
            with self.subTest(registry=registry, locked_registry=locked_registry):
                self.write(f"{directory}/.npmrc", f"registry={registry}\n")
                self.write(f"{directory}/package-lock.json", original_lock.replace(public, locked_registry))
                area = Area("Node.js and npm")
                with (patch.dict(os.environ, env, clear=True), patch.object(node_packages, "check_node"),
                      patch.object(self.probe, "http", side_effect=http) as requests):
                    node_packages.check_node_packages(area, self.probe, self.tracked)
                self.assertCountEqual([call.args[0] for call in requests.call_args_list],
                                      [public + "npm", public + "katex"])
                self.assertIn("katex 0.18.2 -> 99.0.0", rendered(area))

    def test_npm_checks_scoped_packages_only_when_that_scope_uses_the_public_registry(self):
        env = self.npm_fixture_environment()
        self.write("tools/book/.npmrc", "@mirror:registry=https://mirror.example.invalid/npm/\n")
        dependencies = {"@mirror/widget": "1.0.0", "@public/widget": "1.0.0"}
        self.write_json("tools/book/package.json", {"dependencies": dependencies})
        self.write_json("tools/book/package-lock.json", {"packages": {
            f"node_modules/{name}": {"version": version,
                                     "resolved": f"https://registry.npmjs.org/{name}/-/widget-{version}.tgz"}
            for name, version in dependencies.items()
        }})
        area = Area("Node.js and npm")

        def http(url, _accept):
            name = "npm" if url.endswith("/npm") else "@public/widget"
            return {"name": name, "versions": {"100.0.0": {}}}

        with (patch.dict(os.environ, env, clear=True), patch.object(node_packages, "check_node"),
              patch.object(self.probe, "http", side_effect=http) as requests):
            node_packages.check_node_packages(area, self.probe, self.tracked)
        self.assertCountEqual([call.args[0] for call in requests.call_args_list],
                              ["https://registry.npmjs.org/npm", "https://registry.npmjs.org/%40public%2Fwidget"])
        report = rendered(area)
        self.assertIn("@mirror/widget 1.0.0 (the configured npm registry needs a separate check)", report)
        self.assertIn("@public/widget 1.0.0 -> 100.0.0", report)

    def test_npm_requires_public_scope_metadata_and_a_public_locked_source(self):
        env = self.npm_fixture_environment()
        public = "https://registry.npmjs.org/"
        private = "https://mirror.example.invalid/npm/"
        locked_registries = {"@public/widget": public, "@public/other": private, "@mirror/widget": public}
        self.write_json("tools/book/package.json", {"dependencies": dict.fromkeys(locked_registries, "1.0.0")})
        self.write_json("tools/book/package-lock.json", {"packages": {
            f"node_modules/{name}": {"version": "1.0.0", "resolved": f"{registry}{name}/-/widget-1.0.0.tgz"}
            for name, registry in locked_registries.items()
        }})

        def http(url, _accept):
            name = "npm" if url.endswith("/npm") else "@public/widget"
            return {"name": name, "versions": {"99.0.0": {}}}

        for replacement in ("never", "npmjs", "always"):
            with self.subTest(replacement=replacement):
                self.write("tools/book/.npmrc", f"registry={private}\n@public:registry={public}\n"
                                              f"replace-registry-host={replacement}\n")
                area = Area("Node.js and npm")
                with (patch.dict(os.environ, env, clear=True), patch.object(node_packages, "check_node"),
                      patch.object(self.probe, "http", side_effect=http) as requests):
                    node_packages.check_node_packages(area, self.probe, self.tracked)
                expected = [public + "npm"]
                if replacement == "never":
                    expected.append(public + "%40public%2Fwidget")
                    self.assertIn("@public/widget 1.0.0 -> 99.0.0", rendered(area))
                else:
                    self.assertIn("@public/widget 1.0.0 (the configured npm registry needs a separate check)",
                                  rendered(area))
                self.assertCountEqual([call.args[0] for call in requests.call_args_list], expected)
                self.assertIn("@public/other 1.0.0 (check the declared package source separately)", rendered(area))
                self.assertIn("@mirror/widget 1.0.0 (the configured npm registry needs a separate check)", rendered(area))

    def test_npm_queries_only_packages_declared_in_manifest_without_node_modules(self):
        env = self.npm_fixture_environment()
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
        with (patch.dict(os.environ, env, clear=True), patch.object(self.probe, "http", side_effect=http) as requests,
              patch.object(node_packages, "check_node")):
            node_packages.check_node_packages(area, self.probe, self.tracked)
        self.assertCountEqual([call.args[0] for call in requests.call_args_list],
                              ["https://registry.npmjs.org/npm", "https://registry.npmjs.org/katex"])
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
        node = shutil.which("node")
        if node is None:
            self.skipTest("Node.js is required to run the Playwright driver fixture")
        edge = self.write("Edge/msedge.exe", "")
        chromium = self.root / "Chromium/chrome.exe"
        paths = json.dumps({"msedge": str(edge), "chromium": str(chromium)})
        self.write("playwright/driver/package/lib/coreBundle.js", f"""
const paths = {paths};
exports.registry = {{
    findExecutable(channel) {{
        return {{ executablePath: () => paths[channel] }};
    }}
}};
""")
        playwright = SimpleNamespace(locate_file=lambda path: self.root / path)
        releases = [
            {"Product": "Beta", "Releases": [
                {"Platform": "Windows", "Architecture": "x64", "ProductVersion": "151.0.1.2"},
            ]},
            {"Product": "Stable", "Releases": [
                {"Platform": "Windows", "Architecture": "arm64", "ProductVersion": "150.0.1.5"},
                {"Platform": "MacOS", "Architecture": "universal", "ProductVersion": "150.0.1.4"},
                {"Platform": "Windows", "Architecture": "x86", "ProductVersion": "150.0.1.3"},
                {"Platform": "Windows", "Architecture": "x64", "ProductVersion": "150.0.1.2"},
            ]},
        ]
        area = Area("Book Review browsers")
        with (patch.dict(os.environ, {"PLAYWRIGHT_NODEJS_PATH": node}),
              patch.object(browsers.metadata, "distribution", return_value=playwright),
              patch.object(browsers, "edge_installed_version", return_value=((150, 0, 1, 2), "windows", "x64"))
              as read_version,
              patch.object(self.probe, "http", return_value=releases) as requests):
            browsers.check_browsers(area, self.probe, self.tracked)
        read_version.assert_called_once_with(self.probe, edge)
        requests.assert_called_once_with(browsers.EDGE_RELEASES)
        self.assertEqual(area.counts, {"current": 1})
        self.assertEqual(rendered(area), "  No findings.\n")

    def test_edge_uses_the_newest_release_for_the_matching_stable_platform_and_architecture(self):
        edge = self.write("Edge/msedge.exe", "")
        releases = [
            {"Product": "Dev", "Releases": [
                {"Platform": "Windows", "Architecture": "x64", "ProductVersion": "152.0.2.1"},
            ]},
            {"Product": "Stable", "Releases": [
                {"Platform": "Windows", "Architecture": "x64", "ProductVersion": "150.0.1.2"},
                {"Platform": "Windows", "Architecture": "x64", "ProductVersion": "150.0.1.3"},
                {"Platform": "Linux", "Architecture": "x64", "ProductVersion": "150.0.1.4"},
                {"Platform": "Windows", "Architecture": "arm64", "ProductVersion": "150.0.1.5"},
            ]},
        ]
        area = Area("Book Review browsers")
        with (patch.object(browsers, "edge_installed_version", return_value=((150, 0, 1, 1), "windows", "x64")),
              patch.object(self.probe, "http", return_value=releases) as requests):
            browsers.check_edge(area, self.probe, edge)
        requests.assert_called_once_with("https://edgeupdates.microsoft.com/api/products")
        self.assertEqual(area.counts, {"update": 1})
        self.assertIn("Microsoft Edge Stable 150.0.1.1 -> 150.0.1.3", rendered(area))

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
        host = "x86_64-pc-windows-msvc"
        local = [f"rustc 1.99.0 (abc123 2026-10-01)\nhost: {host}",
                 f"rustc 1.102.0-nightly (aabbcc 2026-11-13)\nhost: {host}"]
        remote = ['[pkg.rust]\nversion = "1.100.0 (def456 2026-11-09)"\n',
                  '[pkg.rust]\nversion = "1.102.0-nightly (ddeeff 2026-11-15)"\n']
        with (patch.object(self.probe, "run", side_effect=local) as run,
              patch.object(self.probe, "http", side_effect=remote) as http,
              patch.object(setup_tools, "rust_components")):
            setup_tools.check_rust_toolchain(area, self.probe, stable, native=True, browser=False)
            setup_tools.check_rust_toolchain(area, self.probe, nightly, native=False, browser=True)
        self.assertEqual([call.args[0] for call in run.call_args_list], [
            ["rustup", "run", stable, "rustc", "--version", "--verbose"],
            ["rustup", "run", nightly, "rustc", "--version", "--verbose"],
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

    def rust_role_report(self, native, browser, versions, components, targets):
        area = Area("Rust toolchains and Cargo tools")

        def run(command, *, env=None):
            if command == ["rustup", "show", "active-toolchain"]:
                self.assertIn(env, (None, {"RUSTUP_TOOLCHAIN": "nightly"}))
                selection = browser if env else native
                if isinstance(selection, ProbeError):
                    raise selection
                return selection
            if command[:2] == ["rustup", "run"]:
                selected = command[2]
                self.assertIn(selected, versions)
                if command[3] == "rustc":
                    self.assertEqual(command[4:], ["--version", "--verbose"])
                    return f"rustc {versions[selected]}\nhost: x86_64-pc-windows-msvc"
                return ""
            if command[:1] == ["rustup"] and command[1] in {"component", "target"}:
                self.assertEqual(command[2:-1], ["list", "--installed", "--toolchain"])
                installed = components if command[1] == "component" else targets
                return "\n".join(installed[command[-1]])
            self.fail(f"unexpected command {command!r}")

        def release(url, accept):
            self.assertEqual(accept, "text/plain")
            version = {
                "https://static.rust-lang.org/dist/channel-rust-stable.toml": "1.100.0",
                "https://static.rust-lang.org/dist/channel-rust-nightly.toml": "1.102.0-nightly (ddeeff 2026-11-15)",
            }[url]
            return f'[pkg.rust]\nversion = "{version}"\n'

        with (patch.object(self.probe, "run", side_effect=run) as commands,
              patch.object(self.probe, "http", side_effect=release) as requests,
              patch.object(setup_tools, "check_rustup"),
              patch.object(setup_tools, "check_tool_release", return_value=setup_tools.NEXTEST_BASELINE)):
            setup_tools.check_rust(area, self.probe, self.tracked)
        releases = [call.args[0] for call in requests.call_args_list]
        selection_environments = [call.kwargs.get("env") for call in commands.call_args_list
                                  if call.args[0] == ["rustup", "show", "active-toolchain"]]
        self.assertEqual(selection_environments, [None, {"RUSTUP_TOOLCHAIN": "nightly"}])
        return area, [call.args[0] for call in commands.call_args_list], releases

    def test_rust_probes_only_the_selected_native_and_browser_toolchains(self):
        host = "x86_64-pc-windows-msvc"
        stable = f"stable-{host}"
        nightly = f"nightly-{host}"
        other_stable = "stable-i686-pc-windows-msvc"
        dated = f"nightly-2026-11-11-{host}"
        versions = {
            stable: "1.100.0",
            nightly: "1.102.0-nightly (aabbcc 2026-11-13)",
            other_stable: "1.99.0",
            dated: "1.102.0-nightly (d0d0d0 2026-11-10)",
        }
        area, commands, releases = self.rust_role_report(
            f"{stable} (default)", f"{nightly} (overridden by environment variable RUSTUP_TOOLCHAIN)",
            {**versions, "stable": versions[other_stable], "nightly": versions[dated]},
            {stable: ["cargo", "clippy", "rustfmt", "llvm-tools-preview"], nightly: ["cargo", "rust-src"],
             "stable": [], "nightly": []},
            {stable: [host], nightly: ["wasm32-unknown-unknown"], "stable": [], "nightly": []},
        )

        self.assertCountEqual([tuple(command) for command in commands], [
            ("rustup", "show", "active-toolchain"),
            ("rustup", "show", "active-toolchain"),
            ("rustup", "run", stable, "rustc", "--version", "--verbose"),
            ("rustup", "component", "list", "--installed", "--toolchain", stable),
            ("rustup", "target", "list", "--installed", "--toolchain", stable),
            ("rustup", "run", stable, "cargo", "--version"),
            ("rustup", "run", stable, "cargo", "clippy", "--version"),
            ("rustup", "run", stable, "rustfmt", "--version"),
            ("rustup", "run", nightly, "rustc", "--version", "--verbose"),
            ("rustup", "component", "list", "--installed", "--toolchain", nightly),
            ("rustup", "target", "list", "--installed", "--toolchain", nightly),
            ("rustup", "run", nightly, "cargo", "--version"),
        ])
        self.assertEqual(releases, [
            "https://static.rust-lang.org/dist/channel-rust-stable.toml",
            "https://static.rust-lang.org/dist/channel-rust-nightly.toml",
        ])
        self.assertEqual([(row.item, row.status) for row in area.findings], [("Rust Nightly", "update")])
        report = rendered(area)
        self.assertNotIn("1.100.0", report)
        self.assertIn("Rust Nightly 1.102.0-nightly (2026-11-13) -> 1.102.0-nightly (2026-11-15)", report)

    def test_rust_checks_combined_requirements_once_for_a_shared_nightly(self):
        nightly = "nightly-x86_64-pc-windows-msvc"
        area, commands, releases = self.rust_role_report(
            f"{nightly} (default)", f"{nightly} (overridden by environment variable RUSTUP_TOOLCHAIN)",
            {nightly: "1.102.0-nightly (aabbcc 2026-11-13)"}, {nightly: ["rustfmt"]}, {nightly: []},
        )
        self.assertEqual(releases, ["https://static.rust-lang.org/dist/channel-rust-nightly.toml"])
        self.assertEqual(commands.count(["rustup", "run", nightly, "rustc", "--version", "--verbose"]), 1)
        self.assertEqual(commands.count(["rustup", "run", nightly, "rustfmt", "--version"]), 1)
        self.assertEqual(commands.count(["rustup", "component", "list", "--installed", "--toolchain", nightly]), 1)
        self.assertEqual(commands.count(["rustup", "target", "list", "--installed", "--toolchain", nightly]), 1)
        self.assertCountEqual([(row.status, row.item) for row in area.findings], [
            ("update", "Rust Nightly"),
            ("missing", "Rust Nightly cargo"),
            ("missing", "Rust Nightly clippy"),
            ("missing", "Rust Nightly llvm-tools-preview"),
            ("missing", "Rust Nightly rust-src"),
            ("missing", "Rust Nightly target wasm32-unknown-unknown"),
            ("missing", "Rust Nightly target x86_64-pc-windows-msvc"),
        ])
        self.assertEqual(rendered(area).count("Rust Nightly cargo not installed"), 1)

    def test_rust_compares_browser_nightly_independently_of_a_pinned_or_custom_native_toolchain(self):
        nightly = "nightly-x86_64-pc-windows-msvc"
        host = "x86_64-pc-windows-msvc"
        for native, compiler in ((f"1.98.0-{host}", "1.98.0"), ("linked build", "1.98.0-dev+branch.1")):
            with self.subTest(native=native):
                area, commands, releases = self.rust_role_report(
                    f"{native} (default)", nightly,
                    {native: compiler, nightly: "1.102.0-nightly (aabbcc 2026-11-13)"},
                    {native: ["cargo", "clippy", "rustfmt", "llvm-tools-preview"], nightly: ["cargo", "rust-src"]},
                    {native: [host], nightly: ["wasm32-unknown-unknown"]},
                )
                self.assertEqual(releases, ["https://static.rust-lang.org/dist/channel-rust-nightly.toml"])
                self.assertIn(["rustup", "run", native, "cargo", "--version"], commands)
                self.assertIn(["rustup", "target", "list", "--installed", "--toolchain", native], commands)
                self.assertCountEqual([(row.status, row.item, row.observed) for row in area.findings], [
                    ("inconclusive", f"Rust native toolchain {native}", compiler),
                    ("update", "Rust Nightly", "1.102.0-nightly (2026-11-13)"),
                ])
                self.assertIn("this selection has no rolling Stable or Nightly release comparison", rendered(area))

    def test_rust_keeps_the_other_toolchain_result_after_an_unavailable_selection(self):
        stable = "stable-x86_64-pc-windows-msvc"
        nightly = "nightly-x86_64-pc-windows-msvc"
        absent = CommandFailed(
            1, f"error: override toolchain '{nightly}' is not installed: "
               "the RUSTUP_TOOLCHAIN environment variable specifies an uninstalled toolchain\n",
        )
        for native, browser, successful, channel, unavailable, status in (
            (stable, absent, stable, "stable", "browser Nightly", "missing"),
            (CommandFailed(1, "private fixture diagnostic"), nightly, nightly, "nightly", "native", "inconclusive"),
            (f"{stable}\n{nightly}", nightly, nightly, "nightly", "native", "inconclusive"),
        ):
            with self.subTest(native=native, browser=browser):
                area, commands, releases = self.rust_role_report(
                    native, browser,
                    {successful: "1.99.0" if channel == "stable" else "1.102.0-nightly (aabbcc 2026-11-13)"},
                    {successful: ["cargo", "clippy", "rustfmt", "llvm-tools-preview", "rust-src"]},
                    {successful: ["x86_64-pc-windows-msvc", "wasm32-unknown-unknown"]},
                )
                self.assertEqual(releases, [f"https://static.rust-lang.org/dist/channel-rust-{channel}.toml"])
                self.assertEqual(commands.count(["rustup", "show", "active-toolchain"]), 2)
                self.assertCountEqual([(row.status, row.item) for row in area.findings], [
                    (status, f"Rust {unavailable} toolchain"), ("update", f"Rust {channel.title()}"),
                ])
                self.assertNotIn("private fixture diagnostic", rendered(area))

    def test_rust_stable_counts_current_only_after_a_comparable_release_response(self):
        stable = "stable-x86_64-pc-windows-msvc"
        release = '[pkg.rust]\nversion = "1.100.0"\n'
        for label, installed, response, status in (
            ("equal", "rustc 1.100.0", release, "current"),
            ("local newer", "rustc 1.101.0", release, "current"),
            ("prerelease", "rustc 1.100.0", '[pkg.rust]\nversion = "1.102.0-nightly"\n', "inconclusive"),
            ("malformed", "rustc 1.100.0", "not valid TOML", "inconclusive"),
            ("offline", "rustc 1.100.0", ProbeError("fixture source unavailable"), "inconclusive"),
            ("deadline", "rustc 1.100.0", common.HttpDeadline(7), "inconclusive"),
        ):
            with (self.subTest(label=label),
                  patch.object(self.probe, "run", return_value=f"{installed}\nhost: x86_64-pc-windows-msvc"),
                  patch.object(self.probe, "http", side_effect=[response]),
                  patch.object(setup_tools, "rust_components") as components):
                area = Area("Rust")
                setup_tools.check_rust_toolchain(area, self.probe, stable, native=True, browser=False)
                self.assertEqual(area.counts, {status: 1})
                if status == "inconclusive":
                    self.assertIn("Could not check\n    Rust Stable 1.100.0", rendered(area))
                components.assert_called_once_with(
                    area, self.probe, stable, "Rust Stable", "x86_64-pc-windows-msvc",
                    True, False, setup_tools.NATIVE_RUST_SETUP,
                )

    def test_nextest_compares_the_installation_with_historical_and_upstream_versions_independently(self):
        nextest_command = ["cargo", "nextest", "--version"]
        nextest_url = "https://api.github.com/repos/nextest-rs/nextest/releases/latest"
        for installed, status, candidate in (("0.9.152", "update", "0.9.170"),
                                             ("0.9.170", "baseline difference", None)):
            with self.subTest(installed=installed):
                area = Area("Rust toolchains and Cargo tools")

                def run(command, **_options):
                    if command == ["trunk", "--version"] or command == ["cargo", "llvm-cov", "--version"]:
                        return "1.0.0"
                    self.assertEqual(command, nextest_command)
                    return f"cargo-nextest {installed}"

                def http(url):
                    tag = "cargo-nextest-0.9.170" if url == nextest_url else "v1.0.0"
                    return {"tag_name": tag, "draft": False, "prerelease": False}

                with (patch.object(self.probe, "run", side_effect=run) as commands,
                      patch.object(self.probe, "http", side_effect=http) as requests,
                      patch.object(setup_tools, "rust_selection", return_value=None),
                      patch.object(setup_tools, "check_rustup")):
                    setup_tools.check_rust(area, self.probe, self.tracked)
                self.assertEqual([call.args[0] for call in commands.call_args_list].count(nextest_command), 1)
                self.assertEqual([call.args[0] for call in requests.call_args_list].count(nextest_url), 1)
                rows = [row for row in area.findings if row.item.startswith("cargo-nextest")]
                self.assertEqual([(row.status, row.observed, row.candidate) for row in rows],
                                 [(status, installed, candidate)])
                if candidate is None:
                    self.assertIn("Historical version differences\n    cargo-nextest 0.9.170 (historical version 0.9.152)",
                                  rendered(area))
                else:
                    self.assertIn("Updates available\n    cargo-nextest 0.9.152 -> 0.9.170", rendered(area))

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

        def http(url, *_args, **_kwargs):
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
        self.assertIn("Could not check\n    lemma (top) 1.0.0 (fixture source returned HTTP 503)", report)
        self.assertIn("Updates within current requirements\n    lemma 1.0.0 -> 1.0.1", report)
        self.assertIn(f" -> {python_candidate} (selected release series)", report)
        self.assertIn("Partial report", report)
        self.assertNotIn("Report complete", report)
        self.assertNotIn("Coverage", report)
        self.assertNotIn("\x1b[", report)

    def test_crates_index_deadline_exits_cli_while_an_earlier_lookup_is_pending(self):
        self.write("Cargo.toml", '''
[package]
name = "top"
[dependencies]
aahelp = "1"
zzlate = "1"
''')
        self.write("Cargo.lock", '''
[[package]]
name = "top"
version = "0.1.0"
dependencies = ["aahelp", "zzlate"]
[[package]]
name = "aahelp"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
[[package]]
name = "zzlate"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
''')
        primary_started = Event()
        sibling_started = Event()
        sibling_disconnected = Event()

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.send_header("Content-Length", "100")
                self.end_headers()
                self.wfile.write(b"x")
                self.connection.settimeout(10)
                is_sibling = self.path == "/aa/he/aahelp"
                (sibling_started if is_sibling else primary_started).set()
                try:
                    disconnected = not self.connection.recv(1)
                except ConnectionResetError:
                    disconnected = True
                if is_sibling and disconnected:
                    sibling_disconnected.set()

            def log_message(self, *_args):
                pass

        script = '''
from pathlib import Path
import sys
from threading import Event
from time import sleep

sys.path.insert(0, sys.argv[1])
import check_updates
from dependency_updates import cargo, common

check_updates.ROOT = Path(sys.argv[2])
cargo.CRATES_INDEX = sys.argv[3]
common.HTTP_DEADLINE = 3.5
check_updates.check_rust = lambda *_: None
cargo.cargo_configuration = lambda *_: False
probe = common.Probes(check_updates.ROOT)
probe.tracked_files = lambda: {"Cargo.toml", "Cargo.lock"}
http = probe.http
primary_lookup = Event()

def stagger(url, *args, **kwargs):
    if url.endswith("/zzlate"):
        primary_lookup.set()
    elif url.endswith("/aahelp"):
        if not primary_lookup.wait(3):
            raise AssertionError("the primary lookup did not start")
        sleep(2)
    return http(url, *args, **kwargs)

def command(command, **_kwargs):
    print(f"unexpected probe command: {command}", flush=True)
    raise common.ProbeError("fixture command unavailable")

probe.http = stagger
probe.run = command
check_updates.Probes = lambda _: probe
check_updates.check_python = lambda *_: print("unexpected Python area", flush=True)
for name in ("check_browsers", "check_node_packages", "check_setup", "check_docker"):
    setattr(check_updates, name, check_updates.check_python)
sys.exit(check_updates.main([]))
'''
        tools = Path(check_updates.__file__).parent
        with serve_http(Handler) as url:
            command = [sys.executable, "-B", "-c", script, str(tools), str(self.root), url]
            with subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  encoding="utf-8") as checker:
                try:
                    self.assertTrue(primary_started.wait(8), "the lookup that reaches its deadline must start")
                    self.assertTrue(sibling_started.wait(8), "an earlier lookup must already be in flight")
                    started = monotonic()
                    stdout, stderr = checker.communicate(timeout=10)
                    pending_time = monotonic() - started
                    self.assertTrue(sibling_disconnected.wait(1), "the pending lookup's connection must close")
                finally:
                    if checker.poll() is None:
                        checker.kill()
                        checker.communicate()

        self.assertLess(pending_time, 2.8, f"checker waited for the sibling lookup: {stdout!r}, {stderr!r}")
        self.assertNotEqual(checker.returncode, 0)
        self.assertIn("Cargo", stderr)
        self.assertIn("crates.io", stderr)
        self.assertIn("timed out after 3.5 seconds", stderr)
        self.assertNotIn(url, stdout + stderr)
        self.assertIn("Checking direct crates on crates.io...", stdout)
        self.assertNotIn("Checking what Cargo can resolve", stdout)
        self.assertNotIn("unexpected probe command", stdout)
        self.assertNotIn("Checking Python and pip", stdout)
        self.assertNotIn("unexpected Python area", stdout)
        self.assertNotIn("Partial report.", stdout)
        self.assertNotIn("Report complete.", stdout)

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
                host = "x86_64-pc-windows-msvc"
                local = [f"rustc 1.100.0\nhost: {host}", f"rustc {local_nightly}\nhost: {host}"]
                remote = ['[pkg.rust]\nversion = "1.100.0"\n', f'[pkg.rust]\nversion = "{remote_nightly}"\n']
                with (patch.object(self.probe, "run", side_effect=local),
                      patch.object(self.probe, "http", side_effect=remote),
                      patch.object(setup_tools, "rust_components")):
                    setup_tools.check_rust_toolchain(area, self.probe, stable, native=True, browser=False)
                    setup_tools.check_rust_toolchain(area, self.probe, nightly, native=False, browser=True)
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
