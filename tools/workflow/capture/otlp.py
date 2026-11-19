"""Interpret the attribution fields of OTLP/HTTP JSON records."""

import re

from registry import CaptureError, epoch_ns, thread_id


SIGNALS = {
    "/v1/logs": ("resourceLogs", "scopeLogs", "logRecords"),
    "/v1/traces": ("resourceSpans", "scopeSpans", "spans"),
}
OTLP_SIGNALS = {resource: (scope, leaf) for resource, scope, leaf in SIGNALS.values()}
ROUTE_KEY = "capture.route"
DIAGNOSTIC_KEY = "capture.diagnostic"
ID_KEYS = ("conversation.id", "thread.id", "turn.id")
SINGLE_STRING_KEYS = (*ID_KEYS, ROUTE_KEY, "event.timestamp")
NANOSECONDS = re.compile(r"[0-9]{1,19}")
SOURCE_TIME = re.compile(
    r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.([0-9]{1,9}))?(Z|[+-]\d{2}:\d{2})"
)


def items(value: dict, key: str) -> list[dict]:
    if not isinstance(value, dict):
        raise CaptureError("Malformed OTLP object.")
    children = value.get(key, [])
    if not isinstance(children, list) or any(not isinstance(child, dict) for child in children):
        raise CaptureError(f"Malformed OTLP {key}.")
    return children


def attributes(record: dict, *, span: bool = False, ignore_route: bool = False) -> dict:
    result = {}
    ambiguous = set()
    runtime_thread_seen = False
    for item in items(record, "attributes"):
        key, value = item.get("key"), item.get("value")
        if ignore_route and key == ROUTE_KEY:
            continue
        if not isinstance(key, str) or not isinstance(value, dict) or len(value) != 1:
            raise CaptureError("Malformed OTLP attribute.")
        kind, scalar = next(iter(value.items()))
        if span and key == "thread.id" and kind == "intValue":
            # Codex spans can carry the runtime thread number beside the
            # conversation thread ID. Leave both in the original payload.
            if runtime_thread_seen or not valid_unsigned_number(scalar):
                raise CaptureError("Ambiguous OTLP runtime thread number.")
            runtime_thread_seen = True
            continue
        if key in SINGLE_STRING_KEYS:
            if key in result or kind != "stringValue" or not isinstance(scalar, str) or not scalar:
                raise CaptureError(f"Missing or ambiguous OTLP string attribute {key}.")
        if key in result:
            # Keep repeated data in the payload without projecting a winner.
            ambiguous.add(key)
            del result[key]
        elif key not in ambiguous:
            result[key] = scalar
    return result


def record_attributes(leaf: str, item: dict, scope: dict, resource: dict) -> dict:
    direct = attributes(item, span=leaf == "spans")
    thread = thread_id(direct.get("conversation.id"))
    if direct.get("thread.id", thread) != thread:
        raise CaptureError("OTLP conversation and thread IDs disagree.")
    if any(key in context and context[key] != direct.get(key)
           for context in (scope, resource) for key in ID_KEYS):
        raise CaptureError("OTLP record identity disagrees with its scope or resource.")
    return direct


def valid_unsigned_number(value) -> bool:
    return type(value) in (int, str) and NANOSECONDS.fullmatch(str(value)) is not None


def event_time(value) -> int:
    if not valid_unsigned_number(value) or not 0 < int(value) < 2**63:
        raise CaptureError("A record has no usable OTLP event time.")
    return int(value)


def record_interval(leaf: str, item: dict, attrs: dict) -> tuple[int, int]:
    if leaf == "spans":
        start = event_time(item.get("startTimeUnixNano"))
        end = event_time(item.get("endTimeUnixNano"))
        if end < start:
            raise CaptureError("OTLP span end precedes its start.")
        return start, end
    if "event.timestamp" in attrs:
        source = SOURCE_TIME.fullmatch(attrs["event.timestamp"])
        if source is None:
            raise CaptureError("Invalid OTLP source timestamp.")
        stamp, fraction, zone = source.groups()
        instant = epoch_ns(stamp + zone) + int((fraction or "").ljust(9, "0"))
        start = event_time(instant)
    else:
        start = event_time(item.get("timeUnixNano"))
    return start, start



ANY_ROUTE = object()


def attributed_records(leaf, group, scope, *, ignore_route=False, expected_route=ANY_ROUTE):
    records = items(scope, leaf)
    try:
        resource_attrs = attributes(group.get("resource", {}), ignore_route=ignore_route)
        scope_attrs = attributes(scope.get("scope", {}))
        if expected_route is not ANY_ROUTE and resource_attrs.get(ROUTE_KEY, expected_route) != expected_route:
            raise CaptureError("OTLP resource does not match the selected route.")
    except CaptureError:
        yield from ((record, None) for record in records)
        return
    for record in records:
        try:
            attrs = record_attributes(leaf, record, scope_attrs, resource_attrs)
        except CaptureError:
            attrs = None
        yield record, attrs


def within_bounds(selection, start, end):
    lower = epoch_ns(selection["start_utc"])
    upper = selection["end_utc"]
    return lower <= start <= end and (upper is None or end <= epoch_ns(upper))
