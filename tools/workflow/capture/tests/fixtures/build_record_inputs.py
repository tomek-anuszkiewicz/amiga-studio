"""Prepare environment, record and whole-file preservation inputs."""

import json

from capture_fixtures import json_bytes, sha256


ENVIRONMENT_CASES = ("capture_round_trip", "compare_reports_required")
PRESERVATION_CASES = (
    "actual_live_writers", "collisions_keep", "copy_never_reads", "failed_copy", "only_selected_files",
    "redirected_execution_leaf", "selected_file_in_another_execution", "wrong_root_unignored",
)
RECORDS_CASES = (
    "current_verifier_reports", "existing_revision_keeps", "primary_checkout_can_supply",
    "removed_collection_command", "timing_receipts", "timing_rejects",
)
REPORT = "test-runtime-report.md"
TASK = "fixture-task"
THREAD = "11111111-1111-4111-8111-111111111111"
TURN = "22222222-2222-4222-8222-222222222222"
REVISION = "20261118T100200000000Z-12345678"
NATIVE = f"fixture-client/sessions/rollout-fixture-{THREAD}.jsonl"
PRESERVATION = "primary/.agent/task-archives/trial/execution"
SELECTION = "task/.agent/tmp/selected.json"


def environment(case, tree):
    if case == "compare_reports_required":
        tree.file("old.json", b'{"format_version": 1, "probes": []}')
        return
    tree.file("tools/verification/requirements.txt", b"pytest==8.0\n")
    tree.file(".codex/config.toml", b"[mcp_servers.fixture]\n")
    tree.file("git.exe", b"original tool")
    tree.lines("rollout.jsonl", [
        {"type": "session_meta", "payload": {"cli_version": "1.2.3", "source": "cli"}},
        {"type": "turn_context", "payload": {"turn_id": THREAD, "model": "fixture-model", "effort": "low"}},
    ], final_newline=False)


def preservation(case, tree):
    if case == "actual_live_writers":
        tree.directory("task/.agent/tmp")
        tree.directory(PRESERVATION)
        return

    files = []

    def select(name, path, data, destination):
        tree.file(path, data)
        files.append({"id": name, "source": tree.path(f"source-{name}", path), "path": destination, "complete": True})

    def resident(path, data):
        tree.file(f"{PRESERVATION}/{path}", data)

    if case == "only_selected_files":
        select("main", "main.jsonl", b'{"opening":1}\r\nunparsed\xff\r\nfinal tail', "sessions/main.jsonl")
        select("worker", f"{PRESERVATION}/sessions/agent.jsonl", b"already archived\r\n\x80", "sessions/agent.jsonl")
        select("log", "task/.agent/tmp/output.txt", b"raw\r\n\xfe", "outputs/execution.txt")
        resident("otel/events.jsonl", b"existing telemetry")
        tree.file("decoy/sessions/other.jsonl", b"must remain unobserved")
    if case == "copy_never_reads":
        resident("registration.json", b"unreadable registration")
        select("session", "session.jsonl", b"finished session", "sessions/main.jsonl")
    if case == "collisions_keep":
        resident("same.log", b"original")
        resident("same-2.log", b"occupied fallback")
        for index, data in enumerate((b"original", b"second source"), 1):
            name = f"log-{index}"
            select(name, name, data, "same.log")
    if case == "failed_copy":
        resident("blocked", b"keep obstruction")
        select("first", "first.json", b"complete first", "first.json")
        select("second", "second.json", b"complete second", "blocked/second.json")
    if case == "selected_file_in_another_execution":
        select("output", "primary/.agent/task-archives/trial/other/output.txt", b"keep other execution", "output.txt")
    if case == "wrong_root_unignored":
        select("source", "source.txt", b"selected", "source.txt")
    if case == "redirected_execution_leaf":
        select("source", "source.txt", b"selected", "sentinel.txt")
        tree.file("outside/sentinel.txt", b"do not change")
        tree.file("primary/.agent/task-archives/trial/other/sentinel.txt", b"other execution")
    tree.record(SELECTION, {"files": files})


def timing(tree, profile, attempt, data, checkout="task"):
    relative = f".agent/tmp/verification/diagnostics/{attempt}/{REPORT}"
    path = f"{checkout}/{relative}"
    tree.file(path, data)
    selection = {"task_id": TASK, "profile": profile, "attempt": attempt, "candidate": tree.base,
                 "outcome": "passed", "path": relative, "sha256": sha256(data)}
    if profile == "diagnostic":
        selection["invocation"] = "verify.py check english files AGENTS.md"
    return path, selection


def records(case, tree):
    if case == "removed_collection_command":
        for path in ("primary", "task"):
            tree.directory(path)
    if case == "timing_receipts":
        selected = []
        for profile, attempt, data in (("commit", "1" * 32, b"first commit report\n"),
                                       ("commit", "2" * 32, b"later commit report\n"),
                                       ("milestone", "3" * 32, b"milestone report\n")):
            _, selection = timing(tree, profile, attempt, data)
            selected.append(selection)
        tree.record("timing-inputs.json", selected)
        tree.file("task/.agent/tasks/plan.md", b"unique input\n")
    if case in ("timing_rejects", "primary_checkout_can_supply"):
        if case == "timing_rejects":
            _, selected = timing(tree, "commit", "7" * 32, b"current timing\n")
        else:
            _, selected = timing(tree, "diagnostic", "4" * 32, b"diagnostic report\n", "primary")
        tree.record("timing-inputs.json", selected)
    if case == "existing_revision_keeps":
        history(tree)


def history(tree):
    start, middle, end = ("2026-11-18T10:00:00Z", "2026-11-18T10:01:00Z", "2026-11-18T10:02:00Z")
    rollout = tree.lines(NATIVE, [
        {"timestamp": start, "type": "session_meta", "payload": {"id": THREAD, "cwd": tree.path("task", "task")}},
        {"timestamp": middle, "type": "event_msg", "payload": {"type": "task_started", "turn_id": TURN}},
        {"timestamp": end, "type": "event_msg", "payload": {"type": "task_complete", "turn_id": TURN}},
    ])
    archive = f"primary/.agent/task-archives/{TASK}/registered/revisions/{REVISION}"
    common = tree.path("common", "primary/.git")
    start_utc, end_utc = (stamp.replace("Z", ".000000Z") for stamp in (start, end))
    scope = {"id": THREAD, "start_utc": start_utc, "end_utc": end_utc, "end_turn_id": TURN}
    registration = {
        "schema_version": 1, "task_id": TASK, "execution_id": "registered", "root_thread_id": THREAD,
        "git_common_dir": common, "start_utc": start_utc, "end_utc": end_utc, "otel": False,
        "threads": [scope], "end_turn_id": TURN,
    }
    inventory = []
    expected = {}

    def archived(path, data, kind, **metadata):
        tree.file(f"{archive}/{path}", data, bind=kind in ("registration", "rollout"),
                  native_newlines=kind == "registration")
        inventory.append({"path": path, "kind": kind, "size": len(data),
                          "sha256": sha256(data), **metadata})

    def selected(source, path):
        data = tree.files[source]
        archived(path, data, "artifact", source=tree.path(f"artifact-{len(expected)}", source))
        expected[path] = {"kind": "archive", "path": path, "sha256": sha256(data), "size": len(data)}

    archived("registration.json", json_bytes(registration), "registration")
    native_path = tree.path("native", NATIVE)
    segment = f"rollouts/{THREAD}.jsonl"
    archived(segment, rollout, "rollout", source=native_path,
             ranges=[{"start": 0, "end": len(rollout), "first_line": 1, "last_line": 3}], whole_source=True)
    editable = ".agent/tasks/nested/plan.md"
    snapshots = f"task/.agent/tmp/task-inputs/{TASK}"
    versions = []
    tree.file(f"task/{editable}", b"final\n")
    for version, data in (("start", b"starting\n"), ("revision", b"approved\n"), ("final", b"final\n")):
        relative = f"{version}/nested/plan.md"
        tree.file(f"{snapshots}/{relative}", data)
        selected(f"{snapshots}/{relative}", f"artifacts/worktree/inputs/{relative}")
        versions.append({"version": version, "path": editable, "sha256": sha256(data), "approval": TURN})
    tree.file(f"{snapshots}/versions.json", json.dumps({
        "task_id": TASK, "versions": versions, "final": "final",
    }).encode())
    selected(f"{snapshots}/versions.json", "artifacts/worktree/inputs/versions.json")
    for profile, attempt, data in (("commit", "5" * 32, b"commit duration\n"),
                                   ("milestone", "6" * 32, b"milestone duration\n")):
        report, selection = timing(tree, profile, attempt, data)
        destination = f"artifacts/verification/{profile}/{attempt}"
        selected(report, f"{destination}/{REPORT}")
        selection_path = report.removesuffix(REPORT) + "selection.json"
        tree.file(selection_path, json.dumps(selection).encode())
        selected(selection_path, f"{destination}/selection.json")

    identity = {
        "start_commit": tree.base, "workflow_commit": tree.base, "final_commit": tree.base,
        "final_candidate": tree.base, "task_id": TASK, "execution_id": "registered",
    }
    manifest = f"{archive}/manifest.json"
    tree.record(manifest, {
        "format_version": 1, "tool_version": "records.py/1", "revision": REVISION,
        "previous_revision": None, **identity, "git_common_dir": common, "root_thread_id": THREAD,
        "start_utc": start_utc, "end_utc": end_utc, "outcome": "completed", "collection_status": "collected",
        "threads": [{"thread_id": THREAD, "parent_thread_id": None, "source": native_path,
                     "selection": scope, "turn_ids": [TURN], "segment": segment, "cli_version": None,
                     "last_event": {"timestamp": end, "source_line": 3,
                                    "byte_start": len(rollout) - len(rollout.splitlines(keepends=True)[-1]),
                                    "byte_end": len(rollout), "type": "event_msg", "event": "task_complete",
                                    "turn_id": TURN, "call_id": None}}],
        "files": inventory, "gaps": [],
    })
    observer = "observer.json"
    tree.record(observer, {
        "identity": identity, "threads": [THREAD], "rows": {
            "starting_inputs": {"checks": [check for path, check in expected.items() if "/inputs/" in path]},
            "registration_lineage": {"checks": [{"kind": "source_range", "path": segment,
                                                "source": native_path, "start": 0, "end": len(rollout)}]},
            "usage_verification": {"checks": [check for path, check in expected.items() if "/verification/" in path]},
            "integrity_survival": {"checks": [{"kind": "commit_ref", "commit": tree.base, "ref": "refs/heads/master"}]},
        },
    })
    tree.history = {"manifest": manifest, "observer": observer}
