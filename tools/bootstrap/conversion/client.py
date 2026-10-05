"""Stage-bound calls; validate completion and JSON before publishing cache entries."""

import json
import os
import time
from jsonschema import validate

from .cache import ResponseCache, identity
from .config import selection, validate_config, HTML_STAGES, PDF_STAGES
from .transport import CodexTransport
from .metrics import record


class CodexClient:
    def __init__(self, config, *, stage, transport=None, cache=None, images=False):
        known = HTML_STAGES if stage in HTML_STAGES else PDF_STAGES
        validate_config(config, known, {stage})
        self.selected = selection(config, stage)
        self.timeout = config["llm"].get("timeout_seconds", 180)
        self.transport = transport or CodexTransport()
        self.cache = cache or (ResponseCache(os.environ["CONVERSION_CACHE_DIR"]) if os.environ.get("CONVERSION_CACHE_DIR") else ResponseCache())
        self.call_count = 0
        self.cached_call_count = 0
        self.metrics = []
        try:
            self.transport.validate(self.selected, images=images)
        except BaseException:
            self.close()
            raise

    def _generate(self, prompt, images=(), schema=None, validator=None):
        self.transport.validate(self.selected, images=bool(images))
        request = identity(self.selected, prompt, images, schema)

        def checked(result):
            if result.get("status") != "completed" or not isinstance(result.get("text"), str) or not result["text"].strip():
                raise RuntimeError("Incomplete conversion result")
            if schema is not None:
                validate(json.loads(result["text"]), schema)
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
                result = self.transport.run(self.selected, prompt, images=images, schema=schema, timeout=self.timeout)
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

    def generate_text(self, prompt, *, validator=None):
        return self._generate(prompt, validator=validator)

    def generate_vision(self, prompt, image_path, *, validator=None):
        images = image_path if isinstance(image_path, (list, tuple)) else [image_path]
        return self._generate(prompt, images, validator=validator)

    def generate_json(self, prompt, *, schema, image_path=None, validator=None):
        images = [] if image_path is None else (image_path if isinstance(image_path, (list, tuple)) else [image_path])
        checked = (lambda text: validator(json.loads(text))) if validator is not None else None
        return json.loads(self._generate(prompt, images, schema, checked))

    def close(self):
        self.transport.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
