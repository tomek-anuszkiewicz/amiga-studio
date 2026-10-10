"""Pinned app-server SDK transport, with isolated threads and explicit image detail."""

from importlib.metadata import version
from pathlib import Path
import tempfile
import threading
import time

SDK_VERSION = "0.160.1"
# Empty overrides suppress inherited/default conversion instructions.
BASE_INSTRUCTIONS = ""
DEVELOPER_INSTRUCTIONS = ""
OVERRIDES = (
    'model_provider="openai"', 'forced_login_method="chatgpt"',
    'project_doc_max_bytes=0', 'features.shell_tool=false',
    'features.multi_agent=false', 'features.apps=false',
    'features.plugins=false', 'features.remote_plugin=false',
    'features.memories=false', 'mcp_servers={}', 'web_search="disabled"',
)


class CodexTransport:
    def __init__(self):
        from openai_codex import CodexConfig
        from openai_codex.client import CodexClient
        from openai_codex.generated.v2_all import ConfigReadResponse

        if version("openai-codex") != SDK_VERSION or version("openai-codex-cli-bin") != SDK_VERSION:
            raise RuntimeError(f"Install conversion requirements: SDK/runtime must be {SDK_VERSION}")
        self._directory = tempfile.TemporaryDirectory(prefix="reference-conversion-")
        self._client = CodexClient(CodexConfig(cwd=self._directory.name, config_overrides=OVERRIDES))
        try:
            self._client.start()
            self._client.initialize()
            effective = self._client.request("config/read", {"includeLayers": False}, response_model=ConfigReadResponse).config.model_dump(mode="json")
            servers = effective.get("mcp_servers") or {}
            if servers:
                # Empty tables merge with user configuration; disable every inherited server explicitly.
                self._client.close()
                if any(not name or not all(char.isascii() and (char.isalnum() or char in "_-") for char in name) for name in servers):
                    raise RuntimeError("Inherited MCP server names cannot be safely disabled")
                disable = tuple(f"mcp_servers.{name}.enabled=false" for name in servers)
                self._client = CodexClient(CodexConfig(cwd=self._directory.name, config_overrides=OVERRIDES + disable))
                self._client.start()
                self._client.initialize()
                effective = self._client.request("config/read", {"includeLayers": False}, response_model=ConfigReadResponse).config.model_dump(mode="json")
            if any(server.get("enabled", True) for server in (effective.get("mcp_servers") or {}).values()):
                raise RuntimeError("Unable to disable inherited MCP tools")
            features = effective.get("features") or {}
            if any(features.get(name) is not False for name in ("shell_tool", "multi_agent", "apps", "plugins", "remote_plugin", "memories")):
                raise RuntimeError("Runtime did not disable unrelated tools and memory")
            if effective.get("web_search") != "disabled" or effective.get("project_doc_max_bytes") != 0:
                raise RuntimeError("Runtime did not isolate conversion context")
            account = self._client.account_read().account
            if account is None or type(account.root).__name__ != "ChatgptAccount":
                raise RuntimeError("Conversion requires Codex ChatGPT sign-in; API-key mode is not supported")
        except BaseException:
            self.close()
            raise

    def run(self, selected, prompt, *, images=(), timeout=180):
        started = time.monotonic()
        expired = threading.Event()

        def stop():
            expired.set()
            self._client.close()

        timer = threading.Timer(timeout, stop)
        timer.daemon = True
        timer.start()
        try:
            thread = self._client.thread_start({
                "model": selected.model, "cwd": self._directory.name, "ephemeral": True,
                "approvalPolicy": "never", "sandbox": "read-only",
                "config": {"model_reasoning_effort": selected.reasoning_effort},
                "baseInstructions": BASE_INSTRUCTIONS,
                "developerInstructions": DEVELOPER_INSTRUCTIONS,
            })
            if thread.model != selected.model or thread.reasoning_effort.value != selected.reasoning_effort:
                raise RuntimeError("Runtime did not accept the configured model/effort")
            if thread.instruction_sources or thread.model_provider != "openai":
                raise RuntimeError("Conversion thread inherited instructions or an unexpected provider")
            if thread.sandbox.root.type != "readOnly":
                raise RuntimeError("Conversion requires a read-only thread")
            items = [{"type": "text", "text": prompt}]
            items.extend({"type": "localImage", "path": str(Path(p).resolve()), "detail": "original"} for p in images)
            options = {"model": selected.model, "effort": selected.reasoning_effort}
            turn = self._client.turn_start(thread.thread.id, items, options)
            usage = None
            messages = []
            methods = set()
            while True:
                event = self._client.next_turn_notification(turn.turn.id)
                methods.add(event.method)
                payload = event.payload.model_dump(mode="json", by_alias=True) if hasattr(event.payload, "model_dump") else {}
                if event.method == "error":
                    error = payload.get("error") or {}
                    info = error.get("codexErrorInfo")
                    if isinstance(info, str):
                        capacity = info in {"rateLimitExceeded", "usageLimitExceeded",
                                            "sessionBudgetExceeded", "serverOverloaded"}
                    else:
                        capacity = isinstance(info, dict) and any(
                            isinstance(detail, dict) and detail.get("httpStatusCode") in {429, 503}
                            for detail in info.values())
                    if capacity:
                        # Stop runtime retries before admitting further conversion work.
                        self._client.close()
                        raise RuntimeError(f"Codex capacity limit; conversion stopped without retries: {error}")
                if event.method == "model/rerouted":
                    raise RuntimeError("Conversion model was rerouted; configured model is required")
                if event.method == "thread/tokenUsage/updated":
                    usage = payload["tokenUsage"]["last"]
                if event.method == "item/completed":
                    item = payload["item"]
                    if item["type"] not in {"userMessage", "agentMessage", "reasoning"}:
                        raise RuntimeError(f"Unexpected conversion tool/item: {item['type']}")
                    if item["type"] == "agentMessage" and item.get("phase") in (None, "final_answer"):
                        messages.append(item["text"])
                if event.method == "turn/completed":
                    result = payload["turn"]
                    if result["status"] != "completed" or result.get("error"):
                        raise RuntimeError(f"Codex conversion failed: {result.get('error') or result['status']}")
                    break
            text = messages[-1] if messages else None
            if not text or not text.strip():
                raise RuntimeError("Codex returned no complete final response")
            if expired.is_set() or time.monotonic() - started > timeout:
                raise TimeoutError(f"Codex conversion exceeded {timeout} seconds")
            return {"text": text, "status": "completed", "usage": usage,
                    "duration_seconds": time.monotonic() - started, "events": sorted(methods),
                    "execution": {"model": thread.model, "reasoning_effort": thread.reasoning_effort.value,
                                  "instruction_sources": [], "sandbox": "read-only", "image_detail": "original"}}
        except Exception as error:
            if expired.is_set():
                raise TimeoutError(f"Codex conversion exceeded {timeout} seconds") from error
            raise
        finally:
            timer.cancel()

    def close(self):
        self._client.close()
        self._directory.cleanup()
