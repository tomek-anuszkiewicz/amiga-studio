#!/usr/bin/env python3
"""Deterministic checks owned by obsidian-vault-linking."""

import argparse
import re
import sys
import urllib.parse
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

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


def check_vault_links():
    """Validates markdown links in Obsidian/Amiga/Design/ for broken targets."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    if not design_dir.exists():
        return {"total_docs": 0, "total_links": 0, "broken_links": []}

    md_files = sorted(design_dir.rglob("*.md"))
    total_links = 0
    broken_links = []

    link_pattern = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")

    for doc in md_files:
        try:
            content = doc.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        for match in link_pattern.finditer(content):
            link = match.group(2).strip()
            if link.startswith("http://") or link.startswith("https://") or link.startswith("mailto:"):
                continue

            path_part = link.split("#")[0].strip()
            if not path_part:
                continue

            total_links += 1
            decoded = urllib.parse.unquote(path_part)
            target = (doc.parent / decoded).resolve()

            if not target.exists():
                broken_links.append({
                    "file": doc.relative_to(design_dir).as_posix(),
                    "link": link,
                    "target": str(target),
                })

    return {
        "total_docs": len(md_files),
        "total_links": total_links,
        "broken_links": broken_links,
    }


def check_frontmatter_compliance():
    """Validates that Obsidian design specifications have proper Line 1 frontmatter."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    if not design_dir.exists():
        return {"total_docs": 0, "issues": []}

    md_files = sorted(design_dir.rglob("*.md"))
    issues = []

    for doc in md_files:
        fm, text = parse_markdown_frontmatter(doc)
        if not fm:
            issues.append({
                "file": doc.relative_to(design_dir).as_posix(),
                "message": f"Document `{doc.name}` lacks YAML frontmatter (`---`). Line 1 properties are required.",
            })
            continue

        if "tags" not in fm or not fm["tags"]:
            issues.append({
                "file": doc.relative_to(design_dir).as_posix(),
                "message": f"Document `{doc.name}` is missing `tags` property in frontmatter (e.g. `tags: [spec, ...]`).",
            })

    return {
        "total_docs": len(md_files),
        "issues": issues,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--vault-links", action="store_true", help="Check design-document link targets")
    parser.add_argument("--frontmatter", action="store_true", help="Check design-document frontmatter")
    args = parser.parse_args()
    run_all = args.all or not any([args.vault_links, args.frontmatter])
    total_issues = 0
    if run_all or args.vault_links:
        print("\n[2. OBSIDIAN VAULT LINKING & GRAPH INTEGRITY]")
        links_res = check_vault_links()
        broken = links_res["broken_links"]
        print(f"  - Inspected Documents: {links_res['total_docs']}")
        print(f"  - Total Links Verified: {links_res['total_links']}")
        if broken:
            total_issues += len(broken)
            print(f"  - Status: [FAIL] Found {len(broken)} broken link(s):")
            for b in broken[:10]:
                print(f"    * {b['file']}: `{b['link']}` -> target missing")
            if len(broken) > 10:
                print(f"    ... and {len(broken) - 10} more broken links.")
        else:
            print("  - Status: [PASS] 100% link integrity (zero broken links detected).")

    if run_all or args.frontmatter:
        print("\n[7. FRONTMATTER & INVERTED PYRAMID COMPLIANCE]")
        fm_res = check_frontmatter_compliance()
        fm_issues = fm_res["issues"]
        print(f"  - Inspected Design Specs: {fm_res['total_docs']}")
        if fm_issues:
            total_issues += len(fm_issues)
            print(f"  - Status: [ANOMALY] {len(fm_issues)} frontmatter issue(s):")
            for fi in fm_issues[:10]:
                print(f"    * {fi['file']}: {fi['message']}")
            if len(fm_issues) > 10:
                print(f"    ... and {len(fm_issues) - 10} more frontmatter issues.")
        else:
            print("  - Status: [PASS] All design specs define valid YAML frontmatter properties.")

    print(f"[FAIL] {total_issues} issue(s) detected." if total_issues else "[PASS] Checks completed without issues.")
    return 1 if total_issues else 0


if __name__ == "__main__":
    sys.exit(main())
