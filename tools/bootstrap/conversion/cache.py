"""Completed-response cache isolated from legacy Gemini entries."""

from dataclasses import asdict
from pathlib import Path
import hashlib
import json
import os
import tempfile

from .transport import BASE_INSTRUCTIONS, DEVELOPER_INSTRUCTIONS, OVERRIDES, SDK_VERSION

CONTRACT_VERSION = "1"
DEFAULT_CACHE = Path(__file__).resolve().parents[3] / "Obsidian/Amiga/Reference/.cache/codex"


def identity(selected, prompt, images, schema):
    return {"engine": "codex-chatgpt", "sdk_runtime": SDK_VERSION, "contract": CONTRACT_VERSION,
            **asdict(selected), "instructions": BASE_INSTRUCTIONS,
            "developer_instructions": DEVELOPER_INSTRUCTIONS, "execution": OVERRIDES,
            "prompt": prompt, "image_detail": "original", "schema": schema,
            "images": [hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in images]}


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode("utf-8")).hexdigest()


class ResponseCache:
    def __init__(self, directory=DEFAULT_CACHE):
        self.directory = Path(directory)

    def load(self, request):
        path = self.directory / f"{digest(request)}.json"
        try:
            entry = json.loads(path.read_text(encoding="utf-8"))
            if not isinstance(entry, dict):
                return None
            if entry.get("identity") != json.loads(json.dumps(request)) or entry.get("status") != "completed":
                return None
            return entry
        except (OSError, ValueError):
            return None

    def save(self, request, result):
        self.directory.mkdir(parents=True, exist_ok=True)
        entry = {**result, "identity": request}
        path = self.directory / f"{digest(request)}.json"
        handle, temporary = tempfile.mkstemp(dir=self.directory, suffix=".tmp")
        try:
            with os.fdopen(handle, "w", encoding="utf-8") as stream:
                json.dump(entry, stream, ensure_ascii=False)
            os.replace(temporary, path)
        finally:
            Path(temporary).unlink(missing_ok=True)
