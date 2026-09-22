"""Small, dependency-free loader for the MCP server's local environment file."""

import os
from pathlib import Path


def load_environment_file(environment_file: Path) -> None:
    """Load simple ``NAME=VALUE`` entries without overriding process settings."""
    if not environment_file.is_file():
        return

    for raw_line in environment_file.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        name, value = line.split("=", 1)
        name = name.strip()
        value = value.strip()
        if not name.isidentifier() or not value:
            continue
        if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
            value = value[1:-1]
        os.environ.setdefault(name, value)
