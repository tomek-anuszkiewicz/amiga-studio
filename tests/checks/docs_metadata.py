"""Frontmatter reader for vault checks and tracked-path ownership."""

from pathlib import Path


def parse_markdown_frontmatter(file_path: Path):
    """Parses frontmatter key-values and list items between opening and closing ---."""
    text = file_path.read_text(encoding="utf-8", errors="ignore")
    if not text.startswith("---"):
        return {}, text
    parts = text.split("---", 2)
    if len(parts) < 3:
        return {}, text
    fm_text = parts[1]

    data = {}
    lines = fm_text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.startswith("subsystem:"):
            data["subsystem"] = line.split(":", 1)[1].strip().strip('"\'')
        elif line.startswith("tags:"):
            raw_tags = line.split(":", 1)[1].strip()
            if raw_tags.startswith("[") and raw_tags.endswith("]"):
                data["tags"] = [t.strip().strip('"\'') for t in raw_tags[1:-1].split(",") if t.strip()]
            else:
                data["tags"] = [raw_tags] if raw_tags else []
        elif line.startswith("tracked_paths:"):
            paths = []
            i += 1
            while i < len(lines) and (lines[i].startswith("  -") or lines[i].startswith("    -")):
                item = lines[i].split("-", 1)[1].strip().strip('"\'')
                paths.append(item)
                i += 1
            data["tracked_paths"] = paths
            continue
        i += 1

    return data, text
