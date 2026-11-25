"""Prepare startup, upgrade and Receiver inputs for the capture archive."""

from uuid import UUID

from capture_fixtures import SERVICE, marker
from compose_service import COLLECTOR_IMAGE
from test_startup import AT, CHILD_THREAD, CONTINUATION, OTHER_THREAD, ROOT_THREAD, START, log_line


PROOF = ".agent/task-archives/fixture/execution"
ARCHIVES = "sessions/.agent/task-archives"
LEGACY = f"{ARCHIVES}/_receiver"
SETTINGS = f"{SERVICE}/compose-settings.json"
AUTHORITY = f"{SERVICE}/authority/compose-authority.json"
IGNORE = ".agent/task-archives/\n.agent/tools/\n"


def proof(case, tree):
    tree.directory(PROOF + "/otel")
    tree.directory(f"{SERVICE.removeprefix('sessions/')}/receiver")
    if case in ("archive_truncation", "native_only", "stale_or_ineligible"):
        tree.file(PROOF + "/otel/logs.jsonl", log_line(ROOT_THREAD, AT + 10))
    if case in ("new_collector", "spool_recovery"):
        tree.directory(f"{SERVICE.removeprefix('sessions/')}/collector/spool")
    if case in ("native_only", "trace_proof"):
        tree.record(PROOF + "/registration.json", {
            "schema_version": 1, "task_id": "fixture", "execution_id": "execution",
            "git_common_dir": tree.path("common", ".git"), "root_thread_id": ROOT_THREAD,
            "start_utc": START, "end_utc": None, "otel": case == "trace_proof",
            "startup": {"deadline_ns": AT + 20_000, "baseline": {"chunks": []}},
        })


def service_project(tree):
    tree.directory(ARCHIVES)
    tree.directory(f"{SERVICE}/receiver")
    for name in ("sessions", "archived_sessions"):
        tree.directory(f"codex/{name}")


def install(tree, contents, *, shared=False, legacy=False, revised=False):
    settings = {
        "project_id": marker("project"), "compose_project": "amiga" if shared else marker("compose"),
        "port": 4318, "codex_home": tree.path("home", "codex"), "collector_image": COLLECTOR_IMAGE,
        "token": "a1" * 32, "client_token": "b2" * 32, "user": marker("user"),
    }
    tree.revision("current", settings, contents)
    saved = "current"
    if shared:
        first_settings = {**settings, "compose_project": marker("compose")}
        tree.revision("first", first_settings, contents)
    if legacy:
        mount = b"      - type: bind\n"
        first, *blocks = contents["compose.yaml"].split(mount)
        manifest = first + b"".join(mount + block for block in blocks
                                    if b"/capture/receiver\n" not in block
                                    and b"/capture/collector-output\n" not in block)
        tree.revision("legacy", settings, {**contents, "compose.yaml": manifest})
        tree.directories.remove(f"{SERVICE}/receiver")
        tree.directory(f"{LEGACY}/collector-output")
        saved = "legacy"
    else:
        tree.directory(f"{SERVICE}/collector/spool")
    if revised:
        source(tree, "selected-source", contents, b"\n# revised\n")
    tree.record(SETTINGS, {**settings, "config_id": marker(saved)}, native_newlines=True)
    tree.record(AUTHORITY, {
        "project_id": marker("project"), "git_common_dir": tree.path("common", "sessions/.git"),
        "archive_ignored": True, "service_ignored": True, "windows": marker("windows"),
        "worktree_paths": [marker("worktree")], "rollout_source": marker("rollout"),
    }, native_newlines=True)
    return settings


def source(tree, directory, contents, suffix):
    for name, data in contents.items():
        tree.file(f"sessions/{directory}/{name}", data + (suffix if name == "receiver.py" else b""))


def registration(tree, task, execution, **fields):
    tree.record(f"{ARCHIVES}/{task}/{execution}/registration.json", {
        "schema_version": 1, "task_id": task, "execution_id": execution,
        "git_common_dir": tree.path("common", "sessions/.git"), **fields,
    }, native_newlines=True)


def rollout(tree, thread, *, foreign=False, source=None, events=()):
    tree.lines(f"codex/sessions/rollout-fixture-{thread}.jsonl", [{
        "type": "session_meta", "timestamp": "2026-11-18T10:00:00.000005Z", "payload": {
            "id": thread, "cwd": tree.path("foreign-root", ".") if foreign else tree.path("primary", "sessions"),
            "timestamp": "2026-11-18T10:00:00.000005Z", "source": source,
        },
    }, *events])


def range_registration(tree, execution, start=START, end=None, **fields):
    registration(tree, "fixture", execution, root_thread_id=ROOT_THREAD, otel=True, start_utc=start, end_utc=end,
                 threads=[{"id": ROOT_THREAD, "start_utc": start, "end_utc": end}], **fields)


PROOF_CASES = (
    "archive_truncation", "native_only", "new_collector", "spool_recovery", "stale_or_ineligible", "trace_proof",
)
UNPROVISIONED = (
    "legacy_configuration_or_credential", "missing_ignore_rule", "native_receiver_state",
    "new_installation_derives", "session_bind_inside", "settings_code_and_authority",
)
LEGACY_CASES = (
    "existing_previous_layout", "expired_cutover", "failed_layout_switch", "interrupted_spool_copy",
    "layout_migration_collision", "retention_failure", "rollback_after_retention", "upgrade_moves_legacy",
)
REVISED_CASES = ("partial_upgrade_recovers", "upgrade_preserves_the_host", "upgrade_rejects_open")
SERVICE_CASES = (*UNPROVISIONED, *LEGACY_CASES, *REVISED_CASES, "invalid_saved_revision",
                 "mounted_receiver_accepts", "reloads_compose_name", "saved_compose_name",
                 "shared_project_disables", "short_name_requires", "spool_publication_preserves")

CONTROL_ROLLOUTS = {
    "admission_watermark": (ROOT_THREAD, CONTINUATION, OTHER_THREAD),
    "changed_source_closes": (ROOT_THREAD, CONTINUATION),
    "direct_control_cannot": (ROOT_THREAD,),
    "failed_admission_write": (ROOT_THREAD,),
    "failed_registration_write": (ROOT_THREAD,),
    "invalid_credentials_identity": (ROOT_THREAD,),
    "live_index_keeps": (ROOT_THREAD, CONTINUATION),
    "live_start_and_confirmation": (ROOT_THREAD,),
    "provision_reuses": (),
    "receiver_rejects_retired": (OTHER_THREAD, CONTINUATION),
    "receiver_respects_a_saved": (),
    "receiver_restart_restores": (ROOT_THREAD, CONTINUATION),
    "start_rejects_receipts": (ROOT_THREAD,),
    "start_restores_missing": (ROOT_THREAD, OTHER_THREAD),
    "startup_index_routes": (ROOT_THREAD,),
    "status_combines": (ROOT_THREAD,),
    "status_rejects_a_receiver": (),
    "status_reports_stopped": (ROOT_THREAD,),
    "status_reports_unavailable": (),
    "verify_checks_the_saved": (),
}
PUBLISHED_CASES = ("changed_source_closes", "direct_control_cannot", "start_restores_missing", "status_combines")


def service(case, tree, contents):
    service_project(tree)
    if case in UNPROVISIONED:
        if case == "native_receiver_state":
            tree.file(f"{SERVICE}/receiver/tracked.json", b"preserved state\n")
            tree.stage.append(f"{SERVICE}/receiver/tracked.json")
        if case == "missing_ignore_rule":
            tree.directories.remove(f"{SERVICE}/receiver")
            tree.file("sessions/.gitignore", b".agent/task-archives/\n")
        if case == "legacy_configuration_or_credential":
            tree.file(f"{LEGACY}/compose", b"")
        if case == "session_bind_inside":
            for home in (ARCHIVES, ""):
                for name in ("sessions", "archived_sessions"):
                    tree.directory(f"{home}/{name}".lstrip("/"))
        return

    settings = install(tree, contents, shared=case in (*LEGACY_CASES, *REVISED_CASES),
                       legacy=case in LEGACY_CASES, revised=case in REVISED_CASES)
    if case == "invalid_saved_revision":
        tree.record(SETTINGS, {**settings, "config_id": "../other-installation"}, native_newlines=True)
    if case in ("reloads_compose_name", "shared_project_disables", "short_name_requires", "saved_compose_name"):
        altered = {**settings, "compose_project": "amiga" if case == "shared_project_disables" else "amiga-capture"}
        selected = "alternate"
        if case == "short_name_requires":
            altered["project_id"] = marker("foreign")
        if case == "saved_compose_name":
            selected = "current"
        else:
            tree.revision(selected, altered, contents, materialize=False)
        tree.record("settings-input.json", {**altered, "config_id": marker(selected)})
    if case == "spool_publication_preserves":
        tree.file(f"{SERVICE}/receiver/pending.jsonl", b"complete pending output\n")
    if case == "existing_previous_layout":
        tree.file(f"{SERVICE}/previous-layout/{marker('legacy')}/receiver/preserved.txt", b"previous attempt\n")
    if case == "failed_layout_switch":
        tree.file(f"{LEGACY}/lineage-source.json", b"{}\n")
    if case == "interrupted_spool_copy":
        tree.file(f"{LEGACY}/collector-output/logs-pending.jsonl", b"complete pending output\n")
        tree.directory(f"{SERVICE}/receiver")
        tree.directory(f"{SERVICE}/collector/spool")
    if case == "layout_migration_collision":
        tree.file(f"{LEGACY}/collector-output/pending.jsonl", b"original output\n")
        tree.file(f"{SERVICE}/collector/spool/pending.jsonl", b"other output\n")
    if case == "rollback_after_retention":
        tree.file(f"{LEGACY}/historical.txt", b"preserved data\n")
    if case == "retention_failure":
        tree.file(f"{LEGACY}/control.json", b"historical credential\n")
        tree.file(f"{LEGACY}/collector-output/logs-pending.jsonl", b"pending data\n")
    if case == "upgrade_moves_legacy":
        tree.file(f"{LEGACY}/lineage-source.json", b'{"descendant":"parent"}\n')
        tree.file(f"{LEGACY}/collector-output/logs-pending.jsonl", b'{"pending":true}\n')
        tree.file(f"{LEGACY}/control.json", b'{"token":"historical-credential"}\n')
    if case == "upgrade_preserves_the_host":
        tree.file(f"{SERVICE}/collector/spool/fixture.jsonl", b"saved spool")
    if case in ("retention_failure", "upgrade_moves_legacy", "upgrade_preserves_the_host", "upgrade_rejects_open"):
        registration(tree, "task", "execution", end_utc=None if case == "upgrade_rejects_open" else START)


def control(case, tree, contents):
    service_project(tree)
    if case not in ("live_start_and_confirmation", "verify_checks_the_saved"):
        install(tree, contents)
    if case in PUBLISHED_CASES:
        source(tree, "published", contents, b"\n# published revision\n")
    for thread in CONTROL_ROLLOUTS[case]:
        rollout(tree, thread)
    if case == "admission_watermark":
        rollout(tree, CHILD_THREAD, source={"subagent": {"thread_spawn": {"parent_thread_id": ROOT_THREAD}}})
    if case == "invalid_credentials_identity":
        rollout(tree, OTHER_THREAD, foreign=True)
    cutoff = "2026-11-18T10:00:00.000010Z"
    setup, work = str(UUID(int=40)), str(UUID(int=41))
    if case == "startup_index_routes":
        range_registration(tree, "a", end=cutoff)
    if case == "start_rejects_receipts":
        range_registration(tree, "other", start="2026-11-17T10:00:00Z", end="2026-11-17T10:00:01Z")
    if case == "verify_checks_the_saved":
        range_registration(tree, "a", startup={"deadline_ns": AT + 20_000, "baseline": {"chunks": []}})
    if case in ("status_combines", "status_reports_stopped"):
        for name in ("launcher.lock", "registry.lock"):
            tree.file(f"{SERVICE}/receiver/{name}", b"")
    if case == "status_combines":
        range_registration(tree, "closed", start="2026-11-18T10:00:00.000000Z", end=cutoff)
        range_registration(tree, "open", start="2026-11-18T10:00:00.000020Z")
    if case == "status_reports_stopped":
        range_registration(tree, "a", start="2026-11-18T10:00:00.000000Z")
    if case == "receiver_rejects_retired":
        range_registration(tree, "a", pending_after_turn_id=setup)
        rollout(tree, ROOT_THREAD, events=[{
            "type": "event_msg", "timestamp": cutoff, "payload": {"type": "task_complete", "turn_id": setup},
        }])
    if case == "receiver_respects_a_saved":
        range_registration(tree, "a", "2026-11-18T10:00:00.000011Z", "2026-11-18T10:00:00.000030Z",
                           after_turn_id=setup)
        events = []
        for turn, first, last in ((setup, "000001", "000010"), (work, "000020", "000030")):
            for kind, fraction in (("task_started", first), ("task_complete", last)):
                events.append({"type": "event_msg", "timestamp": f"2026-11-18T10:00:00.{fraction}Z",
                               "payload": {"type": kind, "turn_id": turn}})
        rollout(tree, ROOT_THREAD, events=events)
