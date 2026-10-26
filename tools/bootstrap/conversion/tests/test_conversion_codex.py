"""Offline conversion contracts: explicit selection, cache validity and failure propagation."""

import importlib.util
from contextlib import redirect_stderr
from io import StringIO
import json
import os
from pathlib import Path
import sys
import tempfile
import threading
from concurrent.futures import ThreadPoolExecutor
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "tools/bootstrap"))
sys.path.insert(0, str(ROOT / "tools/bootstrap/html-to-markdown"))
sys.path.insert(0, str(ROOT / "tools/bootstrap/pdf-to-markdown"))
from conversion import CodexClient
from support.client import HtmlClient
from support.config import load_config, validate_config, HTML_STAGES
from support.transport import HtmlTransport
from common.pdf_config import load_config as load_pdf_config
from conversion.cache import ResponseCache, digest, identity
from conversion.config import selection
from conversion.transport import CodexTransport
from conversion.parallel import convert_parallel
from conversion.metrics import record

spec = importlib.util.spec_from_file_location("html_pipeline", ROOT / "tools/bootstrap/html-to-markdown/pipeline.py")
html = importlib.util.module_from_spec(spec)
spec.loader.exec_module(html)


def config():
    return {"llm": {"stages": {"html_to_markdown": {"model": "gpt-6.1-sol", "reasoning_effort": "medium"}}}}


class FakeTransport:
    def __init__(self, text="---\ntitle: Sample\n---\n# Sample\n", status="completed", error=None):
        self.text, self.status, self.error = text, status, error
        self.calls = []

    def validate(self, selected, *, images=False):
        self.selected = selected

    def run(self, selected, prompt, **options):
        self.calls.append((selected, prompt, options))
        if self.error:
            raise self.error
        return {"text": self.text, "status": self.status, "usage": {"inputTokens": 12}}

    def close(self):
        pass


class ConfigTests(unittest.TestCase):
    def test_duplicate_missing_and_malformed_yaml(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "config.yaml"
            for content in ("llm: {}\nllm: {}", "llm: {}", "llm: [", "[]",
                            "llm:\n  stages:\n    html_to_markdown: {}\n    html_to_markdown: {}"):
                with self.subTest(content=content):
                    path.write_text(content)
                    with self.assertRaises(Exception):
                        load_config(path)

    def test_invalid_timeout_and_concurrency(self):
        for field, value in (("timeout_seconds", -1), ("timeout_seconds", float("nan")), ("timeout_seconds", True), ("concurrency", 8)):
            value_config = config()
            value_config["llm"][field] = value
            with self.assertRaises(ValueError):
                validate_config(value_config, HTML_STAGES, HTML_STAGES)

    def test_pdf_configuration_is_parsed_without_html_validation(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "config.yaml"
            path.write_text("render: {dpi: 72}\nllm: {}\nllm: {extra: retained}\n")
            self.assertEqual(load_pdf_config(path)["llm"], {"extra": "retained"})

    def test_runtime_capability_validation(self):
        transport = HtmlTransport.__new__(HtmlTransport)
        transport.models = [{"model": "gpt-6.1-sol", "supportedReasoningEfforts": [{"reasoningEffort": "medium"}], "inputModalities": ["text"]}]
        selected = selection(config(), "html_to_markdown")
        transport.validate(selected)
        with self.assertRaises(ValueError):
            transport.validate(selected, images=True)
        for field in ("model", "reasoning_effort"):
            value_config = config()
            value_config["llm"]["stages"]["html_to_markdown"][field] = "unsupported"
            with self.assertRaises(ValueError):
                transport.validate(selection(value_config, "html_to_markdown"))


class ClientTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cache = ResponseCache(Path(self.tmp.name) / "codex")

    def client(self, transport):
        return HtmlClient(config(), transport=transport, cache=self.cache)

    def test_json_requests_share_bound_selection(self):
        transport = FakeTransport()
        client = CodexClient(config(), stage="html_to_markdown", transport=transport, cache=self.cache, operation="html_to_markdown")
        image = Path(self.tmp.name) / "image.png"
        image.write_bytes(b"sample")
        transport.text = '{"signal":"CCK"}'
        self.assertEqual(client.generate_json("json"), {"signal": "CCK"})
        self.assertEqual({call[0] for call in transport.calls}, {client.selected})
        self.assertNotIn("schema", transport.calls[-1][2])
        self.assertEqual(client.generate_json("prompt-only JSON", image_path=image), {"signal": "CCK"})
        self.assertNotIn("schema", transport.calls[-1][2])

    def test_html_request_contains_only_prompt_and_source(self):
        transport = FakeTransport()
        prompt_file = ROOT / 'tools/bootstrap/html-to-markdown/references/llm-transcription-prompt.md'
        source = '<h1>Sample</h1><p>Body</p>'
        html.convert_with_llm(source, 'Sample', prompt_file, self.client(transport))
        self.assertEqual(transport.calls[0][1], prompt_file.read_text(encoding='utf-8') +
                         '\n\nDocument Title: Sample\n\nHTML Content:\n```html\n' + source + '\n```')
        self.assertNotIn('schema', transport.calls[0][2])

    def test_json_application_validator_runs_before_caching_and_on_reuse(self):
        def check(value):
            if value != {'signal': 'CCK'}:
                raise ValueError('Changed source content')
        transport = FakeTransport(text='{"signal":"wrong"}')
        with CodexClient(config(), stage='html_to_markdown', transport=transport,
                         cache=self.cache, operation='validated-json') as client:
            with self.assertRaisesRegex(ValueError, 'Changed source content'):
                client.generate_json('same request', validator=check)
            transport.text = '{"signal":"CCK"}'
            self.assertEqual(client.generate_json('same request', validator=check), {'signal': 'CCK'})
            self.assertEqual(len(transport.calls), 2)
        unused = FakeTransport(error=AssertionError('Should use validated cache'))
        with CodexClient(config(), stage='html_to_markdown', transport=unused,
                         cache=self.cache, operation='validated-json') as client:
            self.assertEqual(client.generate_json('same request', validator=check), {'signal': 'CCK'})
            def reject(value):
                raise ValueError('New guard rejects this response')
            with self.assertRaisesRegex(RuntimeError, 'Invalid cached conversion response'):
                client.generate_json('same request', validator=reject)
            self.assertEqual(unused.calls, [])

    def test_cache_hits_do_not_make_requests(self):
        image = Path(self.tmp.name) / "image.png"
        image.write_bytes(b"sample image")
        for method, arguments, response in (
            ("generate_text", {}, "Saved text"),
            ("generate_json", {"image_path": image}, '{"signal":"CCK"}'),
        ):
            with self.subTest(method=method):
                cache_directory = Path(self.tmp.name) / method
                transport = FakeTransport(text=response)
                with HtmlClient(config(), stage="html_to_markdown", transport=transport,
                                 cache=ResponseCache(cache_directory)) as first:
                    expected = getattr(first, method)("sample", **arguments)
                    self.assertEqual((first.call_count, first.cached_call_count), (1, 0))
                unused = FakeTransport(error=RuntimeError("Cache replay must not call the model"))
                with HtmlClient(config(), stage="html_to_markdown", transport=unused,
                                 cache=ResponseCache(cache_directory)) as replay:
                    self.assertEqual(getattr(replay, method)("sample", **arguments), expected)
                    self.assertEqual((replay.call_count, replay.cached_call_count), (0, 1))
                    self.assertTrue(replay.metrics[-1]["cached"])
                self.assertEqual(len(transport.calls), 1)
                self.assertEqual(unused.calls, [])

    def test_request_delay_starts_after_response_and_skips_cache(self):
        paced = config()
        paced["llm"]["post_request_delay_seconds"] = 60
        clock = [0.0]
        starts = []

        class SlowTransport(FakeTransport):
            def run(self, selected, prompt, **options):
                starts.append(clock[0])
                clock[0] += 90
                return super().run(selected, prompt, **options)

        transport = SlowTransport(text="Saved text")
        output = StringIO()
        with patch("conversion.client.time.monotonic", side_effect=lambda: clock[0]), \
                patch("conversion.client.time.sleep", side_effect=lambda seconds: clock.__setitem__(0, clock[0] + seconds)) as sleep, \
                patch.dict(os.environ, {"CONVERSION_TRACE": "1"}), redirect_stderr(output):
            with CodexClient(paced, stage="html_to_markdown", operation="paced",
                             transport=transport, cache=self.cache) as client:
                client._generate("first")
                client._generate("first")
                client._generate("second")
        self.assertEqual(len(transport.calls), 2)
        sleep.assert_called_once()
        self.assertEqual(sleep.call_args.args[0], 60)
        self.assertEqual(starts, [0, 150])
        self.assertEqual(output.getvalue().count("request.completed"), 2)
        self.assertNotIn("cache", output.getvalue())
        self.assertIn("request.wait", output.getvalue())
        self.assertNotIn("Saved text", output.getvalue())

    def test_operation_cache_survives_stage_rename(self):
        original = config()
        renamed = {"llm": {"stages": {"renamed_stage": original["llm"]["stages"]["html_to_markdown"]}}}
        with CodexClient(original, stage="html_to_markdown", operation="html_to_markdown",
                         transport=FakeTransport(text="Cached response"), cache=self.cache) as first:
            first._generate("same request")
        unused = FakeTransport(error=AssertionError("Stage rename must reuse the cache"))
        with CodexClient(renamed, stage="renamed_stage", operation="html_to_markdown",
                         transport=unused, cache=self.cache) as replay:
            self.assertEqual(replay._generate("same request"), "Cached response")
            self.assertEqual((replay.call_count, replay.cached_call_count), (0, 1))
            self.assertEqual(replay.metrics[-1]["stage"], "renamed_stage")
        self.assertEqual(unused.calls, [])

    def test_request_changes_make_new_calls_and_cache_the_new_response(self):
        images = [Path(self.tmp.name) / name for name in ("a.png", "b.png")]
        for changed_field in ("prompt", "model", "reasoning_effort", "operation", "image_contents", "image_order"):
            with self.subTest(changed_field=changed_field):
                for index, image in enumerate(images):
                    image.write_bytes(str(index).encode())
                cache = ResponseCache(Path(self.tmp.name) / changed_field)
                with CodexClient(config(), stage="html_to_markdown", transport=FakeTransport(text='{"signal":"old"}'), cache=cache, operation="html_to_markdown") as first:
                    self.assertEqual(first.generate_json("sample", image_path=images), {"signal": "old"})
                changed = config()
                stage, operation, prompt, ordered_images = "html_to_markdown", "html_to_markdown", "sample", images
                if changed_field == "prompt":
                    prompt = "changed prompt"
                elif changed_field in ("model", "reasoning_effort"):
                    changed["llm"]["stages"][stage][changed_field] = "other-model" if changed_field == "model" else "high"
                elif changed_field == "operation":
                    operation = "different_operation"
                elif changed_field == "image_contents":
                    images[0].write_bytes(b"changed image at the same path")
                elif changed_field == "image_order":
                    ordered_images = images[::-1]
                transport = FakeTransport(text='{"signal":"new"}')
                with CodexClient(changed, stage=stage, transport=transport, cache=cache, operation=operation) as client:
                    self.assertEqual(client.generate_json(prompt, image_path=ordered_images), {"signal": "new"})
                    self.assertEqual((client.call_count, client.cached_call_count), (1, 0))
                self.assertEqual(len(transport.calls), 1)
                unused = FakeTransport(error=RuntimeError("Changed request replay must use cache"))
                with CodexClient(changed, stage=stage, transport=unused, cache=cache, operation=operation) as replay:
                    self.assertEqual(replay.generate_json(prompt, image_path=ordered_images), {"signal": "new"})
                    self.assertEqual((replay.call_count, replay.cached_call_count), (0, 1))
                self.assertEqual(unused.calls, [])

    def test_unreadable_incomplete_and_mismatched_cache_entries_are_refetched(self):
        request = identity(selection(config(), "html_to_markdown"), "sample", [], operation="html_to_markdown")
        valid = {"identity": request, "status": "completed", "text": "Old response"}
        mismatch = {**request, "engine": "legacy-engine"}
        bodies = {
            "invalid_json": "truncated {",
            "non_object": "[]",
            "incomplete": json.dumps({**valid, "status": "interrupted"}),
            "mismatched_identity": json.dumps({**valid, "identity": mismatch}),
        }
        self.cache.directory.mkdir()
        path = self.cache.directory / f"{digest(request)}.json"
        for case, body in bodies.items():
            with self.subTest(case=case):
                path.write_text(body, encoding="utf-8")
                transport = FakeTransport(text="Fresh response")
                with self.client(transport) as client:
                    self.assertEqual(client.generate_text("sample"), "Fresh response")
                    self.assertEqual((client.call_count, client.cached_call_count), (1, 0))
                self.assertEqual(len(transport.calls), 1)
                self.assertEqual(self.cache.load(request)["text"], "Fresh response")

    def test_cached_json_is_parsed_before_reuse_without_model_calls(self):
        request = identity(selection(config(), "html_to_markdown"), "sample", [], operation="html_to_markdown")
        for response in ("truncated {", ""):
            with self.subTest(response=response):
                self.cache.save(request, {"status": "completed", "text": response})
                transport = FakeTransport()
                with self.client(transport) as client:
                    with self.assertRaisesRegex(RuntimeError, "Invalid cached conversion response"):
                        client.generate_json("sample")
                    self.assertEqual((client.call_count, client.cached_call_count), (0, 0))
                self.assertEqual(transport.calls, [])
                self.assertEqual(self.cache.load(request)["text"], response)

    def test_environment_cache_directory_and_explicit_cache_are_isolated(self):
        environment_directory = Path(self.tmp.name) / "environment-cache"
        explicit_directory = Path(self.tmp.name) / "explicit-cache"
        with patch.dict(os.environ, {"CONVERSION_CACHE_DIR": str(environment_directory)}):
            with HtmlClient(config(), transport=FakeTransport(text="Environment response")) as client:
                self.assertEqual(client.generate_text("sample"), "Environment response")
                self.assertEqual(client.cache.directory, environment_directory)
            with HtmlClient(config(), transport=FakeTransport(text="Explicit response"),
                             cache=ResponseCache(explicit_directory)) as client:
                self.assertEqual(client.generate_text("sample"), "Explicit response")
                self.assertEqual(client.cache.directory, explicit_directory)
            unused = FakeTransport(error=RuntimeError("Environment cache replay must not call the model"))
            with HtmlClient(config(), transport=unused) as replay:
                self.assertEqual(replay.generate_text("sample"), "Environment response")
                self.assertEqual((replay.call_count, replay.cached_call_count), (0, 1))
            self.assertEqual(unused.calls, [])

    def test_cache_identity_changes_with_effort_images_and_order(self):
        images = [Path(self.tmp.name) / name for name in ("a.png", "b.png")]
        for index, path in enumerate(images):
            path.write_bytes(str(index).encode())
        selected = selection(config(), "html_to_markdown")
        first = identity(selected, "sample", images, operation="html_to_markdown")
        variants = [identity(selected, "other", images, operation="html_to_markdown"),
                    identity(selected, "sample", images[::-1], operation="html_to_markdown")]
        changed = config()
        changed["llm"]["stages"]["html_to_markdown"]["reasoning_effort"] = "high"
        variants.append(identity(selection(changed, "html_to_markdown"), "sample", images, operation="html_to_markdown"))
        for field in ("BASE_INSTRUCTIONS", "DEVELOPER_INSTRUCTIONS", "SDK_VERSION", "CONTRACT_VERSION"):
            with patch(f"conversion.cache.{field}", "changed cache contract"):
                variants.append(identity(selected, "sample", images, operation="html_to_markdown"))
        images[0].write_bytes(b"changed")
        variants.append(identity(selected, "sample", images, operation="html_to_markdown"))
        self.assertEqual(len({digest(first), *(digest(v) for v in variants)}), len(variants) + 1)
        self.assertEqual(first["engine"], "codex-chatgpt")

    def test_failed_interrupted_empty_and_invalid_json_never_cached(self):
        for transport in (FakeTransport(status="failed"), FakeTransport(status="interrupted"), FakeTransport(text=""),
                          FakeTransport(text="truncated {")):
            with self.subTest(text=transport.text, status=transport.status), self.assertRaises(Exception):
                self.client(transport).generate_json("JSON response")
        self.assertFalse(self.cache.directory.exists())

    def test_auth_quota_timeout_propagate_without_retries(self):
        for error in (RuntimeError("Authentication required"), RuntimeError("Quota exceeded"), TimeoutError("Deadline exceeded")):
            transport = FakeTransport(error=error)
            with self.subTest(error=error), self.assertRaises(type(error)):
                self.client(transport).generate_text("failure")
            self.assertEqual(len(transport.calls), 1)
        self.assertFalse(self.cache.directory.exists())

    def test_invalid_json_reports_complete_response_and_request_context(self):
        response = '{"format" "markdown", "md_text": "broken"}\n'
        image = Path(self.tmp.name) / "table.png"
        image.write_bytes(b"image fixture")
        with self.assertRaises(json.JSONDecodeError) as caught:
            self.client(FakeTransport(text=response)).generate_json("table", image_path=image)
        diagnostic = json.loads(caught.exception.__notes__[0])
        self.assertEqual(diagnostic["response"], response)
        self.assertEqual(diagnostic["input_images"], [str(image.resolve())])
        self.assertIn("stage", diagnostic)
        self.assertIn("operation", diagnostic)
        self.assertFalse(self.cache.directory.exists())

    def test_pdf_json_uses_prompt_without_local_schema_enforcement(self):
        pdf_config = {"llm": {"extra": True, "stages": {"03_page_conversion": {"model": "fixture", "reasoning_effort": "medium", "extra": "retained"}}}}
        transport = FakeTransport(text='{"segments": "unvalidated"}')
        transport.validate = lambda *args, **kwargs: self.fail("PDF model configuration must not be locally validated")
        with CodexClient(pdf_config, stage="03_page_conversion", transport=transport, cache=self.cache, operation="pdf_page_to_objects") as client:
            result = client.generate_json("PDF")
            self.assertEqual(result, {"segments": "unvalidated"})
            client.generate_json("PDF")
            self.assertEqual(len(transport.calls), 1)

    def test_markdown_validation_runs_before_cache_write(self):
        with self.assertRaises(ValueError):
            self.client(FakeTransport(text="partial prose")).generate_text("invalid", validator=html.validate_markdown)
        self.assertFalse(self.cache.directory.exists())

    def test_frontmatter_delimiters_are_complete_lines(self):
        html.validate_markdown('---\ntitle: "Signal---timing"\n---\n# Signal timing\n')
        html.validate_markdown('---\r\ntitle: Signal\r\n---\r\n# Signal\r\n')

    def test_single_and_crawl_failures_do_not_emit_dom_fallback(self):
        source = Path(self.tmp.name) / "source"
        source.mkdir()
        (source / "index.html").write_text("<h1>Sample</h1><p>Body</p>")
        output = Path(self.tmp.name) / "output"
        output.mkdir()
        client = self.client(FakeTransport(error=RuntimeError("quota")))
        with patch.object(html, "extract_assets"):
            for function, input_path in ((html.convert_single_html, source / "index.html"), (html.convert_crawl_directory, source)):
                with self.assertRaisesRegex(RuntimeError, "quota"):
                    function(input_path, output, "Sample", client)
        self.assertFalse(list(output.glob("*.md")))


class WireClient:
    def __init__(self, *, status="completed", item_type="agentMessage", blocked=False):
        self.threads = []
        self.turns = []
        self.status, self.item_type, self.blocked = status, item_type, blocked
        self.closed = threading.Event()

    def thread_start(self, params):
        self.threads.append(params)
        return SimpleNamespace(thread=SimpleNamespace(id=str(len(self.threads))), model=params["model"],
                               reasoning_effort=SimpleNamespace(value=params["config"]["model_reasoning_effort"]),
                               instruction_sources=[], model_provider="openai", sandbox=SimpleNamespace(root=SimpleNamespace(type="readOnly")))

    def turn_start(self, thread_id, items, params):
        self.turns.append((thread_id, items, params))
        self.step = 0
        return SimpleNamespace(turn=SimpleNamespace(id="turn"))

    def next_turn_notification(self, turn_id):
        if self.blocked:
            self.closed.wait(2)
            raise RuntimeError("Transport closed")
        self.step += 1
        if self.step == 1:
            method = "item/completed"
            payload = {"item": {"type": self.item_type, "phase": "final_answer", "text": "complete"}}
        else:
            method = "turn/completed"
            payload = {"turn": {"status": self.status, "error": None}}
        return SimpleNamespace(method=method, payload=SimpleNamespace(model_dump=lambda **_: payload))

    def close(self):
        self.closed.set()


class TransportTests(unittest.TestCase):
    def transport(self, wire):
        transport = HtmlTransport.__new__(HtmlTransport)
        transport._client = wire
        transport._directory = SimpleNamespace(name="isolated-cwd")
        transport.models = [{"model":"gpt-6.1-sol", "supportedReasoningEfforts":[{"reasoningEffort":"medium"}], "inputModalities":["text","image"]}]
        return transport

    def test_fresh_thread_original_image_and_explicit_effort(self):
        wire = WireClient()
        original_notification = wire.next_turn_notification
        notifications = [SimpleNamespace(method="item/agentMessage/delta",
                                         payload=SimpleNamespace(model_dump=lambda **_: {"delta": "private response"}))]
        wire.next_turn_notification = lambda turn_id: (
            notifications.pop() if notifications else original_notification(turn_id))
        transport = self.transport(wire)
        selected = selection(config(), "html_to_markdown")
        output = StringIO()
        with patch.dict(os.environ, {"CONVERSION_TRACE": "1"}), redirect_stderr(output):
            transport.run(selected, "private prompt", images=[Path("image.png")])
        trace = output.getvalue()
        self.assertIn("request.sent", trace)
        self.assertNotIn("turn.event", trace)
        self.assertNotIn("delta", trace)
        self.assertNotIn("thread.started", trace)
        self.assertNotIn("turn.finished", trace)
        self.assertNotIn("private prompt", trace)
        self.assertNotIn("image.png", trace)
        transport.run(selected, "two")
        self.assertEqual(len(wire.threads), 2)
        self.assertTrue(all(p["sandbox"] == "read-only" and p["ephemeral"] for p in wire.threads))
        self.assertTrue(all(p['baseInstructions'] == '' and p['developerInstructions'] == '' for p in wire.threads))
        self.assertEqual(wire.turns[0][1], [{'type': 'text', 'text': 'private prompt'},
            {'type': 'localImage', 'path': str(Path('image.png').resolve()), 'detail': 'original'}])
        self.assertEqual(wire.turns[1][1], [{'type': 'text', 'text': 'two'}])
        self.assertTrue(all(set(turn[2]) == {'model', 'effort'} for turn in wire.turns))
        self.assertEqual(wire.turns[0][1][1]["detail"], "original")
        self.assertEqual(wire.turns[0][2]["effort"], "medium")
        pdf_config = {"llm": {"stages": {"03_page_conversion": {"model": "gpt-6.1-sol", "reasoning_effort": "medium"}}}}
        transport.models = []
        CodexTransport.run(transport, selection(pdf_config, "03_page_conversion"), "PDF without catalog precheck")
        self.assertEqual(len(wire.threads), 3)
        self.assertEqual(wire.threads[-1]['baseInstructions'], '')
        self.assertEqual(wire.threads[-1]['developerInstructions'], '')
        self.assertEqual(wire.turns[-1][1], [{'type': 'text', 'text': 'PDF without catalog precheck'}])
        self.assertEqual(set(wire.turns[-1][2]), {'model', 'effort'})

    def test_timeout_closes_transport_and_rejects_completion(self):
        wire = WireClient(blocked=True)
        with self.assertRaises(TimeoutError):
            self.transport(wire).run(selection(config(), "html_to_markdown"), "timeout", timeout=0.02)
        self.assertTrue(wire.closed.is_set())

    def test_interrupted_and_tool_results_are_rejected(self):
        for wire in (WireClient(status="interrupted"), WireClient(item_type="commandExecution")):
            with self.assertRaises(RuntimeError):
                self.transport(wire).run(selection(config(), "html_to_markdown"), "reject")

    def test_unexpected_image_view_reports_path_and_request_context(self):
        wire = WireClient()
        payload = {"item": {"type": "imageView", "id": "image-item", "path": "requested.png"}}
        wire.next_turn_notification = lambda _: SimpleNamespace(
            method="item/completed", payload=SimpleNamespace(model_dump=lambda **_: payload))
        with self.assertRaises(RuntimeError) as caught:
            self.transport(wire).run(selection(config(), "html_to_markdown"), "inspect", images=[Path("input.png")])
        for detail in ("requested.png", "image-item", "input.png", "thread_id", "turn_id"):
            self.assertIn(detail, str(caught.exception))

    def test_image_view_of_attached_image_keeps_waiting_for_final_answer(self):
        wire = WireClient()
        original = wire.next_turn_notification
        payload = {"item": {"type": "imageView", "id": "image-item",
                            "path": str(Path("input.png").resolve())}}
        events = [SimpleNamespace(method="item/completed",
                                  payload=SimpleNamespace(model_dump=lambda **_: payload))]
        wire.next_turn_notification = lambda turn_id: events.pop() if events else original(turn_id)
        result = self.transport(wire).run(selection(config(), "html_to_markdown"),
                                          "inspect", images=[Path("input.png")])
        self.assertEqual(result["text"], "complete")
        self.assertEqual(result["status"], "completed")

    def test_image_view_without_attached_image_or_path_is_rejected(self):
        for path, images in ((str(Path("input.png").resolve()), []), (None, [Path("input.png")])):
            wire = WireClient()
            payload = {"item": {"type": "imageView", "path": path}}
            wire.next_turn_notification = lambda _: SimpleNamespace(
                method="item/completed", payload=SimpleNamespace(model_dump=lambda **_: payload))
            with self.subTest(path=path), self.assertRaises(RuntimeError):
                self.transport(wire).run(selection(config(), "html_to_markdown"), "inspect", images=images)

    def test_capacity_notification_aborts_runtime_retry(self):
        for info in ("rateLimitExceeded", "usageLimitExceeded", "serverOverloaded",
                     {"httpConnectionFailed": {"httpStatusCode": 429}}):
            wire = WireClient()
            payload = {"error": {"message": "Capacity exhausted", "codexErrorInfo": info},
                       "willRetry": True}
            wire.next_turn_notification = lambda _: SimpleNamespace(
                method="error", payload=SimpleNamespace(model_dump=lambda **_: payload))
            with self.subTest(info=info), self.assertRaisesRegex(RuntimeError, "without retries"):
                self.transport(wire).run(selection(config(), "html_to_markdown"), "stop")
            self.assertTrue(wire.closed.is_set())


class ParallelConversionTests(unittest.TestCase):
    def test_worker_clients_are_isolated_and_accept_runs_on_coordinator(self):
        barrier = threading.Barrier(2)
        coordinator = threading.get_ident()
        clients, accepted = [], []

        class Client:
            def __enter__(self):
                self.owner = threading.get_ident()
                clients.append(self)
                return self

            def __exit__(self, *_):
                self.closed = True

        def convert(client, job):
            self.assertEqual(threading.get_ident(), client.owner)
            barrier.wait(timeout=3)
            return job * 2

        def accept(job, value):
            self.assertEqual(threading.get_ident(), coordinator)
            accepted.append((job, value))

        convert_parallel({"llm": {"concurrency": 2}}, stage="stage", operation="operation",
                         jobs=range(4), convert=convert, accept=accept,
                         client_factory=lambda *_, **__: Client())
        self.assertEqual(sorted(accepted), [(0, 0), (1, 2), (2, 4), (3, 6)])
        self.assertEqual(len(clients), 2)
        self.assertTrue(all(client.closed for client in clients))

    def test_failure_stops_pending_jobs_without_retry(self):
        calls = []

        class Client:
            def __enter__(self):
                return self

            def __exit__(self, *_):
                pass

        def convert(client, job):
            calls.append(job)
            raise RuntimeError("rate limit")

        with self.assertRaisesRegex(RuntimeError, "rate limit"):
            convert_parallel({"llm": {"concurrency": 1}}, stage="stage", operation="operation",
                             jobs=range(5), convert=convert, accept=lambda *_: self.fail("No result"),
                             client_factory=lambda *_, **__: Client())
        self.assertEqual(calls, [0])

    def test_concurrent_metrics_preserve_all_calls(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "metrics.json"
            with patch.dict(os.environ, {"LLM_STAGE_METRICS_FILE": str(path)}):
                with ThreadPoolExecutor(max_workers=8) as workers:
                    list(workers.map(lambda index: record({"cached": index % 2 == 0,
                                                         "duration_seconds": 1}), range(80)))
            metrics = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(len(metrics["requests"]), 80)
            self.assertEqual(metrics["llm_calls"], 40)
            self.assertEqual(metrics["llm_cached_calls"], 40)
            self.assertEqual(metrics["llm_time_seconds"], 40)


if __name__ == "__main__":
    unittest.main()
