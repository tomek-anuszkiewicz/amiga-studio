"""Shared calls, completion handling, response caching and metrics."""

import json
import math
import os
from pathlib import Path
import time

from .cache import ResponseCache, identity
from .config import selection
from .transport import CodexTransport
from .metrics import record
from .trace import trace


_REQUEST_TIMEOUT_SECONDS = 180


class CodexClient:
    def __init__(self, config, *, stage, operation, transport=None, cache=None,
                 timeout=_REQUEST_TIMEOUT_SECONDS, cancelled=None):
        self.selected = selection(config, stage)
        delay = config["llm"].get("post_request_delay_seconds", 0)
        if type(delay) not in (int, float) or not math.isfinite(delay) or delay < 0:
            raise ValueError("llm.post_request_delay_seconds must be a nonnegative finite number")
        self.post_request_delay_seconds = delay
        self._previous_request_finished = None
        self.operation = operation
        self.timeout = timeout
        self.cancelled = cancelled
        self.transport = transport or CodexTransport()
        self.cache = cache or (ResponseCache(os.environ["CONVERSION_CACHE_DIR"]) if os.environ.get("CONVERSION_CACHE_DIR") else ResponseCache())
        self.call_count = 0
        self.cached_call_count = 0
        self.metrics = []

    def _generate(self, prompt, images=(), validator=None):
        if self.cancelled is not None and self.cancelled.is_set():
            raise RuntimeError("Codex task was stopped")
        request = identity(self.selected, prompt, images, operation=self.operation)

        def checked(result):
            if result.get("status") != "completed" or not isinstance(result.get("text"), str) or not result["text"].strip():
                raise RuntimeError("Incomplete conversion result")
            if validator is not None:
                validator(result["text"])

        result = self.cache.load(request)
        if result is not None:
            try:
                checked(result)
            except Exception as error:
                # Corrupt cache entries never become successful conversions.
                raise RuntimeError("Invalid cached conversion response; remove the corrupt entry") from error
            self.cached_call_count += 1
            cached = True
        else:
            self.call_count += 1
            if self._previous_request_finished is not None:
                remaining = self.post_request_delay_seconds - (
                    time.monotonic() - self._previous_request_finished)
                if remaining > 0:
                    trace("request.wait", stage=self.selected.stage, seconds=f"{remaining:.1f}")
                    time.sleep(remaining)
            started = time.monotonic()
            try:
                options = {"images": images, "timeout": self.timeout}
                if self.cancelled is not None:
                    options["cancelled"] = self.cancelled
                result = self.transport.run(self.selected, prompt, **options)
                self._previous_request_finished = time.monotonic()
                checked(result)
                if self.cancelled is not None and self.cancelled.is_set():
                    raise RuntimeError("Codex task was stopped")
            except Exception as error:
                trace("request.failed", stage=self.selected.stage, error=type(error).__name__)
                self._record({"stage": self.selected.stage, "model": self.selected.model,
                                     "reasoning_effort": self.selected.reasoning_effort, "cached": False,
                                     "status": "failed", "error_type": type(error).__name__,
                                     "duration_seconds": time.monotonic() - started})
                raise
            self.cache.save(request, result)
            cached = False
            trace("request.completed", stage=self.selected.stage,
                  seconds=f"{time.monotonic() - started:.1f}")
        self._record({"stage": self.selected.stage, "model": self.selected.model,
                             "reasoning_effort": self.selected.reasoning_effort, "cached": cached,
                             "usage": result.get("usage"), "duration_seconds": result.get("duration_seconds"),
                             "events": result.get("events"), "execution": result.get("execution")})
        return result["text"]

    def _record(self, metric):
        self.metrics.append(metric)
        record(metric)

    def generate_json(self, prompt, *, image_path=None, validator=None):
        images = [] if image_path is None else (image_path if isinstance(image_path, (list, tuple)) else [image_path])
        def parse(text):
            try:
                value = json.loads(text)
                if validator is not None:
                    validator(value)
                return value
            except json.JSONDecodeError as error:
                error.add_note(json.dumps({"stage": self.selected.stage, "operation": self.operation,
                                           "input_images": [str(Path(p).resolve()) for p in images],
                                           "response": text}, ensure_ascii=False))
                raise
        return parse(self._generate(prompt, images, validator=parse))

    def close(self):
        self.transport.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
