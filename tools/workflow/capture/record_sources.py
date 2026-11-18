"""Read and correlate selected execution sources without rewriting their records."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import time

from otlp import OTLP_SIGNALS, attributed_records, items, record_interval, within_bounds

from registry import CaptureError, git_path


TERMINAL_EVENTS = {"task_complete", "turn_aborted", "turn_failed"}
SAFE_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]*\Z")
SECRET_NAMES = {
    ".codex", "sessions", "archived_sessions", ".docker", ".kube", "gcloud",
    ".env", ".netrc", ".npmrc", ".pypirc", ".git-credentials", "auth.json",
    "credentials.json", "credentials.toml", "credentials", "secrets.json",
    "secrets.toml", "secrets", "token.json", "tokens.json", "id_rsa", "id_ed25519",
}


def identifier(value):
    if not isinstance(value, str) or not SAFE_ID.fullmatch(value) or value.endswith("."):
        raise ValueError(f"Invalid identifier: {value!r}")
    return value


def utc(value):
    if not isinstance(value, str):
        raise ValueError("Expected an offset-aware timestamp")
    stamp = datetime.fromisoformat(value)
    if stamp.tzinfo is None:
        raise ValueError("Expected an offset-aware timestamp")
    return stamp.astimezone(timezone.utc)


def ns(stamp):
    delta = stamp - datetime(1970, 1, 1, tzinfo=timezone.utc)
    return (delta.days * 86400 + delta.seconds) * 10**9 + delta.microseconds * 1000


def common_dir(cwd):
    try:
        return git_path(cwd, "--git-common-dir")
    except (CaptureError, OSError):
        return None


def protected_name(path):
    return any(
        part.casefold() in SECRET_NAMES | {".git", ".ssh", ".aws", ".gnupg", "task-archives"}
        or part.casefold().startswith((".env.", "credentials.", "secrets."))
        or part.casefold().endswith((".pem", ".key", ".pfx", ".kdbx"))
        for part in path.parts
    )


def linked(path):
    return path.is_symlink() or getattr(path, "is_junction", lambda: False)()


def safe_file(path, *, single_link=False):
    if any(linked(parent) for parent in (path, *path.parents)) or not path.is_file():
        raise ValueError(f"Expected a regular file without links: {path}")
    if single_link and path.stat().st_nlink != 1:
        raise ValueError(f"Hard-linked input is outside the selected file boundary: {path}")
    return path


def snapshot(path):
    def state():
        stat = path.stat()
        return (stat.st_size, stat.st_mtime_ns, stat.st_ino)

    before = state()
    data = path.read_bytes()
    time.sleep(0.05)
    return data, before != state() or len(data) != before[0]


@dataclass
class Line:
    number: int
    start: int
    end: int
    raw: bytes
    value: dict


def json_lines(data):
    """Return the well-formed newline-terminated prefix and describe an invalid tail."""
    rows = []
    offset = 0
    for number, part in enumerate(data.split(b"\n")[:-1], 1):
        raw = part + b"\n"
        try:
            value = json.loads(raw)
            if not isinstance(value, dict):
                raise ValueError("Expected an object")
        except (ValueError, UnicodeError):
            return rows, f"Invalid JSONL at line {number}, byte {offset}"
        rows.append(Line(number, offset, offset + len(raw), raw, value))
        offset += len(raw)
    return rows, f"Unterminated JSONL at byte {offset}" if offset < len(data) else None


def raw_selection(lines):
    ranges = []
    for row in lines:
        if ranges and ranges[-1]["end"] == row.start:
            ranges[-1]["end"] = row.end
            ranges[-1]["last_line"] = row.number
        else:
            ranges.append({
                "start": row.start, "end": row.end,
                "first_line": row.number, "last_line": row.number,
            })
    return b"".join(row.raw for row in lines), ranges


@dataclass
class Rollout:
    path: Path
    thread: str
    parent: str | None
    created: datetime
    cwd: Path | None
    header: dict
    lines: list[Line] = field(default_factory=list)
    selected: list[Line] = field(default_factory=list)
    turns: dict = field(default_factory=dict)
    covered_turns: set = field(default_factory=set)
    gap: str | None = None


def rollout_files(codex_home):
    for directory in (codex_home / "sessions", codex_home / "archived_sessions"):
        if linked(directory):
            continue
        for current, subdirs, names in os.walk(directory):
            root = Path(current)
            subdirs[:] = [name for name in subdirs if not linked(root / name)]
            for name in names:
                if name.startswith("rollout-") and name.endswith(".jsonl"):
                    yield root / name


def rollout_headers(codex_home, *, paths=None):
    found = {}
    for path in rollout_files(codex_home) if paths is None else paths:
        name = path.name
        try:
            safe_file(path)
            with path.open("rb") as stream:
                first = json.loads(stream.readline())
            payload = first["payload"]
            if first["type"] != "session_meta":
                continue
            thread = identifier(payload["id"])
            if not name.endswith(f"-{thread}.jsonl"):
                continue
            source = payload.get("source")
            parent = None
            if isinstance(source, dict):
                subagent = source.get("subagent")
                if isinstance(subagent, dict):
                    spawn = subagent.get("thread_spawn")
                    if isinstance(spawn, dict):
                        parent = identifier(spawn["parent_thread_id"])
            cwd = payload.get("cwd")
            rollout = Rollout(path, thread, parent, utc(first["timestamp"]),
                              Path(cwd) if cwd else None, first)
        except (OSError, KeyError, TypeError, ValueError):
            continue
        found.setdefault(thread, []).append(rollout)
    return found


def unique_rollout(candidates):
    if len(candidates) == 1:
        return candidates[0]
    if candidates:
        original = candidates[0].path.read_bytes()
        if all(candidate.path.read_bytes() == original for candidate in candidates[1:]):
            return candidates[0]
    return None


def rollout_turns(rows):
    turns = {}
    active = None
    for row in rows:
        item = row.value
        payload = item.get("payload")
        if not isinstance(payload, dict):
            active = None
            continue
        kind = item.get("type")
        event = payload.get("type")
        thread_turn = payload.get("turn_id")
        if kind == "session_meta":
            active = None
            continue
        if kind == "event_msg" and event == "task_started" and isinstance(thread_turn, str):
            active = thread_turn
            turns[active] = {
                "start": row, "end": None, "rows": [], "root": payload.get("root_turn_id"),
                "cwd": [], "thread_ids": set(),
            }
        if active is None or (thread_turn is not None and thread_turn != active):
            continue
        turn = turns[active]
        turn["rows"].append(row)
        if kind == "turn_context" and isinstance(payload.get("cwd"), str):
            turn["cwd"].append(Path(payload["cwd"]))
        if isinstance(payload.get("thread_id"), str):
            turn["thread_ids"].add(payload["thread_id"])
        if kind == "event_msg" and event in TERMINAL_EVENTS:
            turn["end"] = row
            active = None
    return turns


def archived_turns(rows, first_range, thread):
    turns = rollout_turns(rows)
    if rows[0].value.get("type") == "session_meta" or any(
        turn["start"] is rows[0] for turn in turns.values()
    ):
        return turns, True
    if first_range["start"] <= 0 or first_range["first_line"] <= 1:
        return turns, False

    # Only a contiguous leading fragment can lack its opening event. The
    # retained native terminal event supplies the ID independently of the manifest.
    prefix = []
    ids = set()
    count = first_range["last_line"] - first_range["first_line"] + 1
    if not 0 < count <= len(rows) or rows[count - 1].end != first_range["end"] - first_range["start"]:
        return turns, False
    for row in rows[:count]:
        kind = row.value.get("type")
        payload = row.value.get("payload")
        if kind == "session_meta" or not isinstance(payload, dict):
            break
        if payload.get("thread_id", thread) != thread:
            break
        event = payload.get("type") if kind == "event_msg" else None
        if event == "task_started":
            break
        turn_id = payload.get("turn_id")
        if turn_id is not None:
            if not isinstance(turn_id, str) or not SAFE_ID.fullmatch(turn_id):
                break
            ids.add(turn_id)
        prefix.append(row)
        if event in TERMINAL_EVENTS:
            if turn_id is None or ids != {turn_id} or turn_id in turns:
                break
            turns[turn_id] = {"start": None, "end": row, "rows": prefix, "root": None,
                              "cwd": [], "thread_ids": set()}
            return turns, True
    return turns, False


def select_rollout(rollout, bounds, repository, *, ancestors=(), covered_roots=frozenset(), inherited_turns=()):
    data, changing = snapshot(rollout.path)
    rows, error = json_lines(data)
    if not rows or rows[0].value != rollout.header:
        rollout.gap = "The rollout header changed while collecting"
        return
    rollout.lines = rows
    rollout.turns = rollout_turns(rows)
    start, end = utc(bounds["start_utc"]), utc(bounds["end_utc"])
    start_turn, end_turn = bounds.get("start_turn_id"), bounds.get("end_turn_id")
    after_turn = bounds.get("after_turn_id")
    gaps = []
    if changing:
        gaps.append("Source changed during the bounded read")
    if error:
        gaps.append(error)
    for marker in (start_turn, end_turn, after_turn):
        if marker and marker not in rollout.turns:
            gaps.append(f"Boundary turn not found: {marker}")
    if all(marker is None or marker in rollout.turns for marker in (start_turn, end_turn, after_turn)):
        inherited = set(inherited_turns).union(*(item.turns.keys() for item in ancestors))
        first_number = rollout.turns[start_turn]["start"].number if start_turn else 0
        if after_turn:
            first_number = max(first_number, rollout.turns[after_turn]["start"].number + 1)
        last_start = rollout.turns[end_turn]["start"].number if end_turn else len(rows)
        selected = []
        admitted_turns = set()
        for turn_id, turn in rollout.turns.items():
            if not first_number <= turn["start"].number <= last_start:
                continue
            if turn_id in inherited or (ancestors and turn["root"] not in covered_roots):
                continue
            if turn["thread_ids"] - {rollout.thread}:
                continue
            try:
                selected_rows = [
                    row for row in turn["rows"] if start <= utc(row.value.get("timestamp")) <= end
                ]
            except ValueError:
                gaps.append(f"Unusable timestamp in turn {turn_id}")
                continue
            if not selected_rows:
                continue
            if any(common_dir(cwd) != repository for cwd in turn["cwd"]):
                gaps.append(f"Git identity could not be verified for turn {turn_id}")
                continue
            admitted_turns.add(turn_id)
            selected.extend(selected_rows)
            if turn["end"] is None or turn["end"] not in selected_rows:
                gaps.append(f"Terminal event unavailable inside the selected range for {turn_id}")
        # A fresh child may contain an inherited parent prefix with rewritten timestamps.
        # Keep its own header, never the inherited prefix. A dedicated root can preserve
        # session-level rows when its entire timeline and every turn belong to this run.
        if selected and start <= rollout.created <= end:
            if rollout.parent is None and admitted_turns == set(rollout.turns) and all(
                row.value.get("type") != "session_meta" for row in rows[1:]
            ):
                try:
                    all_in_range = all(start <= utc(row.value.get("timestamp")) <= end for row in rows)
                except ValueError:
                    all_in_range = False
                if all_in_range:
                    selected = rows
            else:
                selected.insert(0, rows[0])
        rollout.selected = selected
        rollout.covered_turns = admitted_turns
    if not rollout.selected:
        gaps.append("No attributable records in the selected range")
    rollout.gap = "; ".join(gaps) or None


def selected_rollouts(codex_home, registration, repository, *, headers=None):
    if headers is None:
        headers = rollout_headers(codex_home)
    admitted = {}
    boundary = {}
    gaps = {}
    for thread_bounds in registration["threads"]:
        thread = identifier(thread_bounds["id"])
        if thread == registration["root_thread_id"] and registration.get("after_turn_id"):
            thread_bounds = {**thread_bounds, "after_turn_id": registration["after_turn_id"]}
        boundary[thread] = thread_bounds
        item = unique_rollout(headers.get(thread, []))
        if item is None:
            gaps[thread] = "Rollout missing or conflicting copies share the same identity"
        elif item.cwd is None or common_dir(item.cwd) != repository:
            gaps[thread] = "Rollout checkout does not verify against the registered Git identity"
        elif item.parent is not None:
            # An explicitly resumed child still must exclude its copied parent history.
            parent = unique_rollout(headers.get(item.parent, []))
            if parent is None:
                gaps[thread] = "Parent history is unavailable to separate the resumed child's own turns"
                continue
            parent_rows, _ = json_lines(parent.path.read_bytes())
            parent_turns = rollout_turns(parent_rows)
            select_rollout(item, thread_bounds, repository, inherited_turns=parent_turns)
            admitted[thread] = (item, ())
        else:
            select_rollout(item, thread_bounds, repository)
            admitted[thread] = (item, ())
    while True:
        newly_admitted = {}
        for thread, candidates in headers.items():
            if thread in boundary:
                continue
            verified = [candidate for candidate in candidates if candidate.parent in admitted]
            if not verified:
                continue
            roots = []
            for candidate in verified:
                parent, ancestors = admitted[candidate.parent]
                root = ancestors[-1] if ancestors else parent
                if not parent.covered_turns:
                    continue
                bounds = boundary[root.thread]
                if utc(bounds["start_utc"]) <= candidate.created <= utc(bounds["end_utc"]):
                    roots.append((candidate, bounds, (parent, *ancestors), root.covered_turns))
            if not roots:
                continue
            item = unique_rollout(candidates)
            if item is None:
                gaps[thread] = "Conflicting descendant rollouts share the same identity"
                continue
            candidate, bounds, ancestors, covered = next(
                (entry for entry in roots if entry[0].path == item.path), roots[0]
            )
            if candidate.cwd is None or common_dir(candidate.cwd) != repository:
                gaps[thread] = "Descendant checkout does not verify against the registered Git identity"
                continue
            child_bounds = {"start_utc": candidate.created.isoformat(), "end_utc": bounds["end_utc"]}
            # An intermediate parent also has to have a covered turn carrying this root turn.
            parent = ancestors[0]
            if parent is not ancestors[-1]:
                covered = covered & {parent.turns[key]["root"] for key in parent.covered_turns}
            select_rollout(candidate, child_bounds, repository, ancestors=ancestors, covered_roots=covered)
            if candidate.covered_turns:
                newly_admitted[thread] = (candidate, ancestors)
                boundary[thread] = child_bounds
            else:
                gaps[thread] = candidate.gap
        if not newly_admitted:
            break
        admitted.update(newly_admitted)
    lineage = {key: item for key, (item, _) in admitted.items()}
    gaps.update({key: item.gap for key, item in lineage.items() if item.gap})
    return lineage, boundary, gaps


def response_text(payload):
    if isinstance(payload.get("content"), list):
        return "".join(
            part.get("text", "") for part in payload["content"]
            if isinstance(part, dict) and isinstance(part.get("text"), str)
        )
    return ""


def cli_turn(rows, lineage, requested, *, allow_incomplete=False):
    if not rows or rows[0].value.get("type") != "thread.started":
        raise ValueError("CLI stream has no leading thread.started")
    thread = rows[0].value.get("thread_id")
    rollout = lineage.get(thread)
    if rollout is None:
        raise ValueError("CLI stream thread is outside the verified execution")
    starts = [row.value for row in rows if row.value.get("type") == "turn.started"]
    terminals = [row.value for row in rows if row.value.get("type") in {"turn.completed", "turn.failed"}]
    if (sum(row.value.get("type") == "thread.started" for row in rows) != 1
            or len(starts) != 1 or len(terminals) > 1
            or (terminals and rows[-1].value != terminals[0])
            or (not terminals and not allow_incomplete)):
        raise ValueError("CLI stream cannot be joined as a single admitted turn")
    messages = [
        row.value["item"]["text"] for row in rows
        if row.value.get("type") == "item.completed" and isinstance(row.value.get("item"), dict)
        and row.value["item"].get("type") == "agent_message"
        and isinstance(row.value["item"].get("text"), str)
    ]
    ids = {item["turn_id"] for item in starts + terminals if item.get("turn_id")}
    if len(ids) > 1:
        raise ValueError("CLI stream reports conflicting turn IDs")
    matches = []
    for turn_id, turn in rollout.turns.items():
        native = [
            response_text(row.value["payload"]) for row in turn["rows"]
            if row.value.get("type") == "response_item"
            and row.value["payload"].get("type") == "message"
            and row.value["payload"].get("role") == "assistant"
            and row.value["payload"].get("phase") in {"commentary", "final_answer"}
        ]
        if not terminals:
            native = native[:len(messages)]
        if (turn["end"] or not terminals) and (turn_id in ids or (not ids and messages and messages == native)):
            matches.append(turn_id)
    if len(matches) != 1 or matches[0] not in rollout.covered_turns:
        raise ValueError("CLI output does not uniquely match an admitted native turn")
    turn_id = matches[0]
    if requested is not None and requested != turn_id:
        raise ValueError("CLI turn differs from the selected turn ID")
    turn = rollout.turns[turn_id]
    if turn["start"] is None or any(row not in rollout.selected for row in turn["rows"]):
        raise ValueError("CLI stream includes a turn that is only partly inside the selected boundary")
    return thread, turn_id, bool(terminals)



def otlp_envelope(value):
    if len(value) != 1 or next(iter(value)) not in OTLP_SIGNALS:
        raise ValueError("Only OTLP log and span envelopes are supported")
    signal = next(iter(value))
    resources = items(value, signal)
    if not resources:
        raise ValueError("Empty OTLP resource envelope")
    return signal, resources


def otlp_items(value):
    signal, resources = otlp_envelope(value)
    scope_key, leaf = OTLP_SIGNALS[signal]
    for resource in resources:
        for group in items(resource, scope_key):
            for item, attrs in attributed_records(leaf, resource, group):
                yield leaf, item, attrs


@dataclass(frozen=True)
class OtlpTurnCoverage:
    covered: frozenset[str] = frozenset()
    excluded: tuple[tuple[int, int], ...] = ()

    def includes(self, turn, start, end):
        if turn is not None:
            return turn in self.covered
        return not any(first <= end and start <= last for first, last in self.excluded)


def otlp_turn_coverage(rollout, end_utc):
    excluded = []
    for key, other in rollout.turns.items():
        if key in rollout.covered_turns:
            continue
        first = ns(utc(other["start"].value["timestamp"]))
        last = ns(utc(other["end"].value["timestamp"] if other["end"] else end_utc))
        excluded.append((first, last))
    return OtlpTurnCoverage(frozenset(rollout.covered_turns), tuple(excluded))


def otlp_record_in_range(leaf, item, attrs, lineage, bounds, coverage):
    thread = attrs["conversation.id"]
    rollout = lineage.get(thread)
    if rollout is None or not rollout.covered_turns:
        return False
    start, end = record_interval(leaf, item, attrs)
    if not within_bounds(bounds[thread], start, end):
        return False
    if thread not in coverage:
        coverage[thread] = otlp_turn_coverage(rollout, bounds[thread]["end_utc"])
    return coverage[thread].includes(attrs.get("turn.id"), start, end)


def otlp_selection(value, lineage, bounds, *, route=None):
    # Validate every record. Only mixed envelopes are encoded again.
    signal, resources = otlp_envelope(value)
    scope_key, leaf = OTLP_SIGNALS[signal]
    kept_resources = []
    dropped = 0
    coverage = {}
    for resource in resources:
        kept_groups = []
        for group in items(resource, scope_key):
            kept_items = []
            for item, attrs in attributed_records(leaf, resource, group, expected_route=route):
                try:
                    if attrs is not None and otlp_record_in_range(leaf, item, attrs, lineage, bounds, coverage):
                        kept_items.append(item)
                        continue
                except (KeyError, TypeError, ValueError, OverflowError):
                    pass
                dropped += 1
            if kept_items:
                kept_groups.append({**group, leaf: kept_items})
        if kept_groups:
            kept_resources.append({**resource, scope_key: kept_groups})
    if not kept_resources:
        return None, dropped
    if not dropped:
        return value, 0
    return {signal: kept_resources}, dropped
