"""Exercise the Windows controller with a disposable Docker client and listener."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.request import urlopen


TOOL = Path(__file__).resolve().parents[1]
POWERSHELL = shutil.which("powershell.exe")
AUTH_CONTENTS = "private-fixture-credential"


def fake_docker(arguments):
    state = Path(os.environ["REVIEW_TEST_STATE"])
    container = state / "container.json"
    with (state / "calls.jsonl").open("a", encoding="utf-8") as calls:
        calls.write(json.dumps(arguments) + "\n")

    command = arguments[0]
    if command == "ps":
        if (state / "unavailable").exists():
            print("Cannot connect to the fixture Docker endpoint", file=sys.stderr)
            return 7
        siblings = state / "siblings.json"
        records = json.loads(siblings.read_text(encoding="utf-8")) if siblings.exists() else []
        if container.exists():
            records.append(json.loads(container.read_text(encoding="utf-8")))
        filters = [value.removeprefix("label=").split("=", 1)
                   for option, value in zip(arguments, arguments[1:]) if option == "--filter"]
        for record in records:
            if all(record["Config"]["Labels"].get(key) == value for key, value in filters):
                print(record["Id"])
    elif command == "inspect":
        print("[" + container.read_text(encoding="utf-8") + "]")
    elif command == "compose":
        project = arguments[arguments.index("--project-name") + 1]
        files = [arguments[index + 1] for index, value in enumerate(arguments) if value == "--file"]
        if len(files) != 2 or arguments[-1] != "book-review":
            return 99
        base, override = map(Path, files)
        if base.name != "compose.yaml" or not base.is_file() or not override.is_file():
            return 98
        if "rm" in arguments:
            container.unlink()
        elif "up" in arguments:
            if "--remove-orphans" in arguments or os.environ.get("COMPOSE_REMOVE_ORPHANS") == "true":
                return 95
            previous = json.loads(container.read_text(encoding="utf-8"))["Id"] if container.exists() else None
            generation = "two" if previous else "one"
            port = os.environ["BOOK_REVIEW_PORT"]
            mounts = [
                ("BOOK_REVIEW_LIBRARY", "/library", False),
                ("BOOK_REVIEW_CONFIG", "/app/tools/bootstrap/book-to-markdown/config.yaml", False),
                ("BOOK_REVIEW_DIAGNOSTICS", "/app/.agent/tmp", True),
                ("BOOK_REVIEW_AUTH", "/run/secrets/book-review-auth", False),
            ]
            record = {
                "Id": generation,
                "Config": {"Labels": {
                    "com.docker.compose.project": project,
                    "com.docker.compose.service": "book-review",
                    "com.docker.compose.oneoff": "False",
                    "com.docker.compose.project.config_files": ",".join(files),
                }},
                "State": {"Running": True},
                "HostConfig": {"PortBindings": {"8765/tcp": [{"HostIp": "127.0.0.1", "HostPort": port}]}},
                "Mounts": [
                    {"Type": "bind", "Source": os.environ[variable], "Destination": target, "RW": writable}
                    for variable, target, writable in mounts if os.environ.get(variable)
                ],
            }
            for mount in json.loads(override.read_text(encoding="utf-8"))["services"]["book-review"]["volumes"]:
                if mount["target"].startswith("/library/"):
                    record["Mounts"].append({
                        "Type": "bind", "Source": mount["source"], "Destination": mount["target"],
                        "RW": not mount["read_only"],
                    })
            container.write_text(json.dumps(record), encoding="utf-8")
        else:
            return 97
    else:
        return 96
    return 0


class ReviewHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        payload = (b'{"books":["trial"],"reviews":[{"id":"trial","pending":true,"error":null}]}'
                   if self.path == "/api/status" else b"Book Review fixture")
        self.send_response(200)
        self.send_header("Content-Type", "application/json" if self.path == "/api/status" else "text/plain")
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *args):
        pass


@unittest.skipUnless(os.name == "nt" and POWERSHELL, "Windows PowerShell is required")
class ReviewControllerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="Book Review fixture ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.tool = self.root / "tools" / "bootstrap" / "book-to-markdown"
        self.tool.mkdir(parents=True)
        self.script = self.tool / "review-server.ps1"
        for name in ("review-server.ps1", "compose.yaml", "compose-mounts.py", "config.yaml"):
            shutil.copyfile(TOOL / name, self.tool / name)
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True, capture_output=True)

        self.state = self.root / "fake docker"
        self.state.mkdir()
        (self.state / "docker.cmd").write_text(
            '@echo off\n"%REVIEW_TEST_PYTHON%" "%REVIEW_TEST_SCRIPT%" --fake-docker %*\n', encoding="ascii"
        )
        self.container = self.state / "container.json"
        self.library = self.root / "imported books"
        self.diagnostics = self.root / "diagnostic files"
        self.diagnostics.mkdir()
        self.source = self.library / "trial" / "variant" / "source.pdf"
        self.source.parent.mkdir(parents=True)
        self.source.write_bytes(b"fixture source")
        working = self.source.parent / "working"
        working.mkdir()
        self.recipe = working / "book.json"
        self.recipe.write_text("{}", encoding="utf-8")
        self.auth = self.root / "private auth.json"
        self.auth.write_text(AUTH_CONTENTS, encoding="utf-8")
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            self.port = listener.getsockname()[1]

        self.environment = dict(os.environ, PATH=str(self.state) + os.pathsep + os.environ["PATH"],
                                REVIEW_TEST_STATE=str(self.state),
                                REVIEW_TEST_PYTHON=sys.executable, REVIEW_TEST_SCRIPT=str(Path(__file__).resolve()),
                                BOOK_REVIEW_LIBRARY=self.library.name, BOOK_REVIEW_CONFIG="",
                                BOOK_REVIEW_AUTH=self.auth.name,
                                BOOK_REVIEW_DIAGNOSTICS=self.diagnostics.name, BOOK_REVIEW_PORT=str(self.port))

    def invoke(self, *arguments):
        return subprocess.run(
            [POWERSHELL, "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File",
             str(self.script), *arguments], cwd=self.root, env=self.environment, capture_output=True,
            text=True, errors="replace", timeout=45,
        )

    def require_success(self, *arguments):
        result = self.invoke(*arguments)
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertNotIn(AUTH_CONTENTS, result.stdout + result.stderr)
        return result.stdout

    def calls(self):
        history = self.state / "calls.jsonl"
        return [json.loads(line) for line in history.read_text(encoding="utf-8").splitlines()] if history.exists() else []

    def test_no_arguments_and_invalid_port_have_no_side_effects(self):
        self.assertIn("Usage:", self.require_success())
        result = self.invoke("start", "-Port", "0")
        self.assertNotEqual(0, result.returncode)
        self.assertIn("Port", result.stderr)
        self.assertEqual([], self.calls())
        self.assertFalse((self.root / ".agent").exists())

    def test_lifecycle_uses_default_diagnostics_and_preserves_fixture(self):
        self.environment.pop("BOOK_REVIEW_DIAGNOSTICS")
        diagnostics = self.root / ".agent" / "tmp" / "book-review-diagnostics"
        self.check_lifecycle(diagnostics)

    def test_lifecycle_uses_explicit_diagnostics_and_preserves_fixture(self):
        self.check_lifecycle(self.diagnostics)

    def check_lifecycle(self, expected_diagnostics):
        self.environment["COMPOSE_REMOVE_ORPHANS"] = "true"
        self.assertIn("Stopped", self.require_success("status", "-Port", str(self.port)))
        ready = threading.Event()
        stop = threading.Event()

        def serve_when_started():
            while not self.container.exists():
                if stop.wait(0.02):
                    return
            with ThreadingHTTPServer(("127.0.0.1", self.port), ReviewHandler) as service:
                service.timeout = 0.1
                ready.set()
                while not stop.is_set() and self.container.exists():
                    service.handle_request()

        thread = threading.Thread(target=serve_when_started, daemon=True)
        thread.start()
        try:
            output = self.require_success("start")
            self.assertTrue(ready.is_set())
            self.assertIn("container=one, books=1", output)
            self.assertIn("trial: pending=True", output)
            self.assertTrue(expected_diagnostics.is_dir())
            mounts = json.loads(self.container.read_text(encoding="utf-8"))["Mounts"]
            diagnostics, = (mount for mount in mounts if mount["Destination"] == "/app/.agent/tmp")
            self.assertTrue(diagnostics["RW"])
            self.assertEqual(expected_diagnostics, Path(diagnostics["Source"]))
            self.assertIn("container=one", self.require_success("status"))
            self.assertIn("container=one", self.require_success("start"))
            self.assertIn("container=two", self.require_success("restart"))
            with urlopen(f"http://127.0.0.1:{self.port}/", timeout=3) as response:
                self.assertEqual(b"Book Review fixture", response.read())
            self.assertIn("Stopped", self.require_success("stop"))
            thread.join(5)
            self.assertFalse(thread.is_alive())

            self.assertIn("Stopped", self.require_success("stop"))
            compose = [call for call in self.calls() if call[0] == "compose"]
            self.assertEqual(3, len(compose))
            repo_key = hashlib.sha256(str(self.root).upper().encode()).hexdigest()[:16]
            for call in compose:
                self.assertEqual(f"book-review-{repo_key}-{self.port}", call[call.index("--project-name") + 1])
            self.assertIn("up", compose[0])
            self.assertNotIn("--force-recreate", compose[0])
            self.assertIn("--force-recreate", compose[1])
            self.assertIn("rm", compose[2])
            self.assertEqual(b"fixture source", self.source.read_bytes())
            self.assertEqual("{}", self.recipe.read_text(encoding="utf-8"))
            temporary = self.root / ".agent" / "tmp" / "book-review" / str(self.port)
            self.assertEqual(1, len(list(temporary.glob("*/override.json"))))
            self.assertNotIn(AUTH_CONTENTS, next(temporary.glob("*/controller.log")).read_text(encoding="utf-8-sig"))
        finally:
            stop.set()
            thread.join(5)

    def test_protected_checkout_paths_are_refused_before_mounting(self):
        for diagnostic_path in (".", ".agent/tmp", ".agent/tmp/book-review/old",
                                ".agent/tmp/workflow-TA", ".agent/tmp/integration",
                                ".agent/task-archives"):
            with self.subTest(diagnostic_path=diagnostic_path):
                self.environment["BOOK_REVIEW_DIAGNOSTICS"] = diagnostic_path
                result = self.invoke("start")
                self.assertNotEqual(0, result.returncode)
                self.assertIn("BOOK_REVIEW_DIAGNOSTICS must", result.stderr)
        self.assertEqual([], self.calls())
        self.assertFalse((self.root / ".agent").exists())

    def test_unmanaged_port_and_unavailable_docker_are_failures(self):
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", self.port))
            listener.listen()
            for action in ("start", "status", "restart", "stop"):
                result = self.invoke(action)
                self.assertNotEqual(0, result.returncode)
                self.assertIn("occupied by an unmanaged listener", result.stderr)
            self.assertEqual(self.port, listener.getsockname()[1])

        (self.state / "unavailable").touch()
        result = self.invoke("status")
        self.assertNotEqual(0, result.returncode)
        self.assertIn("Docker project inspection failed (exit 7)", result.stderr)
        log = next((self.root / ".agent" / "tmp" / "book-review").glob("*/*/controller.log"))
        self.assertIn("Cannot connect to the fixture Docker endpoint", log.read_text(encoding="utf-8-sig"))
        self.assertFalse(any(call[0] == "compose" for call in self.calls()))

    def test_active_old_process_is_preserved(self):
        old_port = self.root / ".agent" / "tmp" / "book-review" / str(self.port + 1)
        old_port.mkdir(parents=True)
        with subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"]) as child:
            try:
                started = subprocess.check_output(
                    [POWERSHELL, "-NoProfile", "-Command", f"(Get-Process -Id {child.pid}).StartTime.ToUniversalTime().Ticks"],
                    text=True,
                ).strip()
                (old_port / "process.json").write_text(json.dumps({"pid": child.pid, "started": started}), encoding="utf-8")
                result = self.invoke("start")
                self.assertNotEqual(0, result.returncode)
                self.assertIn("recorded native Book Review process is still active", result.stderr)
                self.assertIsNone(child.poll())
                self.assertEqual([], self.calls())
            finally:
                child.terminate()

    def seed_shared_project(self):
        self.environment.pop("BOOK_REVIEW_PORT")
        self.port = 8765
        project = "amiga"
        override = self.root / ".agent" / "tmp" / "book-review" / str(self.port) / project / "override.json"
        override.parent.mkdir(parents=True)
        override.write_text("{}", encoding="utf-8")
        siblings = self.state / "siblings.json"
        siblings.write_text(json.dumps([
            {"Id": service, "Config": {"Labels": {
                "com.docker.compose.project": project,
                "com.docker.compose.service": service,
                "com.docker.compose.project.config_files": compose_file,
                "com.docker.compose.oneoff": "False",
            }}}
            for service, compose_file in (("collector", "capture/compose.yaml"), ("qdrant", "rag/compose.yaml"))
        ]), encoding="utf-8")
        container = {
            "Id": "book",
            "Config": {"Labels": {
                "com.docker.compose.project": project,
                "com.docker.compose.service": "book-review",
                "com.docker.compose.oneoff": "False",
                "com.docker.compose.project.config_files": f"{self.tool / 'compose.yaml'},{override}",
            }},
            "HostConfig": {"PortBindings": {"8765/tcp": [{"HostIp": "127.0.0.1", "HostPort": str(self.port)}]}},
            "Mounts": [
                {"Type": "bind", "Source": str(source), "Destination": target, "RW": writable}
                for source, target, writable in (
                    (self.library, "/library", False),
                    (self.tool / "config.yaml", "/app/tools/bootstrap/book-to-markdown/config.yaml", False),
                    (self.diagnostics, "/app/.agent/tmp", True),
                    (self.auth, "/run/secrets/book-review-auth", False),
                )
            ],
            "State": {"Running": True},
        }
        self.container.write_text(json.dumps(container), encoding="utf-8")
        return override, siblings, container

    def test_default_port_stops_only_book_review_in_shared_project(self):
        override, siblings, _ = self.seed_shared_project()
        before = siblings.read_bytes()
        self.assertIn("Stopped", self.require_success("stop"))
        self.assertFalse(self.container.exists())
        self.assertEqual(before, siblings.read_bytes())
        self.assertEqual([
            ["ps", "--all", "--quiet", "--filter", "label=com.docker.compose.project=amiga",
             "--filter", "label=com.docker.compose.service=book-review"],
            ["inspect", "--type", "container", "book"],
            ["compose", "--project-name", "amiga", "--project-directory", str(self.tool),
             "--file", str(self.tool / "compose.yaml"), "--file", str(override),
             "rm", "--stop", "--force", "book-review"],
        ], self.calls())

    def test_wrong_project_definition_and_mount_are_refused(self):
        override, siblings, container = self.seed_shared_project()
        before = siblings.read_bytes()
        container["Config"]["Env"] = [f"SECRET={AUTH_CONTENTS}"]
        container["Config"]["Labels"]["com.docker.compose.project.config_files"] = "different-compose.yaml"
        self.container.write_text(json.dumps(container), encoding="utf-8")
        wrong_project = self.invoke("stop", "-Port", "8765")
        self.assertNotEqual(0, wrong_project.returncode)
        self.assertIn("does not belong to this Book Review Compose definition", wrong_project.stderr)

        container["Config"]["Labels"]["com.docker.compose.project.config_files"] = f"{self.tool / 'compose.yaml'},{override}"
        container["Mounts"][0]["Source"] = str(self.diagnostics)
        self.container.write_text(json.dumps(container), encoding="utf-8")
        wrong_mount = self.invoke("restart")
        self.assertNotEqual(0, wrong_mount.returncode)
        self.assertIn("uses different Book Review mounts", wrong_mount.stderr)
        self.assertEqual(container, json.loads(self.container.read_text(encoding="utf-8")))
        self.assertEqual(before, siblings.read_bytes())
        self.assertNotIn(AUTH_CONTENTS, (override.parent / "controller.log").read_text(encoding="utf-8-sig"))
        self.assertFalse(any(call[0] == "compose" for call in self.calls()))


if __name__ == "__main__":
    if sys.argv[1:2] == ["--fake-docker"]:
        sys.exit(fake_docker(sys.argv[2:]))
    unittest.main()
