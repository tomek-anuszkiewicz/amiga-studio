"""Content-addressed PDF preparation bundles with binary artifact files."""

import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import tempfile

import pymupdf

from conversion.cache import digest


DEFAULT_CACHE = Path(__file__).resolve().parents[2] / ".cache/pdf"


def file_digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def runtime_identity():
    return {"pymupdf": pymupdf.VersionBind, "mupdf": pymupdf.VersionFitz,
            "python": platform.python_version(), "platform": platform.system(),
            "machine": platform.machine()}


class PreparationCache:
    def __init__(self, operation):
        self.directory = DEFAULT_CACHE / operation

    def restore(self, request, outputs):
        directory = self.directory / digest(request)
        try:
            entry = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
            if not isinstance(entry, dict) or entry.get("identity") != request or entry.get("status") != "completed":
                return False
            files = entry.get("files")
            if not isinstance(files, dict) or files.keys() != outputs.keys():
                return False
            if any(file_digest(directory / name) != checksum for name, checksum in files.items()):
                return False
        except (OSError, ValueError):
            return False
        for name, output in outputs.items():
            output = Path(output)
            output.parent.mkdir(parents=True, exist_ok=True)
            if not output.is_file() or file_digest(output) != files[name]:
                shutil.copyfile(directory / name, output)
        return True

    def save(self, request, outputs):
        directory = self.directory / digest(request)
        directory.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=self.directory, prefix=".pending-") as temporary:
            temporary = Path(temporary)
            files = {}
            for name, output in outputs.items():
                shutil.copyfile(output, temporary / name)
                files[name] = file_digest(temporary / name)
            manifest = {"identity": request, "status": "completed", "files": files}
            (temporary / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
            for name in (*outputs, "manifest.json"):
                os.replace(temporary / name, directory / name)
