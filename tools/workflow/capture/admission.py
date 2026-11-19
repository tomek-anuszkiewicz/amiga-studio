"""Admit OTLP records using verified rollout ancestry and execution time bounds."""

from collections import Counter
from functools import lru_cache
from pathlib import Path

from otlp import DIAGNOSTIC_KEY, ROUTE_KEY, SIGNALS, attributed_records, items, record_interval, within_bounds
from record_sources import OtlpTurnCoverage, common_dir, otlp_turn_coverage, rollout_headers, selected_rollouts
from registry import (
    CaptureError, epoch_ns, path_identity, read_json, rollout_header, rollout_paths, thread_id, write_json,
)


MAX_PARENT_DEPTH = 32
OPEN_END = "9999-12-31T23:59:59Z"
NO_TURN_COVERAGE = OtlpTurnCoverage()


@lru_cache(maxsize=32)
def native_turns(common, codex_home, root, bounds, sources, resolve_common):
    # Cache covered IDs and excluded intervals rather than retaining native records.
    headers = rollout_headers(codex_home, paths=[path for path, *_ in sources])
    selection = dict(bounds)
    registration = {"root_thread_id": root, "threads": [selection]}
    selected_rollouts(codex_home, registration, common, headers=headers, resolve_common=resolve_common)
    # A checked descendant can have only excluded turns and be absent from the
    # selected lineage. Its known intervals still apply before forwarding.
    return {
        thread: otlp_turn_coverage(rollout, selection["end_utc"])
        for thread, candidates in headers.items()
        for rollout in candidates if rollout.turns
    }


def with_route(resource: dict, route: str, diagnostic: bool) -> dict:
    attributes = [item for item in items(resource, "attributes") if item.get("key") not in (ROUTE_KEY, DIAGNOSTIC_KEY)]
    attributes.append({"key": ROUTE_KEY, "value": {"stringValue": route}})
    if diagnostic:
        attributes.append({"key": DIAGNOSTIC_KEY, "value": {"boolValue": True}})
    return {**resource, "attributes": attributes}


class RolloutLineage:
    def __init__(self, project, codex_home: Path, source_id=None):
        self.project = project
        self.codex_home = codex_home.resolve()
        self.source_id = source_id or path_identity(self.codex_home)
        self.path = project.plain_path(project.control / f"lineage-{self.source_id}.json")
        self.verified = read_json(self.path) if self.path.exists() else {}
        self.unsaved = set()

    def parent(self, child: str, resolve_common) -> tuple[str, int] | None:
        if child in self.verified:
            return tuple(self.verified[child])
        candidates = rollout_paths(self.codex_home, child)
        if len(candidates) != 1:
            return None
        try:
            with candidates.pop().open("rb") as stream:
                info = rollout_header(stream, child, self.project.common, resolve_common=resolve_common)
            parent = thread_id(info["source"]["subagent"]["thread_spawn"]["parent_thread_id"])
            evidence = (parent, epoch_ns(info["timestamp"]))
        except (KeyError, OSError, TypeError, ValueError):
            return None
        self.verified[child] = evidence
        self.unsaved.add(child)
        return evidence

    def save(self, proved: set[str]):
        # The persistent cache contains only descendants of a selected thread.
        if proved & self.unsaved:
            durable = {key: value for key, value in self.verified.items() if key not in self.unsaved}
            durable.update({child: self.verified[child] for child in proved})
            self.project.require_ignored(self.path)
            write_json(self.path, durable)
            self.unsaved.difference_update(proved)


class Admission:
    def __init__(self, project, lineage: RolloutLineage):
        self.project = project
        self.lineage = lineage
        self.stats = Counter()

    def filter(self, endpoint: str, message: dict, *, resolve_common=common_dir, diagnostic=False) -> dict:
        resource_key, scope_key, leaf = SIGNALS[endpoint]
        selections = {}
        for path, registration in self.project.registrations():
            for selected in registration["threads"]:
                selections.setdefault(selected["id"], []).append((path, registration, selected))

        parents = {}
        turns = {}
        proved = set()
        watermarks = {}
        counters = Counter()
        output = []
        for group in items(message, resource_key):
            routes = {}
            for scope in items(group, scope_key):
                records = {}
                for record, attrs in attributed_records(leaf, group, scope, ignore_route=True):
                    if attrs is None:
                        counters["unattributed_or_invalid"] += 1
                        continue
                    try:
                        start, end = record_interval(leaf, record, attrs)
                    except (ValueError, OverflowError):
                        counters["unattributed_or_invalid"] += 1
                        continue
                    thread = attrs["conversation.id"]
                    match, descendants = self.match(thread, start, end, selections, parents, counters, resolve_common)
                    if match is None:
                        continue
                    path, registration, selection = match
                    key = (path, selection["id"], thread)
                    if key not in turns:
                        coverage = self.turn_coverage(registration, selection, descendants, parents, resolve_common)
                        observed = coverage.get(thread, NO_TURN_COVERAGE)
                        turn_bounded = (
                            selection.get("start_turn_id") or selection.get("end_turn_id")
                            or (selection["id"] == registration["root_thread_id"] and registration.get("after_turn_id"))
                        )
                        turns[key] = None if turn_bounded and not observed.covered else observed
                    if turns[key] is None or not turns[key].includes(attrs.get("turn.id"), start, end):
                        counters["unattributed_or_invalid"] += 1
                        continue
                    proved.update(descendants)
                    if selection["end_utc"] is not None:
                        counters["accepted_after_stop"] += 1
                    counters["accepted"] += 1
                    route = f"{registration['task_id']}/{registration['execution_id']}"
                    records.setdefault(route, []).append(record)
                    watermarks[path] = (registration, max(end, watermarks.get(path, (None, 0))[1]))
                for route, accepted in records.items():
                    routes.setdefault(route, []).append({**scope, leaf: accepted})
            for route, scopes in routes.items():
                self.project.require_ignored(self.project.archive / route / "otel")
                output.append({
                    **group, "resource": with_route(group.get("resource", {}), route, diagnostic), scope_key: scopes,
                })

        # Reserve the latest admitted timestamp before forwarding so a failed request
        # with a partly completed export cannot permit an inconsistent stop.
        for path, (registration, latest) in watermarks.items():
            if latest > registration.get("latest_admitted_ns", 0):
                self.project.require_ignored(path)
                registration["latest_admitted_ns"] = latest
                write_json(path, registration)
        self.lineage.save(proved)
        self.stats.update(counters)
        return {resource_key: output} if output else {}

    def parent(self, thread, parents, resolve_common):
        if thread not in parents:
            parents[thread] = self.lineage.parent(thread, resolve_common)
        return parents[thread]

    def match(self, thread, start, end, selections, parents, counters, resolve_common):
        descendants = {}
        registered = False
        for _ in range(MAX_PARENT_DEPTH):
            if thread in descendants:
                break
            choices = selections.get(thread, ())
            registered |= bool(choices)
            matches = [choice for choice in choices if self.in_range(choice[2], start, end, descendants)]
            if matches:
                # A nearer explicit registration owns its applicable interval,
                # even while OTLP is disabled or registration is still pending.
                if len(matches) == 1 and matches[0][1]["otel"] and "pending_after_turn_id" not in matches[0][1]:
                    return matches[0], descendants
                break
            parent = self.parent(thread, parents, resolve_common)
            if parent is None:
                break
            parent_id, spawned = parent
            descendants[thread] = spawned
            thread = parent_id
        counters["outside_selection" if registered else "unresolved"] += 1
        return None, {}

    def turn_coverage(self, registration, selection, descendants, parents, resolve_common):
        root = selection["id"]
        threads = {root, *descendants}
        if (parent := self.parent(root, parents, resolve_common)) is not None:
            # A selected child can carry copied turns from its immediate parent.
            threads.add(parent[0])
        sources = []
        try:
            for thread in threads:
                for path in rollout_paths(self.lineage.codex_home, thread):
                    stat = path.stat()
                    sources.append((path, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns, stat.st_ino))
            bounds = {**selection, "end_utc": selection["end_utc"] or OPEN_END}
            if root == registration["root_thread_id"] and registration.get("after_turn_id"):
                bounds["after_turn_id"] = registration["after_turn_id"]
            return native_turns(
                self.project.common, self.lineage.codex_home, root,
                tuple(sorted(bounds.items())), tuple(sorted(sources)), resolve_common,
            )
        except (OSError, KeyError, TypeError, ValueError, OverflowError):
            return {}

    @staticmethod
    def in_range(selection: dict, start: int, end: int, descendants: dict[str, int]) -> bool:
        if not within_bounds(selection, start, end):
            return False
        lower = epoch_ns(selection["start_utc"])
        # Parent-to-child spawn times must be ordered inside the selected range.
        previous = start
        for child_started in descendants.values():
            if not lower <= child_started <= previous:
                return False
            previous = child_started
        return True
