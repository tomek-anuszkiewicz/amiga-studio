"""Project-scoped OTLP/HTTP admission service."""

from collections import Counter
import gzip
import hmac
from http.server import BaseHTTPRequestHandler, HTTPServer
from io import BytesIO
import json
import os
import secrets
import stat
import sys
import time

from admission import Admission, RolloutLineage, SIGNALS
from compose_service import CLIENT_HEADER, MOUNTED_HOME, UPSTREAM, MountedProject
from confirmation import CHUNK, MAX_RECORD
from control import ENDPOINT, IDENTITY, MAX_BODY, ControlBusy, authorization, control_key, mutation_claim, query
from register import controlled_registration
from otlp import DIAGNOSTIC_KEY, ROUTE_KEY, attributes, items
from record_sources import safe_file
from registry import CaptureError


PORT = 4318
MAX_REQUEST = 16 * 1024 * 1024
OUTPUT_KEY = "capture.output"
OUTPUT_WAIT_SECONDS = 2



class CollectorOutput:
    def __init__(self, project, key):
        self.project = project
        self.directory = project.plain_path(project.spool)
        self.key = key

    def signature(self, endpoint, route, diagnostic, nonce):
        binding = json.dumps([endpoint, route, diagnostic, nonce]).encode("utf-8")
        return hmac.digest(self.key, binding, "sha256").hex()

    def assign(self, endpoint, message):
        resource_key, _, _ = SIGNALS[endpoint]
        for group in items(message, resource_key):
            resource = group["resource"]
            resource["attributes"] = [item for item in items(resource, "attributes")
                                      if item.get("key") != OUTPUT_KEY]
            data = attributes(resource)
            nonce = secrets.token_hex(16)
            signature = self.signature(endpoint, data[ROUTE_KEY], DIAGNOSTIC_KEY in data, nonce)
            ticket = f"{nonce}-{signature}"
            resource["attributes"].append({"key": OUTPUT_KEY, "value": {"stringValue": ticket}})
            yield f"{endpoint.rsplit('/', 1)[-1]}-{ticket}.jsonl"

    def read(self, source):
        # The Collector owns this directory. Refuse links and nonregular files
        # before reading, including a replacement between lookup and open.
        flags = os.O_RDONLY | getattr(os, "O_BINARY", 0)
        flags |= getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
        try:
            if source.is_symlink():
                return b""
            with os.fdopen(os.open(source, flags), "rb") as stream:
                metadata = os.fstat(stream.fileno())
                if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
                    return b""
                return stream.read(MAX_RECORD + 1)
        except FileNotFoundError:
            return b""

    def destination(self, source, raw):
        match = CHUNK.fullmatch(source.name)
        endpoint = f"/v1/{match[1]}"
        resource_key, _, _ = SIGNALS[endpoint]
        if len(raw) > MAX_RECORD or not raw.endswith(b"\n") or raw.count(b"\n") != 1:
            return None
        try:
            record = json.loads(raw)
            groups = items(record, resource_key)
            if len(groups) != 1 or set(record) != {resource_key}:
                return None
            data = attributes(groups[0].get("resource", {}))
            route = data[ROUTE_KEY]
            expected = self.signature(endpoint, route, DIAGNOSTIC_KEY in data, match[2])
            if data.get(OUTPUT_KEY) != f"{match[2]}-{match[3]}" or not hmac.compare_digest(expected, match[3]):
                return None
            task, execution = route.split("/")
        except (KeyError, TypeError, ValueError):
            return None
        registration = self.project.registration_path(task, execution)
        if not registration.is_file():
            raise OSError("The Collector output no longer has a matching registration.")
        return registration.parent / "otel" / source.name

    def promote(self, source):
        raw = self.read(source)
        target = self.destination(source, raw)
        if target is None:
            return False
        self.project.require_ignored(target)
        if target.exists():
            if safe_file(target, single_link=True).read_bytes() != raw:
                raise OSError("Collector output conflicts with a published archive file.")
        else:
            target.parent.mkdir(exist_ok=True)
            pending = target.with_suffix(".pending")
            self.project.require_ignored(pending)
            pending.unlink(missing_ok=True)
            try:
                with pending.open("xb") as stream:
                    stream.write(raw)
                    stream.flush()
                    os.fsync(stream.fileno())
                # Publish a receiver-owned file. The Collector can still have
                # its original file open when the completed line is readable.
                pending.replace(target)
            finally:
                pending.unlink(missing_ok=True)
        source.unlink(missing_ok=True)
        return True

    def drain(self):
        error = None
        for source in self.directory.iterdir():
            if CHUNK.fullmatch(source.name):
                try:
                    self.promote(source)
                except (OSError, ValueError) as failure:
                    error = failure
        if error is not None:
            raise error

    def wait_for(self, names):
        pending = {self.directory / name for name in names}
        expires = time.monotonic() + OUTPUT_WAIT_SECONDS
        while pending:
            pending = {source for source in pending if not self.promote(source)}
            if pending:
                if time.monotonic() >= expires:
                    raise OSError("Collector did not write a complete admitted record to the archive.")
                time.sleep(0.025)


class Receiver(HTTPServer):
    def __init__(self, project):
        self.project = project
        source = project.authority()["rollout_source"]
        with project.lock():
            self.admission = Admission(project, RolloutLineage(project, MOUNTED_HOME, source_id=source))
        self.token = os.environ["CAPTURE_COLLECTOR_TOKEN"]
        self.client_token = os.environ["CAPTURE_CLIENT_TOKEN"].encode("ascii")
        self.control_key = control_key(self.token, os.environ["CAPTURE_CLIENT_TOKEN"])
        self.output = CollectorOutput(project, self.client_token)
        self.output_error = False
        self.counts = Counter()
        self.instance = secrets.token_hex(12)
        super().__init__(("0.0.0.0", PORT), Handler)

    def forward(self, endpoint, message):
        names = list(self.output.assign(endpoint, message))
        result = query(f"{UPSTREAM}{endpoint}", message, self.token, timeout=1)
        partial = result.get("partialSuccess") or {}
        if any(int(value) for key, value in partial.items() if key.startswith("rejected")):
            raise OSError("Collector rejected part of an admitted request.")
        self.output.wait_for(names)

    def service_actions(self):
        # Retry complete files left by an interrupted request or archive write.
        try:
            self.output.drain()
        except (OSError, ValueError):
            if not self.output_error:
                print("Capture processing failed while publishing Collector output.", file=sys.stderr, flush=True)
            self.output_error = True
        else:
            self.output_error = False

    def diagnostic(self, headers):
        supplied = headers.get_all(CLIENT_HEADER, [])
        return (headers.get("X-Capture-Diagnostic") is not None or len(supplied) != 1
                or not hmac.compare_digest(supplied[0].encode("utf-8"), self.client_token))

    def status(self):
        return {
            "ready": True,
            "control_version": 1,
            "project_id": self.project.identity,
            "compose_project": os.environ["CAPTURE_COMPOSE_PROJECT"],
            "config_id": os.environ["CAPTURE_CONFIG_ID"],
            "rollout_source": self.admission.lineage.source_id,
            "instance_id": self.instance,
            "upstream": UPSTREAM,
            "counts": dict(self.admission.stats + self.counts),
        }


class Handler(BaseHTTPRequestHandler):
    def setup(self):
        super().setup()
        self.connection.settimeout(5)

    def log_message(self, *_):
        pass

    def reply(self, status, result):
        data = json.dumps(result).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path != "/status":
            self.reply(404, {"error": "Unknown capture endpoint."})
            return
        try:
            self.server.project.authority()
            self.server.forward("/v1/logs", {})
            self.server.forward("/v1/traces", {})
            self.server.output.drain()
        except (OSError, KeyError, ValueError):
            self.reply(503, {"error": "Collector or project archive is unavailable."})
        else:
            self.reply(200, self.server.status())

    def control(self):
        try:
            lengths = self.headers.get_all("Content-Length", [])
            if (self.headers.get_content_type() != "application/json"
                    or self.headers.get("Transfer-Encoding") is not None
                    or self.headers.get("Content-Encoding") is not None
                    or len(lengths) != 1 or len(lengths[0]) > 4 or not lengths[0].isdecimal()
                    or not 0 < int(lengths[0]) <= MAX_BODY):
                raise CaptureError("Invalid capture control request format or size.")
            supplied = self.headers.get_all("Authorization", [])
            if len(supplied) != 1:
                self.reply(401, {"error": "Capture control authorization is required."})
                return
            body = self.rfile.read(int(lengths[0]))
            if len(body) != int(lengths[0]):
                raise CaptureError("Incomplete capture control request.")
            if not hmac.compare_digest(supplied[0].encode("utf-8"), authorization(self.server.control_key, body)):
                self.reply(401, {"error": "Capture control authorization failed."})
                return
            request = json.loads(body)
            current = self.server.status()
            if not isinstance(request, dict) or any(request.get(field) != current[field] for field in IDENTITY):
                self.reply(409, {"error": "Capture control installation or receiver instance changed."})
                return
            with mutation_claim(self.server.project):
                report = controlled_registration(self.server.project, request, self.server.admission.registrations)
        except ControlBusy as error:
            self.reply(409, {"error": str(error)})
        except (CaptureError, ValueError, TimeoutError) as error:
            self.reply(400, {"error": str(error)})
        except (KeyError, OSError, TypeError):
            self.reply(503, {"error": "The capture registration or project archive is unavailable."})
        else:
            self.reply(200, report)

    def do_POST(self):
        if self.path == ENDPOINT:
            self.control()
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
                    admitted = self.server.admission.filter(
                        self.path, message, resolve_common=self.server.project.checkouts(),
                        diagnostic=self.server.diagnostic(self.headers),
                    )
                    if admitted:
                        self.server.forward(self.path, admitted)
        except (CaptureError, UnicodeError, TimeoutError, json.JSONDecodeError) as error:
            self.reply(400, {"error": str(error)})
        except (OSError, KeyError, TypeError, ValueError):
            print("Capture processing failed while admitting or exporting records.", file=sys.stderr, flush=True)
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


def main():
    try:
        project = MountedProject()
        project.require_ignored(project.control / "registry.lock")
        with project.service_lock(), Receiver(project) as service:
            service.serve_forever()
    except (OSError, KeyError, ValueError) as error:
        print(f"Cannot run the capture receiver. {error}", file=sys.stderr, flush=True)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
