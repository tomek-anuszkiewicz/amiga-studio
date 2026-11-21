"""Exercise archive startup proof, Compose identity and the receiver admission boundary."""

from collections import Counter
from email.message import Message
import hashlib
from http.client import HTTPConnection
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
from threading import Thread
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from uuid import UUID

from capture_fixtures import restore_capture

CAPTURE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(CAPTURE))
from admission import Admission
from compose_service import (
    CONFIG_LABEL, LABEL, PROJECT_LABEL, ComposeService, Deadline, HostCheckouts, saved_settings,
)
from confirmation import baseline, confirm
from control import ENDPOINT, IDENTITY, authorization, control_key, status
from register import host_request
from receiver import CollectorOutput, Receiver
from registry import CaptureError, Project, epoch_ns, path_identity, read_json, write_json
from upgrade import upgrade


ROOT_THREAD = str(UUID(int=1))
CONTINUATION = str(UUID(int=2))
OTHER_THREAD = str(UUID(int=3))
CHILD_THREAD = str(UUID(int=4))
UNRELATED = str(UUID(int=5))
START = "2026-11-18T10:00:00Z"
AT = epoch_ns(START)
ROUTE = "fixture/execution"


def attribute(key, value):
    return {"key": key, "value": {"stringValue": value}}


def log_record(thread, event, value=None):
    return {"timeUnixNano": str(event),
            "attributes": [attribute("conversation.id", thread)],
            "body": {"stringValue": value or thread}}


def envelope(records, route=ROUTE, diagnostic=False):
    attributes = [attribute("capture.route", route)]
    if diagnostic:
        attributes.append({"key": "capture.diagnostic", "value": {"boolValue": True}})
    return {"resourceLogs": [{"resource": {"attributes": attributes},
                              "scopeLogs": [{"logRecords": records}]}]}


def log_line(thread, event, route=ROUTE, diagnostic=False):
    return json.dumps(envelope([log_record(thread, event)], route, diagnostic)).encode() + b"\n"


def fake_git(cwd, *args):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, check=True)


class ArchiveProofTests(unittest.TestCase):
    def setUp(self):
        directory = TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.primary = Path(directory.name).resolve()
        restore_capture("proof", self.primary)
        self.project = Project(self.primary, self.primary)
        self.project.prepare()
        self.registration = self.project.registration_path("fixture", "execution")
        self.logs = self.registration.parent / "otel" / "logs.jsonl"
        self.logs.parent.mkdir(parents=True)

    def register(self, **changes):
        value = {
            "schema_version": 1, "task_id": "fixture", "execution_id": "execution",
            "git_common_dir": str(self.project.common), "root_thread_id": ROOT_THREAD,
            "start_utc": START, "end_utc": None, "otel": True,
            "startup": {"deadline_ns": AT + 20_000, "baseline": baseline(self.project, self.registration)},
        }
        self.registration.write_text(json.dumps({**value, **changes}), encoding="utf-8")

    def read_proof(self, now=AT + 1_000):
        with patch("time.time_ns", return_value=now):
            return confirm(self.project, self.registration, ROOT_THREAD, 0, Deadline)

    def test_stale_or_ineligible_appends_do_not_confirm_and_a_completed_append_does(self):
        # A preexisting file can even carry a later-looking timestamp.
        prefix = log_line(ROOT_THREAD, AT + 10)
        self.logs.write_bytes(prefix)
        self.register()
        self.assertEqual(self.read_proof()["status"], "pending")
        invalid = (
            log_line(ROOT_THREAD, AT - 1) + log_line(ROOT_THREAD, AT)
            + log_line(OTHER_THREAD, AT + 11) + log_line(CHILD_THREAD, AT + 12)
            + log_line(ROOT_THREAD, AT + 13, "fixture/other-execution")
            + log_line(ROOT_THREAD, AT + 14, diagnostic=True)
        )
        valid = log_line(ROOT_THREAD, AT + 15)
        self.logs.write_bytes(prefix + invalid + valid[:-1])
        self.assertEqual(self.read_proof()["status"], "pending")
        self.logs.write_bytes(prefix + invalid + valid)
        proof = self.read_proof()
        self.assertEqual(proof["status"], "confirmed")
        self.assertEqual(proof["deadline_ns"], AT + 20_000)
        self.assertEqual(proof["record"]["event_ns"], AT + 15)
        self.assertEqual(proof["record"]["byte_start"], len(prefix + invalid))
        self.assertEqual(proof["record"]["byte_end"], len(prefix + invalid + valid))

    def collector_output(self):
        output = CollectorOutput(self.project, b"fixture-private-key")
        output.directory.mkdir()
        return output

    def test_new_collector_chunks_obey_the_archive_baseline_and_retain_exact_bytes(self):
        output = self.collector_output()
        for event in (AT + 10, AT + 11):
            message = envelope([log_record(ROOT_THREAD, event)])
            [name] = output.assign("/v1/logs", message)
            raw = json.dumps(message).encode() + b"\n"
            source = output.directory / name
            source.write_bytes(raw[:-1])
            self.assertFalse(output.promote(source))
            source.write_bytes(raw)
            if event == AT + 10:
                self.registration.touch()
                self.assertTrue(output.promote(source))
                self.register()
                self.assertEqual(self.read_proof()["status"], "pending")
            else:
                self.assertTrue(output.promote(source))

        self.assertFalse(source.exists())
        final = self.logs.parent / name
        self.assertEqual(final.read_bytes(), raw)
        proof = self.read_proof()["record"]
        self.assertTrue(proof["path"].endswith("/otel/" + name))
        self.assertEqual((proof["event_ns"], proof["byte_start"], proof["byte_end"]),
                         (AT + 11, 0, len(raw)))

    def test_spool_recovery_is_idempotent_and_rejects_changed_admission(self):
        output = self.collector_output()
        self.register()
        message = envelope([log_record(ROOT_THREAD, AT + 10)], diagnostic=True)
        [name] = output.assign("/v1/logs", message)
        raw = json.dumps(message).encode() + b"\n"
        source = output.directory / name
        resource = message["resourceLogs"][0]["resource"]
        resource["attributes"].pop(1)
        source.write_bytes(json.dumps(message).encode() + b"\n")
        output.drain()
        final = self.logs.parent / name
        self.assertFalse(final.exists())
        source.write_bytes(raw)
        with patch("pathlib.Path.replace", side_effect=OSError("publication failed")):
            with self.assertRaisesRegex(OSError, "publication failed"):
                output.drain()
        self.assertEqual(source.read_bytes(), raw)
        self.assertFalse(final.exists())
        output.drain()
        self.assertEqual(final.read_bytes(), raw)
        source.write_bytes(raw)
        with patch("pathlib.Path.replace", side_effect=AssertionError("already published")):
            output.drain()
        self.assertFalse(source.exists())
        self.assertEqual(self.read_proof()["status"], "pending")

    def test_native_only_and_wrong_execution_archives_cannot_confirm(self):
        self.register(otel=False)
        self.logs.write_bytes(log_line(ROOT_THREAD, AT + 10))
        with self.assertRaisesRegex(CaptureError, "open OTLP registration"):
            self.read_proof()
        self.register()
        wrong_path = self.registration.parent.parent / "wrong" / "otel" / "logs.jsonl"
        wrong_path.parent.mkdir(parents=True)
        wrong_path.write_bytes(log_line(ROOT_THREAD, AT + 10))
        self.assertEqual(self.read_proof()["status"], "pending")

    def test_archive_truncation_and_git_ignore_failure_block_confirmation(self):
        self.logs.write_bytes(log_line(ROOT_THREAD, AT + 10))
        self.register()
        self.logs.write_bytes(b"")
        with self.assertRaisesRegex(CaptureError, "file was truncated"):
            self.read_proof()
        self.primary.joinpath(".gitignore").write_text("", encoding="utf-8")
        with self.assertRaisesRegex(CaptureError, "Git must ignore"):
            self.read_proof()

    def test_trace_proof_carries_its_event_start_and_indexes(self):
        self.register()
        trace = {
            "resourceSpans": [{
                "resource": {"attributes": [attribute("capture.route", ROUTE)]},
                "scopeSpans": [{"spans": [
                    {"attributes": [attribute("conversation.id", ROOT_THREAD)],
                     "startTimeUnixNano": str(AT + 69), "endTimeUnixNano": str(AT + 70)}
                ]}],
            }],
        }
        self.logs.with_name("traces.jsonl").write_bytes(json.dumps(trace).encode() + b"\n")
        proof = self.read_proof()
        self.assertEqual(proof["record"]["signal"], "resourceSpans")
        self.assertEqual(proof["record"]["event_ns"], AT + 69)
        self.assertEqual([proof["record"][key] for key in ("resource_index", "scope_index", "record_index")],
                         [0, 0, 0])


class ServiceFixture:
    def setUp(self):
        directory = TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name).resolve()
        self.primary = root / "sessions"
        restore_capture("service", self.primary)
        self.ignore = self.primary / ".gitignore"
        self.project = Project(self.primary, self.primary)
        self.project.prepare()
        self.args = SimpleNamespace(port=4318, codex_home=root / "codex", collector_image=None)
        for name in ("sessions", "archived_sessions"):
            (self.args.codex_home / name).mkdir(parents=True)

    def provision(self):
        service = ComposeService(self.project, self.args, Deadline.start(10))
        service.persist(self.primary)
        return service

    def save_revision(self, service, **changes):
        settings = {**service.settings, **changes}
        identity = hashlib.sha256(json.dumps(settings, sort_keys=True).encode())
        for name, data in service.contents.items():
            identity.update(name.encode() + b"\0" + hashlib.sha256(data).digest())
        write_json(service.root / "compose-settings.json", {**settings, "config_id": identity.hexdigest()})
        return identity.hexdigest()

class ServicePathsTests(ServiceFixture, unittest.TestCase):
    def test_new_installation_derives_a_name_and_reloads_it(self):
        service = self.provision()
        expected = f"amiga-capture-{self.project.identity[:24]}"
        self.assertEqual(saved_settings(self.project)["compose_project"], expected)
        loaded = ComposeService(self.project, self.args, Deadline.start(10))
        self.assertFalse(loaded.new)
        self.assertEqual(loaded.name, expected)
        self.assertEqual(loaded.config_id, service.config_id)

    def test_reloads_compose_name_from_the_saved_revision(self):
        service = self.provision()
        config_id = self.save_revision(service, compose_project="amiga-capture")
        loaded = self.provision()
        self.assertFalse(loaded.new)
        self.assertEqual(loaded.name, "amiga-capture")
        self.assertEqual(loaded.env["CAPTURE_COMPOSE_PROJECT"], "amiga-capture")
        self.assertEqual(loaded.config_id, config_id)
        self.assertEqual(loaded.env["CAPTURE_PROJECT_ID"], self.project.identity)
        self.assertEqual(saved_settings(self.project)["compose_project"], "amiga-capture")
        with patch.object(loaded, "call") as docker_call:
            loaded.compose("fixture", "config", "--quiet")
        docker_call.assert_called_once_with(
            "fixture", "compose", "--project-name", "amiga-capture", "--file",
            str(loaded.code / "compose.yaml"), "config", "--quiet",
        )

    def test_shared_project_disables_orphan_removal_even_if_enabled_by_the_caller(self):
        service = self.provision()
        self.save_revision(service, compose_project="amiga")
        with patch.dict(os.environ, {"COMPOSE_REMOVE_ORPHANS": "true"}):
            loaded = self.provision()
        self.assertEqual(loaded.env["COMPOSE_REMOVE_ORPHANS"], "false")

    def test_short_name_requires_full_git_identity_and_matching_options(self):
        service = self.provision()
        identity = self.project.identity
        foreign_identity = identity[:-1] + ("0" if identity[-1] != "0" else "1")
        self.save_revision(service, compose_project="amiga-capture", project_id=foreign_identity)
        with self.assertRaisesRegex(CaptureError, "settings differ from the saved project"):
            ComposeService(self.project, self.args, Deadline.start(10))

        self.save_revision(service, compose_project="amiga-capture")
        self.args.port += 1
        with self.assertRaisesRegex(CaptureError, "settings differ from the saved project"):
            ComposeService(self.project, self.args, Deadline.start(10))

    def test_saved_compose_name_is_part_of_the_configuration_digest(self):
        service = self.provision()
        settings = saved_settings(self.project)
        settings["compose_project"] = "amiga-capture"
        write_json(service.root / "compose-settings.json", settings)
        with self.assertRaisesRegex(CaptureError, "different capture service revision"):
            ComposeService(self.project, self.args, Deadline.start(10))

    def test_settings_code_and_authority_use_distinct_paths_outside_the_archive(self):
        service = self.provision()
        root = self.primary / ".agent/tools/workflow-capture"
        self.assertEqual(Path(service.env["CAPTURE_CODE"]), root / "compose" / service.config_id)
        self.assertEqual(Path(service.env["CAPTURE_AUTHORITY"]), root / "authority")
        self.assertEqual(set(root.iterdir()), {
            root / "authority", root / "compose", root / "compose-settings.json",
        })
        config = saved_settings(self.project)
        self.assertEqual(config["config_id"], service.config_id)
        authority = root / "authority/compose-authority.json"
        self.assertEqual(json.loads(authority.read_text(encoding="utf-8"))["project_id"], self.project.identity)
        self.assertEqual({p.name for p in self.project.control.iterdir()}, {"collector-output"})

    def test_missing_ignore_rule_or_tracked_content_blocks_private_service_writes(self):
        root = self.primary / ".agent/tools/workflow-capture"
        self.ignore.write_text(".agent/task-archives/\n", encoding="utf-8")
        with self.assertRaisesRegex(CaptureError, "ignore the entire service directory"):
            self.provision()
        self.assertFalse(root.exists())
        self.ignore.write_text(".agent/task-archives/\n.agent/tools/\n", encoding="utf-8")
        root.mkdir(parents=True)
        tracked = root / "tracked.txt"
        tracked.touch()
        fake_git(self.primary, "add", "-f", str(tracked))
        with self.assertRaisesRegex(CaptureError, "service directory must be untracked"):
            self.provision()
        self.assertEqual(list(root.iterdir()), [tracked])

    def test_legacy_configuration_or_credential_backup_blocks_provision(self):
        for name in ("compose", "compose-authority.json", "compose-settings.json",
                     "compose-settings.before-output-isolation-fixture.json"):
            with self.subTest(name=name):
                path = self.project.control / name
                path.touch()
                with self.assertRaisesRegex(CaptureError, "migrate the legacy service files"):
                    self.provision()
                self.assertTrue(path.is_file())
                path.unlink()

    def test_session_bind_inside_the_archive_or_covering_the_primary_is_refused(self):
        for home in (self.project.archive, self.primary.parent):
            with self.subTest(home=home):
                self.args.codex_home = home
                for name in ("sessions", "archived_sessions"):
                    (home / name).mkdir(exist_ok=True)
                with self.assertRaisesRegex(CaptureError, "must be separate from the capture paths"):
                    self.provision()


    def revised_installation(self):
        service = self.provision()
        self.save_revision(service, compose_project="amiga")
        installed = self.provision()
        source = self.primary / "selected-source"
        source.mkdir()
        for name, data in installed.contents.items():
            (source / name).write_bytes(data + (b"\n# revised\n" if name == "receiver.py" else b""))
        args = SimpleNamespace(project=self.primary, rollback=False)
        return installed, source, args

    def test_upgrade_preserves_the_host_settings_and_reuses_the_matching_revision(self):
        installed, source, args = self.revised_installation()
        registration = self.project.registration_path("task", "execution")
        registration.parent.mkdir(parents=True)
        write_json(registration, dict(schema_version=1, task_id="task", execution_id="execution",
                                      git_common_dir=str(self.project.common), end_utc=START))
        spool = self.project.control / "collector-output" / "fixture.jsonl"
        spool.write_bytes(b"saved spool")
        old_files = [*installed.code.iterdir(), spool, registration,
                     installed.authority / "compose-authority.json"]
        old_bytes = {path: path.read_bytes() for path in old_files}
        engine = UpgradeEngine(self, installed)
        with patch("compose_service.SOURCE", source), patch("compose_service.docker", engine.docker), \
                patch("upgrade.status", engine.status):
            with self.assertRaisesRegex(CaptureError, "different capture service revision"):
                self.provision()
            report = upgrade(args, self.primary)
            self.assertEqual(report["status"], "upgraded")
            self.assertEqual(report["previous_config_id"], installed.config_id)
            self.assertNotEqual(report["config_id"], installed.config_id)
            self.assertEqual(saved_settings(self.project), {**installed.settings, "config_id": report["config_id"]})
            self.assertEqual(self.provision().config_id, report["config_id"])
            self.assertEqual(upgrade(args, self.primary)["status"], "already_current")
        self.assertEqual(engine.actions, [("config", report["config_id"]), ("up", report["config_id"])])
        self.assertEqual({path: path.read_bytes() for path in old_files}, old_bytes)

    def test_partial_upgrade_recovers_both_installed_services_with_the_saved_settings(self):
        installed, source, args = self.revised_installation()
        engine = UpgradeEngine(self, installed, fail_after_receiver=True)
        settings = installed.root / "compose-settings.json"
        original = settings.read_bytes()
        with patch("compose_service.SOURCE", source), patch("compose_service.docker", engine.docker), \
                patch("upgrade.status", engine.status):
            with self.assertRaisesRegex(CaptureError, "cutover fixture.*installed revision was restored"):
                upgrade(args, self.primary)
        self.assertEqual([action for action, _ in engine.actions], ["config", "up", "config", "up"])
        self.assertEqual(engine.actions[-2:], [("config", installed.config_id), ("up", installed.config_id)])
        self.assertEqual(set(engine.revisions.values()), {installed.config_id})
        self.assertEqual(settings.read_bytes(), original)

    def test_upgrade_rejects_open_capture_and_a_changed_installed_snapshot(self):
        installed, source, args = self.revised_installation()
        registration = self.project.registration_path("task", "execution")
        registration.parent.mkdir(parents=True)
        record = dict(schema_version=1, task_id="task", execution_id="execution",
                      git_common_dir=str(self.project.common), end_utc=None)
        write_json(registration, record)
        with patch("compose_service.SOURCE", source), patch("compose_service.docker") as docker:
            with self.assertRaisesRegex(CaptureError, "Close all capture registrations"):
                upgrade(args, self.primary)
            write_json(registration, {**record, "end_utc": START})
            (installed.code / "receiver.py").write_bytes(b"changed installed snapshot")
            with self.assertRaisesRegex(CaptureError, "saved service snapshot differs"):
                upgrade(args, self.primary)
        docker.assert_not_called()



class LoopbackReceiver(Receiver):
    def server_bind(self):
        self.server_address = ("127.0.0.1", 0)
        super().server_bind()


class RegistrationControlTests(ServiceFixture, unittest.TestCase):
    def receiver(self):
        self.args.port = 4318
        self.service = self.provision()
        self.key = control_key(self.service.settings["token"], self.service.settings["client_token"])
        self.project.authority = lambda: read_json(self.service.authority / "compose-authority.json")
        self.project.checkouts = lambda: HostCheckouts(
            self.project.common, os.name == "nt", frozenset(self.project.authority()["worktree_paths"]),
        )
        self.enterContext(patch.dict(os.environ, self.service.env, clear=False))
        for module in ("receiver", "register"):
            self.enterContext(patch(f"{module}.MOUNTED_HOME", self.service.codex_home))
        self.enterContext(patch("receiver.query", self.collector))
        receiver = self.enterContext(LoopbackReceiver(self.project))
        self.port = receiver.server_port
        worker = Thread(target=receiver.serve_forever, kwargs={"poll_interval": 0.02})
        worker.start()
        self.addCleanup(worker.join)
        self.addCleanup(receiver.shutdown)
        self.enterContext(patch("register.status", lambda _port, timeout: status(self.port, timeout=timeout)))
        self.enterContext(patch("register.send_registration", self.host_transport))
        self.enterContext(patch.object(ComposeService, "wait_engine"))
        self.enterContext(patch.object(ComposeService, "containers", return_value={
            name: {"Id": name, "State": {"Running": True}} for name in ("receiver", "collector")
        }))
        self.enterContext(patch.object(ComposeService, "check_logs"))
        self.exec = self.enterContext(patch.object(ComposeService, "legacy_stop"))
        self.receiver_status = receiver.status

    def host_transport(self, _port, key, request, deadline):
        from control import send_registration

        return send_registration(self.port, key, request, deadline)

    def collector(self, endpoint, payload, token, timeout):
        self.assertEqual(token, self.service.settings["token"])
        self.assertLessEqual(timeout, 1)
        for group in payload.get("resourceLogs", []):
            ticket = next(item["value"]["stringValue"] for item in group["resource"]["attributes"]
                          if item["key"] == "capture.output")
            spool = self.project.control / "collector-output" / f"logs-{ticket}.jsonl"
            spool.write_bytes(json.dumps({"resourceLogs": [group]}).encode() + b"\n")
        return {}

    def rollout(self, thread, *, cwd=None, source=None, events=()):
        path = self.args.codex_home / "sessions" / f"rollout-fixture-{thread}.jsonl"
        records = [{"type": "session_meta", "payload": {
            "id": thread, "cwd": str(cwd or self.primary), "timestamp": "2026-11-18T10:00:00.000005Z",
            "source": source,
        }}, *events]
        path.write_bytes(b"".join(json.dumps(record).encode() + b"\n" for record in records))

    def cli(self, action, execution="a", thread=ROOT_THREAD, **kwargs):
        args = SimpleNamespace(
            action=action, execution=execution, task="fixture", project=self.primary, port=None, codex_home=None,
            collector_image=None, otel=action == "start", at=None, after_turn=None, turn_id=None,
        )
        vars(args).update(kwargs)
        with patch("register.Path.cwd", return_value=self.primary), patch.dict(os.environ):
            if thread is None:
                os.environ.pop("CODEX_THREAD_ID", None)
            else:
                os.environ["CODEX_THREAD_ID"] = thread
            return host_request(args)

    def registration(self, execution="a"):
        return self.project.registration_path("fixture", execution)

    def post(self, endpoint, data, headers):
        connection = HTTPConnection("127.0.0.1", self.port, timeout=3)
        try:
            connection.request("POST", endpoint, data, {"Content-Type": "application/json", **headers})
            result = connection.getresponse()
            return result.status, json.load(result)
        finally:
            connection.close()

    def direct(self, action, *, signer=None, exporter=False, **changes):
        current = self.receiver_status()
        packet = {key: current[key] for key in IDENTITY}
        packet.update(action=action, thread=ROOT_THREAD, task="fixture", execution="a", turn_id=None,
                      after_turn=None, at=None, otel=action == "start", deadline_ns=Deadline.start(10).expires_ns)
        packet.update(changes)
        raw = json.dumps(packet).encode()
        headers = {}
        if signer is not None:
            headers["Authorization"] = authorization(signer, raw).decode("ascii")
        if exporter:
            headers["X-Amiga-Codex-Capture"] = self.service.settings["client_token"]
            headers["Authorization"] = f"Bearer {self.service.settings['client_token']}"
        return self.post(ENDPOINT, raw, headers)

    def otlp(self, records):
        code, reply = self.post("/v1/logs", json.dumps(envelope(records)).encode(), {
            "X-Amiga-Codex-Capture": self.service.settings["client_token"],
        })
        self.assertEqual((code, reply), (200, {}))

    def archive_labels(self, execution):
        files = (self.registration(execution).parent / "otel").glob("logs-*.jsonl")
        return {record["body"]["stringValue"] for file in files
                for group in read_json(file)["resourceLogs"] for scope in group["scopeLogs"]
                for record in scope["logRecords"]}

    def modified_source(self):
        source = self.primary / "published"
        source.mkdir()
        for name, content in self.service.contents.items():
            if name == "receiver.py":
                content += b"\n# published revision\n"
            (source / name).write_bytes(content)
        return source

    def test_changed_source_closes_only_the_owned_range_and_can_arm_and_include(self):
        self.receiver()
        self.rollout(ROOT_THREAD)
        self.rollout(CONTINUATION)
        self.cli("start", at=START)
        turn = str(UUID(int=40))
        end = "2026-11-18T10:00:00.000050Z"
        with patch("compose_service.SOURCE", self.modified_source()):
            receipt = self.cli("stop", at=end, turn_id=turn)
        record = read_json(self.registration())
        self.assertEqual(receipt["registration"], self.registration().relative_to(self.primary).as_posix())
        self.assertEqual(record["end_utc"], end)
        self.assertEqual(record["end_turn_id"], turn)
        self.assertEqual(record["threads"], [{"id": ROOT_THREAD, "start_utc": "2026-11-18T10:00:00.000000Z",
                                              "end_utc": end, "end_turn_id": turn}])
        self.rollout(ROOT_THREAD, events=[{
            "type": "event_msg", "timestamp": "2026-11-18T10:00:00.000080Z",
            "payload": {"type": "task_complete", "turn_id": turn},
        }])
        self.assertTrue(self.cli("start", execution="b", after_turn=turn)["pending"])
        armed = self.cli("arm", execution="b", thread=None)
        self.assertEqual(armed["boundary_utc"], "2026-11-18T10:00:00.000081Z")
        self.assertFalse(armed["pending"])
        self.cli("include", execution="b", thread=CONTINUATION, at="2026-11-18T10:00:00.000090Z")
        self.assertEqual([item["id"] for item in read_json(self.registration("b"))["threads"]],
                         [ROOT_THREAD, CONTINUATION])
        self.exec.assert_not_called()

    def test_invalid_credentials_identity_rollout_and_boundaries_keep_registry_intact(self):
        self.receiver()
        self.rollout(ROOT_THREAD)
        self.rollout(OTHER_THREAD, cwd=self.primary.parent)
        self.cli("start", at=START)
        original = self.registration().read_bytes()
        foreign_key = control_key("fe" * 32, "de" * 32)
        rejected = [
            (401, dict()),
            (401, dict(exporter=True)),
            (401, dict(signer=foreign_key)),
            (409, dict(signer=self.key, project_id="another repository")),
            (409, dict(signer=self.key, instance_id="another receiver")),
            (400, dict(signer=self.key, thread=UNRELATED)),
            (400, dict(signer=self.key, at="2026-11-18T09:00:00Z")),
        ]
        for code, argument in rejected:
            with self.subTest(argument=argument):
                self.assertEqual(self.direct("stop", **argument)[0], code)
                self.assertEqual(self.registration().read_bytes(), original)
        self.assertEqual(self.direct("include", signer=self.key, thread=OTHER_THREAD, at=START)[0], 400)
        self.assertEqual(self.registration().read_bytes(), original)
        mismatched = {**read_json(self.registration()), "execution_id": "wrong"}
        write_json(self.registration(), mismatched)
        self.assertEqual(self.direct("stop", signer=self.key)[0], 400)
        self.assertEqual(read_json(self.registration()), mismatched)
        self.assertFalse((self.registration().parent / "otel").exists())
        with patch("register.control_key", return_value=foreign_key):
            with self.assertRaisesRegex(CaptureError, "HTTP 401"):
                self.cli("stop")
        with self.assertRaisesRegex(CaptureError, "supports HTTP control"):
            self.cli("stop", legacy_installed=True)
        self.exec.assert_not_called()

    def test_admission_watermark_and_closed_range_keep_archive_routes_independent(self):
        self.receiver()
        for thread in (ROOT_THREAD, CONTINUATION, OTHER_THREAD):
            self.rollout(thread)
        self.rollout(CHILD_THREAD, source={"subagent": {"thread_spawn": {"parent_thread_id": ROOT_THREAD}}})
        self.cli("start", at=START)
        self.cli("start", execution="b", thread=OTHER_THREAD, at=START)
        self.cli("include", thread=CONTINUATION, at=START)
        self.otlp([log_record(thread, AT + 9000, label) for thread, label in [
            (ROOT_THREAD, "root"), (CHILD_THREAD, "child"), (CONTINUATION, "continuation"),
            (OTHER_THREAD, "other"), (UNRELATED, "unrelated"),
        ]])
        with self.assertRaisesRegex(CaptureError, "Telemetry accepted after"):
            self.cli("stop", at="2026-11-18T10:00:00.000008Z")
        self.assertIsNone(read_json(self.registration())["end_utc"])
        self.cli("stop", at="2026-11-18T10:00:00.000010Z")
        self.otlp([log_record(ROOT_THREAD, AT + 7000, "delayed"),
                   log_record(ROOT_THREAD, AT + 11000, "after"),
                   log_record(OTHER_THREAD, AT + 11000, "still-open")])
        self.assertEqual(self.archive_labels("a"), {"root", "child", "continuation", "delayed"})
        self.assertEqual(self.archive_labels("b"), {"other", "still-open"})
        self.assertIsNone(read_json(self.registration("b"))["end_utc"])

    def test_direct_control_cannot_start_after_upgrade_closed_check(self):
        self.receiver()
        self.rollout(ROOT_THREAD)
        self.cli("start", at=START)
        args = SimpleNamespace(project=self.primary, rollback=False)
        with self.assertRaisesRegex(CaptureError, "Close all capture registrations"):
            upgrade(args, self.primary)
        self.cli("stop", at="2026-11-18T10:00:00.000020Z")

        def replace(service, stage):
            code, response = self.direct("start", signer=self.key, execution="b")
            self.assertEqual(code, 409)
            self.assertIn("Capture control is busy", response["error"])
            self.assertFalse(self.registration("b").exists())
            self.assertTrue(status(self.port)["ready"])
            return {"ready": True, "config_id": service.config_id}

        with patch("compose_service.SOURCE", self.modified_source()), \
                patch("upgrade.status", lambda _port, timeout: status(self.port, timeout=timeout)), \
                patch.object(ComposeService, "compose") as compose, patch("upgrade.replace_services", replace):
            report = upgrade(args, self.primary)
        self.assertEqual(report["status"], "upgraded")
        compose.assert_called_once_with("Capture upgrade configuration", "config", "--quiet")

    def test_live_start_and_confirmation_preserve_deadline_and_native_needs_no_docker(self):
        with patch.object(ComposeService, "wait_engine", side_effect=AssertionError("Native capture uses Docker")), \
                patch("register.send_registration", side_effect=AssertionError("Native capture uses HTTP")):
            self.cli("start", execution="native", otel=False, at=START)
            native = self.cli("stop", execution="native", at="2026-11-18T10:00:00.000003Z")
            self.assertEqual(native["boundary_utc"], "2026-11-18T10:00:00.000003Z")
        self.receiver()
        self.rollout(ROOT_THREAD)
        now = AT + 5000
        with patch("time.time_ns", return_value=now), patch("register.datetime") as clock:
            from datetime import UTC, datetime

            clock.now.return_value = datetime(2026, 11, 18, 10, 0, 0, 5, UTC)
            receipt = self.cli("start")
        self.assertEqual(receipt["deadline_ns"], now + 180 * 1_000_000_000)
        self.assertEqual(read_json(self.registration())["startup"]["deadline_ns"], receipt["deadline_ns"])
        with patch("time.time_ns", return_value=now + 1000):
            self.assertEqual(self.cli("confirm", wait_seconds=0)["status"], "pending")
            self.otlp([log_record(ROOT_THREAD, now + 700, "fresh")])
            proved = self.cli("confirm", wait_seconds=0)
        self.assertEqual(proved["status"], "confirmed")
        self.assertEqual(proved["record"]["event_ns"], now + 700)
        with patch("time.time_ns", return_value=receipt["deadline_ns"]):
            with self.assertRaisesRegex(CaptureError, "deadline expired"):
                self.cli("confirm", wait_seconds=0)


class UpgradeEngine:
    def __init__(self, test, installed, *, fail_after_receiver=False):
        self.test = test
        self.installed = installed
        self.revisions = dict.fromkeys(("receiver", "collector"), installed.config_id)
        self.actions = []
        self.fail_after_receiver = fail_after_receiver

    def docker(self, _deadline, stage, args, *, env=None, **_options):
        result = SimpleNamespace(returncode=0, stdout="", stderr="")
        installed = self.installed
        if args[0] == "info":
            result.stdout = "linux"
        elif args[0] == "ps":
            self.test.assertIn(f"label={LABEL}.project=amiga", args)
            result.stdout = "receiver collector book-review"
        elif args[0] == "inspect":
            self.test.assertEqual(set(args[1:]), {"receiver", "collector", "book-review"})
            result.stdout = json.dumps([
                {"Id": name, "Config": {"Labels": {f"{LABEL}.service": name,
                                                   PROJECT_LABEL: installed.project.identity,
                                                   CONFIG_LABEL: config}},
                 "State": {"Running": True}, "HostConfig": {"RestartPolicy": {"Name": "unless-stopped"}}}
                for name, config in self.revisions.items()
            ] + [{"Config": {"Labels": {f"{LABEL}.service": "book-review"}}}])
        elif args[0] == "compose":
            revision = env["CAPTURE_CONFIG_ID"]
            self.test.assertEqual(args[1:5], ("--project-name", "amiga", "--file",
                                             str(installed.code.parent / revision / "compose.yaml")))
            self.test.assertEqual(env["COMPOSE_REMOVE_ORPHANS"], "false")
            self.test.assertEqual({key: value for key, value in env.items()
                                   if key.startswith("CAPTURE_") and key not in ("CAPTURE_CODE", "CAPTURE_CONFIG_ID")},
                                  {key: value for key, value in installed.env.items()
                                   if key.startswith("CAPTURE_") and key not in ("CAPTURE_CODE", "CAPTURE_CONFIG_ID")})
            self.test.assertEqual(saved_settings(installed.project), installed.settings)
            self.actions.append((args[5], revision))
            if args[5] == "config":
                self.test.assertEqual(args[6:], ("--quiet",))
                self.test.assertTrue(Path(args[4]).is_file())
            else:
                self.test.assertEqual(args[5:], ("up", "--detach", "--no-deps", "--force-recreate",
                                                "receiver", "collector"))
                self.revisions["receiver"] = revision
                if self.fail_after_receiver:
                    self.fail_after_receiver = False
                    return SimpleNamespace(returncode=1, stdout="", stderr="cutover fixture")
                self.revisions["collector"] = revision
        else:
            self.test.assertEqual(args[0], "logs", stage)
        return result

    def status(self, port, *, timeout):
        installed = self.installed
        self.test.assertEqual(port, installed.port)
        self.test.assertGreater(timeout, 0)
        self.test.assertEqual(set(self.revisions.values()), {self.revisions["receiver"]})
        return dict(ready=True, project_id=installed.project.identity, compose_project="amiga",
                    config_id=self.revisions["receiver"], rollout_source=path_identity(installed.codex_home),
                    upstream="http://collector:4318")


class ComposeContractTests(unittest.TestCase):
    def test_only_an_unambiguous_client_credential_can_supply_startup_evidence(self):
        receiver = object.__new__(Receiver)
        receiver.client_token = b"fixture-client"
        headers = Message()
        self.assertTrue(receiver.diagnostic(headers))
        headers.add_header("x-amiga-codex-capture", "fixture-client")
        self.assertFalse(receiver.diagnostic(headers))
        headers.add_header("X-Capture-Diagnostic", "1")
        self.assertTrue(receiver.diagnostic(headers))
        del headers["X-Capture-Diagnostic"]
        headers.add_header("x-amiga-codex-capture", "fixture-client")
        self.assertTrue(receiver.diagnostic(headers))

    def test_windows_common_directory_is_preserved_and_only_proven_worktrees_match(self):
        common = r"D:\Fixture\Primary\.git"
        checkouts = HostCheckouts(common, True, frozenset({r"d:\fixture\primary", r"d:\fixture\worktree"}))
        self.assertEqual(checkouts("d:/Fixture/Worktree"), common)
        self.assertEqual(checkouts("D:/FIXTURE/PRIMARY"), common)
        self.assertIsNone(checkouts(r"D:\AnotherClone\worktree"))
        self.assertIsNone(checkouts(r"D:\Fixture\Other\..\worktree"))
        self.assertIsNone(checkouts(r"D:\Fixture\worktree\unverified-repository"))

    def compose_service(self, name="amiga"):
        service = object.__new__(ComposeService)
        service.name, service.config_id = name, "selected-config"
        service.code = Path("saved/capture")
        service.project = SimpleNamespace(identity="selected-primary")
        return service

    def container(self, name, **labels):
        return {
            "Config": {"Labels": {f"{LABEL}.service": name, **labels}},
            "HostConfig": {"RestartPolicy": {"Name": "unless-stopped"}},
            "State": {"Running": True},
        }

    def test_shared_project_selects_only_capture_containers(self):
        service = self.compose_service()
        capture_labels = {PROJECT_LABEL: "selected-primary", CONFIG_LABEL: "selected-config"}
        receiver = self.container("receiver", **capture_labels)
        collector = self.container("collector", **capture_labels)
        peers = [self.container(name) for name in ("book-review", "qdrant", "rag-qdrant")]
        with patch.object(service, "call", side_effect=[
            "book receiver qdrant collector rag",
            json.dumps([peers[0], receiver, peers[1], collector, peers[2]]),
        ]):
            self.assertEqual(service.containers(), {"receiver": receiver, "collector": collector})

    def test_capture_collision_in_the_shared_project_is_rejected(self):
        service = self.compose_service()
        items = [
            self.container("receiver"),
            self.container("receiver", **{PROJECT_LABEL: "foreign-primary", CONFIG_LABEL: "selected-config"}),
            self.container("collector", **{PROJECT_LABEL: "selected-primary", CONFIG_LABEL: "foreign-config"}),
            self.container("unexpected", **{PROJECT_LABEL: "selected-primary"}),
            self.container("book-review", **{CONFIG_LABEL: "selected-config"}),
        ]
        for item in items:
            with self.subTest(labels=item["Config"]["Labels"]):
                with patch.object(service, "call", side_effect=["fixture-id", json.dumps([item])]):
                    with self.assertRaisesRegex(CaptureError, "different capture ownership"):
                        service.containers()

    def test_recovery_starts_only_the_capture_services(self):
        service = self.compose_service()
        receiver, collector = self.container("receiver"), self.container("collector")
        before = {"receiver": receiver}
        after = {**before, "collector": collector}
        with patch.object(service, "wait_engine"), patch.object(service, "persist"), \
                patch.object(service, "check_logs"), patch.object(service, "call") as docker_call, \
                patch.object(service, "containers", side_effect=[before, after]), \
                patch.object(service, "healthy", return_value={"ready": True}) as healthy:
            self.assertEqual(service.ensure(Path.cwd(), None), {"ready": True})
        healthy.assert_called_once_with(after, None)
        docker_call.assert_called_once_with(
            "Compose recovery", "compose", "--project-name", "amiga", "--file",
            str(service.code / "compose.yaml"), "up", "--detach", "--no-recreate",
            "receiver", "collector",
        )

    def test_unavailable_engine_consumes_the_original_deadline(self):
        service = object.__new__(ComposeService)
        service.deadline = Deadline(AT + 1_000)
        stages = []

        def probe(deadline, stage, args, **options):
            stages.append((deadline.expires_ns, stage))
            return SimpleNamespace(returncode=1, stdout="", stderr="Engine offline")

        with patch("compose_service.docker", side_effect=probe), \
                patch("time.time_ns", side_effect=[AT, AT + 500, AT + 1_000]), \
                patch("time.sleep"):
            with self.assertRaisesRegex(CaptureError, "Docker engine unavailable:.*deadline expired"):
                service.wait_engine()
        self.assertTrue(stages)
        self.assertTrue(all(observed == (AT + 1_000, "Docker engine") for observed in stages))


class RoutingTests(unittest.TestCase):
    def test_mixed_concurrent_ranges_keep_child_continuation_and_thread_routes(self):
        selections = [
            (Path("a/registration.json"), {
                "task_id": "fixture", "execution_id": "a", "root_thread_id": ROOT_THREAD, "otel": True,
                "threads": [{"id": thread, "start_utc": START, "end_utc": None}
                            for thread in (ROOT_THREAD, CONTINUATION)],
            }),
            (Path("b/registration.json"), {
                "task_id": "fixture", "execution_id": "b", "root_thread_id": OTHER_THREAD, "otel": True,
                "threads": [{"id": OTHER_THREAD, "start_utc": START, "end_utc": None}],
            }),
        ]
        project = SimpleNamespace(registrations=lambda: iter(selections), archive=Path("fixture-archive"),
                                  require_ignored=lambda _path: None)
        lineage = SimpleNamespace(parent=lambda thread, _: (ROOT_THREAD, AT + 5) if thread == CHILD_THREAD else None,
                                  save=lambda _proofs: None)
        admission = Admission(project, lineage)
        records = [log_record(thread, AT + 9) for thread in
                   (ROOT_THREAD, OTHER_THREAD, UNRELATED, CHILD_THREAD, CONTINUATION)]
        with patch.object(admission, "turn_coverage", return_value={}), patch("admission.write_json"):
            routed = admission.filter("/v1/logs", envelope(records), diagnostic=True)
        actual = {
            group["resource"]["attributes"][0]["value"]["stringValue"]:
            {entry["body"]["stringValue"] for entry in group["scopeLogs"][0]["logRecords"]}
            for group in routed["resourceLogs"]
        }
        self.assertEqual(actual, {"fixture/a": {ROOT_THREAD, CHILD_THREAD, CONTINUATION},
                                  "fixture/b": {OTHER_THREAD}})
        self.assertEqual(admission.stats, Counter(accepted=4, unresolved=1))


if __name__ == "__main__":
    unittest.main()
