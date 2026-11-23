"""Wire contract for the local receiver's installation-scoped registration API."""

from contextlib import contextmanager
from http.client import HTTPConnection, HTTPException
import hmac
import json
import re
from urllib.request import ProxyHandler, Request, build_opener

from registry import CaptureError, segment, thread_id, timestamp


ENDPOINT = "/control/v1/registration"
MAX_BODY = 8192
IDENTITY = ("project_id", "compose_project", "config_id", "instance_id")
HTTP = build_opener(ProxyHandler({}))


def control_key(collector, exporter):
    if not all(isinstance(token, str) and re.fullmatch(r"[0-9a-f]{64}", token)
               for token in (collector, exporter)) or collector == exporter:
        raise CaptureError("Capture control requires distinct private installation credentials.")
    return hmac.digest(bytes.fromhex(collector),
                       b"amiga.capture.registration.v1\0" + bytes.fromhex(exporter), "sha256")


def authorization(key, body):
    return b"HMAC-SHA256 " + hmac.digest(key, body, "sha256").hex().encode("ascii")


@contextmanager
def mutation_claim(project, *, control=None):
    # Atomic directory creation coordinates Windows hosts and Linux bind mounts.
    # Their native advisory file locks do not share a lock namespace.
    path = (project.control if control is None else control) / "control-mutation"
    project.require_ignored(path)
    try:
        path.mkdir()
    except FileExistsError as error:
        raise ControlBusy("Capture control is busy. Check the selected maintenance operation.") from error
    try:
        yield
    finally:
        path.rmdir()


class ControlBusy(CaptureError):
    pass


def registration_request(data):
    # The installed v1 Receiver requires this exact set of fields.
    fields = {*IDENTITY, "action", "task", "execution", "thread", "at", "turn_id", "after_turn", "otel", "deadline_ns"}
    if not isinstance(data, dict) or set(data) != fields:
        raise CaptureError("Invalid capture control request fields.")
    action = data["action"]
    if action not in ("start", "include", "stop"):
        raise CaptureError("Unknown registration operation.")
    if data["after_turn"] is not None:
        raise CaptureError("Delayed activation is no longer supported.")
    for field in ("task", "execution"):
        segment(data[field])
    thread_id(data["thread"])
    if data["turn_id"] is not None:
        thread_id(data["turn_id"])
    if data["at"] is not None:
        timestamp(data["at"])
    if type(data["deadline_ns"]) is not int or not 0 < data["deadline_ns"] < 2**63:
        raise CaptureError("Invalid capture control deadline.")
    if type(data["otel"]) is not bool or (action != "start" and data["otel"]):
        raise CaptureError("Invalid capture mode.")
    return data


def query(url: str, data: dict | None = None, token: str | None = None, timeout=2):
    headers = {"Content-Type": "application/json"}
    if token is not None:
        headers["Authorization"] = f"Bearer {token}"
    body = json.dumps(data).encode("utf-8") if data is not None else None
    with HTTP.open(Request(url, data=body, headers=headers), timeout=timeout) as response:
        return json.load(response)


def status(number, timeout=2):
    return query(f"http://127.0.0.1:{number}/status", timeout=timeout)


def send_registration(port, key, request, deadline):
    stage = "Execution registration"
    body = json.dumps(request, separators=(",", ":")).encode("utf-8")
    connection = HTTPConnection("127.0.0.1", port, timeout=deadline.remaining(stage))
    try:
        connection.request("POST", ENDPOINT, body, {
            "Content-Type": "application/json", "Authorization": authorization(key, body).decode("ascii"),
        })
        response = connection.getresponse()
        raw = response.read(MAX_BODY + 1)
        result = json.loads(raw) if len(raw) <= MAX_BODY else None
        if not isinstance(result, dict):
            raise ValueError("Invalid capture receipt.")
    except (HTTPException, OSError, ValueError) as error:
        raise CaptureError(f"{stage}: no verified control reply. Inspect the selected registration before retrying.") from error
    finally:
        connection.close()
    if response.status != 200:
        raise CaptureError(f"{stage} (HTTP {response.status}): {result.get('error', 'Capture control refused the request.')}")
    if (result.get("action"), result.get("task_id"), result.get("execution_id")) != (
        request["action"], request["task"], request["execution"]
    ):
        raise CaptureError(f"{stage}: the control receipt identifies a different registration.")
    return result
