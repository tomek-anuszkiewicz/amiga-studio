"""Offline conversion contracts: explicit selection, cache validity and failure propagation."""

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import threading
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools/bootstrap"))
from conversion import CodexClient, load_config
from conversion.cache import ResponseCache, digest, identity
from conversion.config import selection, validate_config, HTML_STAGES
from conversion.transport import CodexTransport

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

    def test_unknown_legacy_empty_and_deterministic_selections(self):
        cases = []
        for key, value in (("provider", "google-genai"), ("model_prose", "old"), ("default_thinking_budget", 512)):
            value_config = config()
            value_config["llm"][key] = value
            cases.append(value_config)
        for field, value in (("model", ""), ("reasoning_effort", None), ("extra", "medium")):
            value_config = config()
            value_config["llm"]["stages"]["html_to_markdown"][field] = value
            cases.append(value_config)
        value_config = config()
        value_config["llm"]["stages"]["03_build_raw_stream"] = {"model": "x", "reasoning_effort": "medium"}
        cases.append(value_config)
        for value_config in cases:
            with self.subTest(config=value_config), self.assertRaises(ValueError):
                validate_config(value_config, HTML_STAGES, HTML_STAGES)

    def test_invalid_timeout_and_concurrency(self):
        for field, value in (("timeout_seconds", -1), ("timeout_seconds", float("nan")), ("timeout_seconds", True), ("concurrency", 8)):
            value_config = config()
            value_config["llm"][field] = value
            with self.assertRaises(ValueError):
                validate_config(value_config, HTML_STAGES, HTML_STAGES)

    def test_runtime_capability_validation(self):
        transport = CodexTransport.__new__(CodexTransport)
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
        return CodexClient(config(), stage="html_to_markdown", transport=transport, cache=self.cache)

    def test_text_vision_json_share_bound_selection(self):
        transport = FakeTransport()
        client = self.client(transport)
        client.generate_text("text")
        image = Path(self.tmp.name) / "image.png"
        image.write_bytes(b"sample")
        client.generate_vision("vision", [image])
        transport.text = '{"signal":"CCK"}'
        schema = {"type": "object", "required": ["signal"], "properties": {"signal": {"type": "string"}}}
        self.assertEqual(client.generate_json("json", schema=schema), {"signal": "CCK"})
        self.assertEqual({call[0] for call in transport.calls}, {client.selected})
        self.assertEqual(transport.calls[-1][2]["schema"], schema)

    def test_cache_hits_do_not_make_requests(self):
        transport = FakeTransport()
        client = self.client(transport)
        client.generate_text("sample")
        client.generate_text("sample")
        self.assertEqual((client.call_count, client.cached_call_count), (1, 1))
        self.assertEqual(len(transport.calls), 1)

    def test_cache_identity_changes_with_effort_schema_images_and_order(self):
        images = [Path(self.tmp.name) / name for name in ("a.png", "b.png")]
        for index, path in enumerate(images):
            path.write_bytes(str(index).encode())
        selected = selection(config(), "html_to_markdown")
        first = identity(selected, "sample", images, None)
        variants = [identity(selected, "other", images, None), identity(selected, "sample", images[::-1], None),
                    identity(selected, "sample", images, {"type": "object"})]
        changed = config()
        changed["llm"]["stages"]["html_to_markdown"]["reasoning_effort"] = "high"
        variants.append(identity(selection(changed, "html_to_markdown"), "sample", images, None))
        images[0].write_bytes(b"changed")
        variants.append(identity(selected, "sample", images, None))
        self.assertEqual(len({digest(first), *(digest(v) for v in variants)}), 6)
        self.assertEqual(first["engine"], "codex-chatgpt")

    def test_failed_interrupted_empty_and_invalid_json_never_cached(self):
        for transport in (FakeTransport(status="failed"), FakeTransport(status="interrupted"), FakeTransport(text=""),
                          FakeTransport(text="truncated {"), FakeTransport(text='{"signal":12}')):
            with self.subTest(text=transport.text, status=transport.status), self.assertRaises(Exception):
                self.client(transport).generate_json("schema", schema={"type":"object", "properties":{"signal":{"type":"string"}}, "required":["signal"]})
        self.assertFalse(self.cache.directory.exists())

    def test_auth_quota_timeout_propagate_without_retries(self):
        for error in (RuntimeError("Authentication required"), RuntimeError("Quota exceeded"), TimeoutError("Deadline exceeded")):
            transport = FakeTransport(error=error)
            with self.subTest(error=error), self.assertRaises(type(error)):
                self.client(transport).generate_text("failure")
            self.assertEqual(len(transport.calls), 1)
        self.assertFalse(self.cache.directory.exists())

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
        transport = CodexTransport.__new__(CodexTransport)
        transport._client = wire
        transport._directory = SimpleNamespace(name="isolated-cwd")
        transport.models = [{"model":"gpt-6.1-sol", "supportedReasoningEfforts":[{"reasoningEffort":"medium"}], "inputModalities":["text","image"]}]
        return transport

    def test_fresh_thread_original_image_and_explicit_effort(self):
        wire = WireClient()
        transport = self.transport(wire)
        selected = selection(config(), "html_to_markdown")
        transport.run(selected, "one", images=[Path("image.png")])
        transport.run(selected, "two")
        self.assertEqual(len(wire.threads), 2)
        self.assertTrue(all(p["sandbox"] == "read-only" and p["ephemeral"] for p in wire.threads))
        self.assertEqual(wire.turns[0][1][1]["detail"], "original")
        self.assertEqual(wire.turns[0][2]["effort"], "medium")

    def test_timeout_closes_transport_and_rejects_completion(self):
        wire = WireClient(blocked=True)
        with self.assertRaises(TimeoutError):
            self.transport(wire).run(selection(config(), "html_to_markdown"), "timeout", timeout=0.02)
        self.assertTrue(wire.closed.is_set())

    def test_interrupted_and_tool_results_are_rejected(self):
        for wire in (WireClient(status="interrupted"), WireClient(item_type="commandExecution")):
            with self.assertRaises(RuntimeError):
                self.transport(wire).run(selection(config(), "html_to_markdown"), "reject")


if __name__ == "__main__":
    unittest.main()
