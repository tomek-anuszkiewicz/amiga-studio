"""Project-scoped OTLP/HTTP admission service and upstream Collector lifecycle."""

import argparse
from collections import Counter
import gzip
import hashlib
import hmac
from http.server import BaseHTTPRequestHandler, HTTPServer
from io import BytesIO
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import time
from urllib.error import HTTPError
from urllib.request import ProxyHandler, Request, build_opener

from admission import Admission, RolloutLineage, SIGNALS
from registry import CaptureError, Project, read_json, write_json


ADDRESS = "127.0.0.1"
PORT = 4318
UPSTREAM_PORT = 4319
COLLECTOR_IMAGE = (
    "otel/opentelemetry-collector-contrib@"
    "sha256:e40d61f10c9a617cca1ffd1be4d7e2b3a4b0f0001798ff07e58cabe82e618cea"
)
COLLECTOR_CONFIG = Path(__file__).with_name("collector.yaml").resolve()
PROTOCOL = 1
MAX_REQUEST = 16 * 1024 * 1024
HTTP = build_opener(ProxyHandler({}))


def port(value: str) -> int:
    number = int(value)
    if not 1024 <= number <= 65535:
        raise argparse.ArgumentTypeError("Select a port between 1024 and 65535.")
    return number


def query(number: int, path: str, data: dict | None = None, token: str | None = None, timeout=4):
    headers = {"Content-Type": "application/json"}
    if token is not None:
        headers["Authorization"] = f"Bearer {token}"
    body = json.dumps(data).encode("utf-8") if data is not None else None
    req = Request(f"http://{ADDRESS}:{number}{path}", data=body, headers=headers)
    with HTTP.open(req, timeout=timeout) as response:
        return json.load(response)


def handler_fingerprint() -> str:
    digest = hashlib.sha256()
    for name in ("receiver.py", "admission.py", "otlp.py", "record_sources.py", "registry.py"):
        digest.update(hashlib.sha256(Path(__file__).with_name(name).read_bytes()).digest())
    return digest.hexdigest()


def docker(*args: str, env=None, check=True):
    try:
        result = subprocess.run(
            ["docker", *args],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=20,
            env=env,
        )
    except FileNotFoundError as error:
        raise CaptureError("The Docker executable is unavailable to the capture receiver.") from error
    if check and result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise CaptureError(f"Collector Docker command {args[0]} failed. {detail}")
    return result


class Collector:
    def __init__(self, project: Project, number: int, image: str, token: str):
        self.project = project
        self.number = number
        self.image = image
        self.token = token
        self.name = f"amiga-capture-{project.identity[:24]}"
        self.fingerprint = hashlib.sha256(
            COLLECTOR_CONFIG.read_bytes() + f"\n{number}\n{image}\n{token}".encode("utf-8")
        ).hexdigest()

    def start(self):
        existing = docker(
            "container", "inspect", "--format",
            '{{.State.Status}} {{index .Config.Labels "capture.config"}}', self.name,
            check=False,
        )
        if existing.returncode:
            docker("image", "inspect", self.image)
            env = {**os.environ, "CAPTURE_COLLECTOR_TOKEN": self.token}
            docker(
                "run", "--detach", "--rm", "--pull=never", "--name", self.name,
                "--label", f"capture.config={self.fingerprint}",
                "--publish", f"{ADDRESS}:{self.number}:4318",
                "--mount", f"type=bind,source={COLLECTOR_CONFIG},target=/capture/collector.yaml,readonly",
                "--mount", f"type=bind,source={self.project.archive},target=/capture/output",
                "--env", "CAPTURE_COLLECTOR_TOKEN", self.image, "--config=/capture/collector.yaml",
                env=env,
            )
        elif existing.stdout.strip() != f"running {self.fingerprint}":
            raise CaptureError("An existing Collector container has different capture settings.")
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            try:
                self.send("/v1/logs", {})
                self.send("/v1/traces", {})
                return
            except OSError:
                time.sleep(0.2)
        logs = docker("logs", "--tail", "15", self.name, check=False)
        docker("stop", self.name, check=False)
        raise CaptureError(f"Collector did not become ready. {logs.stdout.strip()} {logs.stderr.strip()}")

    def send(self, endpoint, message):
        result = query(self.number, endpoint, message, self.token)
        partial = result.get("partialSuccess") or {}
        if any(int(value) for key, value in partial.items() if key.startswith("rejected")):
            raise OSError("Collector rejected part of an admitted request.")

    def stop(self):
        docker("stop", self.name)


class Receiver(HTTPServer):
    def __init__(self, project, codex_home, number, collector):
        self.handler_config = handler_fingerprint()
        self.admission = Admission(project, RolloutLineage(project, codex_home))
        super().__init__((ADDRESS, number), Handler)
        self.timeout = 0.5
        self.project = project
        self.collector = collector
        self.instance = secrets.token_hex(12)
        self.counts = Counter()
        self.closed = False

    def status(self):
        return {
            "ready": True,
            "protocol": PROTOCOL,
            "project_id": self.project.identity,
            "rollout_source": self.admission.lineage.source_id,
            "handler_config": self.handler_config,
            "instance_id": self.instance,
            "collector": self.collector.name,
            "collector_image": self.collector.image,
            "collector_port": self.collector.number,
            "collector_config": self.collector.fingerprint,
            "counts": dict(self.admission.stats + self.counts),
        }


class Handler(BaseHTTPRequestHandler):
    def setup(self):
        super().setup()
        self.connection.settimeout(5)

    def log_message(self, *_):
        pass

    def reply(self, status: int, result: dict):
        data = json.dumps(result).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/status":
            try:
                self.server.collector.send("/v1/logs", {})
                self.server.collector.send("/v1/traces", {})
            except OSError:
                self.reply(503, {"error": "Collector is unavailable."})
            else:
                self.reply(200, self.server.status())
        else:
            self.reply(404, {"error": "Unknown capture endpoint."})

    def do_POST(self):
        if self.path == "/shutdown":
            self.stop_service()
            return
        if self.path not in (*SIGNALS, "/v1/metrics"):
            self.reply(404, {"error": "Unknown capture endpoint."})
            return
        try:
            message = self.read_message()
            if self.path == "/v1/metrics":
                self.server.counts["metrics_requests_discarded"] += 1
            else:
                with self.server.project.lock():
                    admitted = self.server.admission.filter(self.path, message)
                    if admitted:
                        self.server.collector.send(self.path, admitted)
        except (CaptureError, UnicodeError, TimeoutError, json.JSONDecodeError) as error:
            self.reply(400, {"error": str(error)})
        except (OSError, KeyError, TypeError, ValueError) as error:
            print(f"Capture processing failed. {error}", file=sys.stderr, flush=True)
            self.server.counts["failed_requests"] += 1
            self.reply(503, {"error": "Could not process or export the capture request."})
        else:
            self.reply(200, {})

    def read_message(self):
        if self.headers.get_content_type() != "application/json":
            raise CaptureError("The capture endpoint accepts OTLP/HTTP JSON.")
        length = self.headers.get("Content-Length", "")
        if not length.isdecimal() or len(length) > 8 or not 0 < int(length) <= MAX_REQUEST:
            raise CaptureError("Invalid OTLP request size.")
        data = self.rfile.read(int(length))
        if len(data) != int(length):
            raise CaptureError("Incomplete OTLP request.")
        encoding = self.headers.get("Content-Encoding", "").lower()
        if encoding == "gzip":
            try:
                with gzip.GzipFile(fileobj=BytesIO(data)) as stream:
                    data = stream.read(MAX_REQUEST + 1)
            except (OSError, EOFError) as error:
                raise CaptureError("Invalid compressed OTLP request.") from error
        elif encoding not in ("", "identity"):
            raise CaptureError("Unsupported OTLP request encoding.")
        if len(data) > MAX_REQUEST:
            raise CaptureError("The decoded OTLP request is too large.")
        message = json.loads(data)
        if not isinstance(message, dict):
            raise CaptureError("An OTLP request must be a JSON object.")
        return message

    def stop_service(self):
        expected = f"Bearer {self.server.collector.token}".encode()
        supplied = self.headers.get("Authorization", "").encode("utf-8")
        if not hmac.compare_digest(supplied, expected):
            self.reply(403, {"error": "Invalid capture shutdown authorization."})
            return
        try:
            with self.server.project.lock():
                active = any(
                    data["otel"] and data["end_utc"] is None
                    for _, data in self.server.project.registrations()
                )
                if active:
                    self.reply(409, {"error": "Close all active OTLP executions before shutdown."})
                    return
                self.server.collector.stop()
                self.server.closed = True
        except (CaptureError, OSError, ValueError, subprocess.TimeoutExpired) as error:
            print(f"Collector shutdown failed. {error}", file=sys.stderr, flush=True)
            self.reply(503, {"error": "Collector shutdown failed."})
        else:
            self.reply(200, {"stopped": True, "counts": self.server.status()["counts"]})


def control_token(project: Project, *, create: bool = True) -> str:
    path = project.plain_path(project.control / "control.json")
    if path.exists():
        return read_json(path)["token"]
    if not create:
        raise CaptureError("The active receiver has no project control token.")
    project.require_ignored(path)
    token = secrets.token_hex(32)
    write_json(path, {"token": token})
    return token


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--codex-home", required=True, type=Path)
    parser.add_argument("--port", type=port, required=True)
    parser.add_argument("--upstream-port", type=port, required=True)
    parser.add_argument("--collector-image", required=True)
    args = parser.parse_args()
    try:
        project = Project(args.project, Path.cwd())
        project.prepare()
        with project.lock():
            token = control_token(project)
        collector = Collector(project, args.upstream_port, args.collector_image, token)
        with project.service_lock(), Receiver(project, args.codex_home, args.port, collector) as service:
            collector.start()
            while not service.closed:
                service.handle_request()
    except (CaptureError, OSError, subprocess.TimeoutExpired) as error:
        print(f"Cannot run the capture receiver. {error}", file=sys.stderr, flush=True)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
