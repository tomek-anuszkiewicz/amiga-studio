"""Verify project MCP discovery in Codex without a model turn or RAG mutation."""

import importlib.util
import json
from pathlib import Path
import queue
import shutil
import subprocess
import tempfile
import threading
import time
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
CODEX_EXECUTABLE = shutil.which("codex")
HAS_FASTMCP = importlib.util.find_spec("fastmcp") is not None


@unittest.skipUnless(CODEX_EXECUTABLE and HAS_FASTMCP, "Install Codex and the MCP server requirements first")
class CodexMcpLaunchTests(unittest.TestCase):
    def test_project_session_discovers_tools_from_unrelated_host_directory(self):
        messages = queue.Queue()
        with tempfile.TemporaryDirectory() as host_directory:
            process = subprocess.Popen(
                [CODEX_EXECUTABLE, "app-server", "--stdio"], cwd=host_directory,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                text=True, encoding="utf-8", errors="replace",
            )

            def collect_messages():
                for line in process.stdout:
                    messages.put(json.loads(line))

            reader = threading.Thread(target=collect_messages, daemon=True)
            reader.start()
            request_id = 0

            def send(message):
                process.stdin.write(json.dumps(message) + "\n")
                process.stdin.flush()

            def rpc(method, params):
                nonlocal request_id
                request_id += 1
                current_id = request_id
                send({"id": current_id, "method": method, "params": params})
                deadline = time.monotonic() + 30
                while time.monotonic() < deadline:
                    message = messages.get(timeout=max(0.01, deadline - time.monotonic()))
                    if message.get("id") == current_id and "method" not in message:
                        self.assertNotIn("error", message, message)
                        return message["result"]
                    self.assertFalse("id" in message and "method" in message, message)
                self.fail(f"Codex did not respond to {method}")

            try:
                rpc("initialize", {
                    "clientInfo": {"name": "rag_launch_test", "version": "1.0"},
                    "capabilities": {"experimentalApi": True},
                })
                send({"method": "initialized"})
                thread = rpc("thread/start", {"cwd": str(REPOSITORY_ROOT), "ephemeral": True})
                status = rpc("mcpServerStatus/list", {
                    "threadId": thread["thread"]["id"], "serverName": "amiga-rag",
                })
                self.assertEqual(len(status["data"]), 1, status)
                server = status["data"][0]
                self.assertEqual(server["runtimeStatus"], "connected", server)
                self.assertIsNone(server["toolsError"], server)
                self.assertEqual(set(server["tools"]), {"rag_search", "rag_list_sources", "rag_status"})
            finally:
                process.stdin.close()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    process.wait(timeout=5)
                reader.join(timeout=2)
                process.stdout.close()


if __name__ == "__main__":
    unittest.main()
