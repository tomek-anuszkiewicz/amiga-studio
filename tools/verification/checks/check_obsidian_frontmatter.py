#!/usr/bin/env python3
"""Check local Obsidian Markdown, including Git-ignored reference books."""

from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]


def frontmatter_issue(path, book_file=False):
    lines = path.read_text(encoding="utf-8-sig").splitlines()
    if not lines or lines[0] != "---":
        return "missing opening YAML frontmatter marker on line 1"
    try:
        end = lines.index("---", 1)
    except ValueError:
        return "missing closing YAML frontmatter marker"
    properties = lines[1:end]
    if not any(line.startswith("title:") and line[6:].strip() for line in properties):
        return "missing title property"
    if not any(line.startswith("tags:") for line in properties):
        return "missing tags property"
    if book_file:
        if any(line.startswith("manual:") for line in properties):
            return "obsolete manual property; use book"
        for key in ("book", "chapter"):
            if not any(line.startswith(f"{key}:") and line[len(key) + 1:].strip()
                       for line in properties):
                return f"missing {key} property"
        tag_start = next(index for index, line in enumerate(properties) if line.startswith("tags:"))
        inline = properties[tag_start].split(":", 1)[1].strip()
        if inline.startswith("[") and inline.endswith("]"):
            tags = [tag.strip().strip("\"'") for tag in inline[1:-1].split(",")]
        else:
            tags = []
            for line in properties[tag_start + 1:]:
                if line and not line[0].isspace():
                    break
                if line.strip().startswith("- "):
                    tags.append(line.strip()[2:].strip("\"'"))
        for tag in ("reference", "book", "chapter"):
            if tag not in tags:
                return f"missing {tag} tag"
    return None


def check_vault(root):
    vault = root / "Obsidian" / "Amiga"
    if not vault.is_dir():
        return [(vault, "vault directory is missing")], 0
    files = sorted(vault.rglob("*.md"))
    reference = vault / "Reference"
    issues = [(path, issue) for path in files
              if (issue := frontmatter_issue(path, path.is_relative_to(reference)
                                             and path.parent != reference))]
    return issues, len(files)


def main():
    issues, count = check_vault(REPO_ROOT)
    for path, issue in issues:
        print(f"{path.relative_to(REPO_ROOT).as_posix()}: {issue}")
    if issues:
        print(f"[FAIL] {len(issues)} of {count} Obsidian Markdown files need frontmatter.")
        return 1
    print(f"[PASS] {count} Obsidian Markdown files have frontmatter.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
