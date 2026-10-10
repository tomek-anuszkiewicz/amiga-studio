"""Shared calls, completion handling, response caching and metrics."""

import json
import os
import time

from .cache import ResponseCache, identity
from .config import selection
from .transport import CodexTransport
from .metrics import record


_REQUEST_TIMEOUT_SECONDS = 180


class CodexClient:
    def __init__(self, config, *, stage, operation, transport=None, cache=None, timeout=_REQUEST_TIMEOUT_SECONDS):
        self.selected = selection(config, stage)
        self.operation = operation
        self.timeout = timeout
        self.transport = transport or CodexTransport()
        self.cache = cache or (ResponseCache(os.environ["CONVERSION_CACHE_DIR"]) if os.environ.get("CONVERSION_CACHE_DIR") else ResponseCache())
        self.call_count = 0
        self.cached_call_count = 0
        self.metrics = []

    def _generate(self, prompt, images=(), validator=None):
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
            started = time.monotonic()
            try:
                result = self.transport.run(self.selected, prompt, images=images, timeout=self.timeout)
                checked(result)
            except Exception as error:
                self._record({"stage": self.selected.stage, "model": self.selected.model,
                                     "reasoning_effort": self.selected.reasoning_effort, "cached": False,
                                     "status": "failed", "error_type": type(error).__name__,
                                     "duration_seconds": time.monotonic() - started})
                raise
            self.cache.save(request, result)
            cached = False
        self._record({"stage": self.selected.stage, "model": self.selected.model,
                             "reasoning_effort": self.selected.reasoning_effort, "cached": cached,
                             "usage": result.get("usage"), "duration_seconds": result.get("duration_seconds"),
                             "events": result.get("events"), "execution": result.get("execution")})
        return result["text"]

    def _record(self, metric):
        self.metrics.append(metric)
        record(metric)

    def generate_json(self, prompt, *, image_path=None):
        images = [] if image_path is None else (image_path if isinstance(image_path, (list, tuple)) else [image_path])
        return json.loads(self._generate(prompt, images, validator=json.loads))

    def close(self):
        self.transport.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
