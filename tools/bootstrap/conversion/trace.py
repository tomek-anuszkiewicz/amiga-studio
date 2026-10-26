"""Opt-in, content-free live trace for conversion requests."""

from datetime import datetime, timezone
import os
import sys
from threading import Lock, current_thread


_LOCK = Lock()


def trace(event, **fields):
    if os.environ.get("CONVERSION_TRACE") != "1":
        return
    timestamp = datetime.now(timezone.utc).isoformat(timespec="seconds")
    details = " ".join(f"{key}={value}" for key, value in fields.items())
    with _LOCK:
        print(f"[{timestamp}] [codex-trace] worker={current_thread().name} {event} {details}".rstrip(),
              file=sys.stderr, flush=True)
