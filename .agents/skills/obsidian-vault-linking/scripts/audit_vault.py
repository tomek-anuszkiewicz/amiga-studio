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

sys.path.insert(0, str(REPO_ROOT / "tests/checks"))
from docs_metadata import parse_markdown_frontmatter

def check_vault_links():
    """Validates markdown links in Obsidian/Amiga/Design/ for broken targets."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    if not design_dir.exists():
        return {"total_docs": 0, "total_links": 0, "broken_links": []}

    md_files = sorted(design_dir.glob("*.md"))
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
            target = (design_dir / decoded).resolve()

            if not target.exists():
                broken_links.append({
                    "file": doc.name,
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

    md_files = sorted(design_dir.glob("*.md"))
    issues = []

    for doc in md_files:
        fm, text = parse_markdown_frontmatter(doc)
        if not fm:
            issues.append({
                "file": doc.name,
                "message": f"Document `{doc.name}` lacks YAML frontmatter (`---`). Line 1 properties are required.",
            })
            continue

        if "tags" not in fm or not fm["tags"]:
            issues.append({
                "file": doc.name,
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
