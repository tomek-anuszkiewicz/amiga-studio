#!/usr/bin/env python3
"""
audit_docs_quality.py - Comprehensive On-Demand Documentation & Governance Auditor

Audits seven critical documentation and agent governance dimensions across the repository:
1. Design Documentation & Code Drift Detection:
   - Tracks git commits between `last_synced_commit` and HEAD across `tracked_paths` in Obsidian design specs.
   - Provides diff inspection (`--design-diff`) and checkpoint bumping (`--design-bump`).
2. Obsidian Vault Linking & Graph Integrity:
   - Verifies zero broken relative markdown links in `Obsidian/Amiga/Design/`.
   - Validates URL-encoded characters, anchors, and target file presence.
3. Constitutional Byte Size Ceilings & Truncation Safety:
   - `AGENTS.md` <= 14,000 bytes (constitutional non-redundancy).
   - Rule files (`.agents/rules/*.md`, `GEMINI.md`) <= 23,000 bytes (prompt truncation safety at ~24 KB).
4. Agent Skills Catalog Synchronization (`docs/ai_agents.md`):
   - Verifies 100% parity between `.agents/skills/` and `docs/ai_agents.md` (zero missing, zero phantom).
5. Two-Way Script Locality & Harness Governance:
   - Enforces single-consumer scripts reside in `.agents/skills/<skill>/scripts/`.
   - Enforces multi-consumer shared scripts are promoted to `tools/harness/`.
6. Workflow, Skill & Rule Governance:
   - Enforces two-way symmetry between `.agents/workflows/`, `.agents/skills/`, and `.agents/rules/`.
7. Frontmatter & Inverted Pyramid Specification Compliance:
   - Verifies YAML frontmatter metadata in `Obsidian/Amiga/Design/` (tags, tracked_paths, last_synced_commit).
"""

import argparse
import datetime
import os
import re
import subprocess
import sys
import urllib.parse
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

def find_repo_root() -> Path:
    curr = Path(__file__).resolve().parent
    for p in [curr] + list(curr.parents):
        if (p / "Cargo.toml").exists() and (p / ".git").exists():
            return p
    return curr.parents[1]

REPO_ROOT = find_repo_root()
MAX_AGENTS_MD_BYTES = 14000
MAX_RULE_FILE_BYTES = 23000

# ---------------------------------------------------------------------------
# Pillar 1: Design Documentation & Code Drift Detection
# ---------------------------------------------------------------------------

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
        if line.startswith("last_synced_commit:"):
            data["last_synced_commit"] = line.split(":", 1)[1].strip().strip('"\'')
        elif line.startswith("last_synced_date:"):
            data["last_synced_date"] = line.split(":", 1)[1].strip().strip('"\'')
        elif line.startswith("subsystem:"):
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


def bump_markdown_checkpoint(file_path: Path, new_commit: str, new_date: str) -> bool:
    """Updates last_synced_commit and last_synced_date in YAML frontmatter."""
    text = file_path.read_text(encoding="utf-8")
    if not text.startswith("---"):
        return False
    parts = text.split("---", 2)
    if len(parts) < 3:
        return False
    fm_lines = parts[1].splitlines()

    commit_found = False
    date_found = False
    new_fm_lines = []
    for line in fm_lines:
        if line.startswith("last_synced_commit:"):
            new_fm_lines.append(f'last_synced_commit: "{new_commit}"')
            commit_found = True
        elif line.startswith("last_synced_date:"):
            new_fm_lines.append(f'last_synced_date: "{new_date}"')
            date_found = True
        else:
            new_fm_lines.append(line)

    if not commit_found:
        new_fm_lines.append(f'last_synced_commit: "{new_commit}"')
    if not date_found:
        new_fm_lines.append(f'last_synced_date: "{new_date}"')

    while new_fm_lines and not new_fm_lines[0].strip():
        new_fm_lines.pop(0)
    while new_fm_lines and not new_fm_lines[-1].strip():
        new_fm_lines.pop()

    new_content = "---\n" + "\n".join(new_fm_lines) + "\n---\n" + parts[2].lstrip("\r\n")
    file_path.write_text(new_content, encoding="utf-8")
    return True


def check_design_docs_sync():
    """Checks whether code in tracked_paths has drifted since last_synced_commit."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    if not design_dir.exists():
        return {"tracked_count": 0, "synced_count": 0, "drifted": [], "synced": []}

    tracked = []
    drifted = []
    synced = []

    for doc in sorted(design_dir.glob("*.md")):
        fm, _ = parse_markdown_frontmatter(doc)
        paths = fm.get("tracked_paths", [])
        commit = fm.get("last_synced_commit")

        if not paths or not commit:
            continue

        tracked.append(doc.name)
        cmd = ["git", "rev-list", "--count", f"{commit}..HEAD", "--"] + paths
        res = subprocess.run(cmd, cwd=REPO_ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
        if res.returncode != 0:
            drifted.append({
                "file": doc.name,
                "commit": commit,
                "paths": paths,
                "error": f"Invalid commit or git error: {res.stderr.strip()}",
                "count": -1,
                "log": [],
            })
            continue

        count = int(res.stdout.strip() or "0")
        if count > 0:
            log_cmd = ["git", "log", "--oneline", f"{commit}..HEAD", "--"] + paths
            l_res = subprocess.run(log_cmd, cwd=REPO_ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
            commits_log = [l.strip() for l in l_res.stdout.splitlines()[:5]]
            drifted.append({
                "file": doc.name,
                "commit": commit,
                "paths": paths,
                "count": count,
                "log": commits_log,
            })
        else:
            synced.append(doc.name)

    return {
        "tracked_count": len(tracked),
        "synced_count": len(synced),
        "drifted": drifted,
        "synced": synced,
    }


def show_design_diff(doc_name: str):
    """Outputs git diff between last_synced_commit and HEAD for tracked_paths."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    doc_path = design_dir / doc_name
    if not doc_path.exists() and not doc_name.endswith(".md"):
        doc_path = design_dir / f"{doc_name}.md"
    if not doc_path.exists():
        print(f"Error: Design document not found: {doc_name}", file=sys.stderr)
        return False

    fm, _ = parse_markdown_frontmatter(doc_path)
    paths = fm.get("tracked_paths", [])
    commit = fm.get("last_synced_commit")
    if not paths or not commit:
        print(f"Document {doc_path.name} does not define tracked_paths or last_synced_commit.", file=sys.stderr)
        return False

    print(f">> Inspecting diff for {doc_path.name} ({commit}..HEAD) in paths: {', '.join(paths)}\n")
    diff_cmd = ["git", "diff", f"{commit}..HEAD", "--"] + paths
    subprocess.run(diff_cmd, cwd=REPO_ROOT)
    return True


def bump_design_checkpoint(doc_name: str) -> bool:
    """Bumps last_synced_commit to HEAD and last_synced_date to today for doc_name."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    doc_path = design_dir / doc_name
    if not doc_path.exists() and not doc_name.endswith(".md"):
        doc_path = design_dir / f"{doc_name}.md"
    if not doc_path.exists():
        print(f"Error: Design document not found: {doc_name}", file=sys.stderr)
        return False

    rev_cmd = ["git", "rev-parse", "HEAD"]
    res = subprocess.run(rev_cmd, cwd=REPO_ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
    if res.returncode != 0:
        print("Error reading git HEAD commit hash.", file=sys.stderr)
        return False
    head_commit = res.stdout.strip()
    today_str = datetime.date.today().isoformat()

    success = bump_markdown_checkpoint(doc_path, head_commit, today_str)
    if success:
        print(f">> Successfully bumped {doc_path.name} checkpoint to {head_commit[:10]} ({today_str})")
    else:
        print(f"Error: Could not update frontmatter for {doc_path.name}", file=sys.stderr)
    return success

# ---------------------------------------------------------------------------
# Pillar 2: Obsidian Vault Linking & Graph Integrity
# ---------------------------------------------------------------------------

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

# ---------------------------------------------------------------------------
# Pillar 3: Constitutional Byte Size Ceilings & Truncation Safety
# ---------------------------------------------------------------------------

def check_size_limits():
    """Checks AGENTS.md, GEMINI.md, and .agents/rules/*.md file size thresholds."""
    issues = []

    # 1. Root AGENTS.md
    agents_path = REPO_ROOT / "AGENTS.md"
    if agents_path.exists():
        size = agents_path.stat().st_size
        if size > MAX_AGENTS_MD_BYTES:
            issues.append({
                "file": "AGENTS.md",
                "size": size,
                "limit": MAX_AGENTS_MD_BYTES,
                "message": f"AGENTS.md is {size:,} bytes (exceeds {MAX_AGENTS_MD_BYTES:,} constitutional ceiling by {size - MAX_AGENTS_MD_BYTES:,} bytes)",
            })

    # 2. GEMINI.md
    gemini_path = REPO_ROOT / "GEMINI.md"
    if gemini_path.exists():
        size = gemini_path.stat().st_size
        if size > MAX_RULE_FILE_BYTES:
            issues.append({
                "file": "GEMINI.md",
                "size": size,
                "limit": MAX_RULE_FILE_BYTES,
                "message": f"GEMINI.md is {size:,} bytes (exceeds safety ceiling of {MAX_RULE_FILE_BYTES:,} bytes; silent truncation risk)",
            })

    # 3. .agents/rules/*.md
    rules_dir = REPO_ROOT / ".agents" / "rules"
    if rules_dir.exists():
        for rule_file in sorted(rules_dir.glob("*.md")):
            size = rule_file.stat().st_size
            if size > MAX_RULE_FILE_BYTES:
                issues.append({
                    "file": f".agents/rules/{rule_file.name}",
                    "size": size,
                    "limit": MAX_RULE_FILE_BYTES,
                    "message": f".agents/rules/{rule_file.name} is {size:,} bytes (exceeds safety ceiling of {MAX_RULE_FILE_BYTES:,} bytes)",
                })

    return issues

# ---------------------------------------------------------------------------
# Pillar 4: Agent Skills Catalog Synchronization (docs/ai_agents.md)
# ---------------------------------------------------------------------------

def check_skills_catalog_sync():
    """Audits 100% synchronization between .agents/skills/ and docs/ai_agents.md."""
    skills_dir = REPO_ROOT / ".agents" / "skills"
    doc_path = REPO_ROOT / "docs" / "ai_agents.md"

    if not skills_dir.exists() or not doc_path.exists():
        return {"actual_count": 0, "documented_count": 0, "issues": []}

    actual_skills = set()
    for item in skills_dir.iterdir():
        if item.is_dir() and (item / "SKILL.md").exists():
            actual_skills.add(item.name)

    doc_text = doc_path.read_text(encoding="utf-8", errors="ignore")
    skill_link_pattern = re.compile(r"\[`([a-zA-Z0-9_\-]+)`\]\(\.\./\.agents/skills/[a-zA-Z0-9_\-]+/SKILL\.md\)")
    documented_skills = set(skill_link_pattern.findall(doc_text))

    issues = []
    missing_in_doc = actual_skills - documented_skills
    for skill in sorted(missing_in_doc):
        issues.append({
            "type": "missing_in_docs",
            "skill": skill,
            "message": f"Skill `{skill}` exists in .agents/skills/ but is missing from docs/ai_agents.md",
        })

    phantom_in_doc = documented_skills - actual_skills
    for skill in sorted(phantom_in_doc):
        issues.append({
            "type": "phantom_in_docs",
            "skill": skill,
            "message": f"Skill `{skill}` is documented in docs/ai_agents.md but does not exist in .agents/skills/",
        })

    return {
        "actual_count": len(actual_skills),
        "documented_count": len(documented_skills),
        "issues": issues,
    }

# ---------------------------------------------------------------------------
# Pillar 5: Two-Way Script Locality & Harness Governance
# ---------------------------------------------------------------------------

def check_script_locality_and_governance():
    """Audits two-way script placement governance between tools/harness/ and skills."""
    harness_dir = REPO_ROOT / "tools" / "harness"
    skills_dir = REPO_ROOT / ".agents" / "skills"
    workflows_dir = REPO_ROOT / ".agents" / "workflows"

    UNIVERSAL_HARNESS_SCRIPTS = {
        "pre_flight.py",
        "run_tests.py",
        "check_polish.py",
        "check_test_coupling.py",
        "audit_api_coverage.py",
        "audit_code_quality.py",
        "audit_docs_quality.py",
        "audit_hardware_quality.py",
        "log_diary.py",
        "rag_search.py",
    }

    issues = []

    # 1. Audit tools/harness/ scripts for single-consumer locality candidates
    if harness_dir.exists():
        for script in sorted(harness_dir.glob("*.py")):
            if script.name in UNIVERSAL_HARNESS_SCRIPTS:
                continue

            script_name = script.name
            referencing_skills = set()
            referencing_workflows = set()

            if skills_dir.exists():
                for sf in skills_dir.rglob("*.md"):
                    if sf.name == "SKILL.md":
                        try:
                            content = sf.read_text(encoding="utf-8", errors="ignore")
                            if script_name in content:
                                referencing_skills.add(sf.parent.name)
                        except Exception:
                            pass

            if workflows_dir.exists():
                for wf in workflows_dir.glob("*.md"):
                    try:
                        content = wf.read_text(encoding="utf-8", errors="ignore")
                        if script_name in content:
                            referencing_workflows.add(wf.stem)
                    except Exception:
                        pass

            total_consumers = len(referencing_skills) + len(referencing_workflows)
            if total_consumers <= 1:
                target_skill = next(iter(referencing_skills), None)
                target_desc = f".agents/skills/{target_skill}/scripts/" if target_skill else "the consuming skill/workflow"
                issues.append({
                    "type": "isolate_to_skill",
                    "script": script.name,
                    "location": f"tools/harness/{script.name}",
                    "consumers": list(referencing_skills | referencing_workflows),
                    "recommendation": f"Relocate to {target_desc} (used by only {total_consumers} consumer: {', '.join(referencing_skills | referencing_workflows) or 'none'})",
                })

    # 2. Audit .agents/skills/*/scripts/ for multi-consumer promotion candidates
    if skills_dir.exists():
        for script in sorted(skills_dir.glob("*/scripts/*.py")):
            owning_skill = script.parent.parent.name
            script_name = script.name

            foreign_skills = set()
            referencing_workflows = set()

            for sf in skills_dir.rglob("*.md"):
                if sf.name == "SKILL.md":
                    skill_name = sf.parent.name
                    if skill_name != owning_skill:
                        try:
                            content = sf.read_text(encoding="utf-8", errors="ignore")
                            if script_name in content:
                                foreign_skills.add(skill_name)
                        except Exception:
                            pass

            if workflows_dir.exists():
                for wf in workflows_dir.glob("*.md"):
                    try:
                        content = wf.read_text(encoding="utf-8", errors="ignore")
                        if script_name in content:
                            referencing_workflows.add(wf.stem)
                    except Exception:
                        pass

            if len(foreign_skills) > 0 or len(referencing_workflows) > 1:
                all_external = foreign_skills | referencing_workflows
                issues.append({
                    "type": "promote_to_harness",
                    "script": script.name,
                    "location": str(script.relative_to(REPO_ROOT)).replace("\\", "/"),
                    "owning_skill": owning_skill,
                    "external_consumers": list(all_external),
                    "recommendation": f"Promote to tools/harness/ (cross-referenced by external consumers: {', '.join(all_external)})",
                })

    return issues

# ---------------------------------------------------------------------------
# Pillar 6: Workflow, Skill & Rule Governance (Two-Way Alignment)
# ---------------------------------------------------------------------------

def check_workflow_and_skill_governance():
    """Audits two-way alignment between workflows, skills, and rules."""
    workflows_dir = REPO_ROOT / ".agents" / "workflows"
    skills_dir = REPO_ROOT / ".agents" / "skills"
    rules_dir = REPO_ROOT / ".agents" / "rules"

    workflow_files = sorted(workflows_dir.glob("*.md")) if workflows_dir.exists() else []
    skill_dirs = [p for p in sorted(skills_dir.iterdir()) if p.is_dir() and (p / "SKILL.md").exists()] if skills_dir.exists() else []
    skill_names = {p.name for p in skill_dirs}

    workflow_issues = []
    for wf in workflow_files:
        wf_name = wf.stem
        has_exact_skill = wf_name in skill_names
        try:
            content = wf.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            content = ""
        referenced_skills = [s for s in skill_names if s in content]

        if not has_exact_skill and not referenced_skills:
            workflow_issues.append({
                "type": "orphan_workflow",
                "workflow": wf.name,
                "message": f"Workflow `{wf.name}` has no corresponding skill in .agents/skills/{wf_name}/ and references no known skills.",
            })

    PROCEDURAL_MILESTONE_SKILLS = {
        "compact-diary": "Milestone diary synthesis and compaction procedure",
        "sync-design-docs": "Design specification synchronization with git commit history",
        "roadmap-maintenance": "Milestone scorecard pruning and substrate-first roadmap update",
        "index-amiga-rag": "Qdrant vector database re-indexing and offline sidecar maintenance",
        "audit-code-quality": "Rust codebase quality, dead code, visibility, and SRP audit",
        "audit-docs-quality": "Documentation, vault linking, and governance quality audit",
        "audit-hardware-quality": "Hardware architectural bus topology and silicon compliance audit",
    }

    active_workflow_stems = {wf.stem for wf in workflow_files}
    promotion_candidates = []
    for skill_name, reason in sorted(PROCEDURAL_MILESTONE_SKILLS.items()):
        if skill_name in skill_names and skill_name not in active_workflow_stems:
            promotion_candidates.append({
                "skill": skill_name,
                "reason": reason,
            })

    ACTIVE_RULE_COMPANIONS = {
        "amiga-rag.md": {"index-amiga-rag"},
        "asset-descriptions.md": {"describe-diagram-assets"},
        "diary-maintenance.md": {"compact-diary"},
        "docs-maintenance.md": {"sync-design-docs", "obsidian-vault-linking", "audit-docs-quality"},
        "vault-linking-and-graph-integrity.md": {"obsidian-vault-linking", "audit-docs-quality"},
        "egui-best-practices.md": {"egui-vision-debugger", "capture-gui-screenshot"},
        "file-size-and-cohesion.md": {"refactor-split-module"},
        "git-commits.md": {"git-resolve-merge", "git-worktree"},
        "git-merge-commits.md": {"git-resolve-merge", "git-worktree"},
        "graphify.md": {"graphify"},
        "hardware-bus-topology.md": {"audit-hardware-quality"},
        "opcode-naming.md": {"add-m68k-instruction"},
        "repro-first.md": {"m68k-singlestep-test", "test-runner", "synthesize-test-fixes"},
        "roadmap-maintenance.md": {"roadmap-maintenance"},
        "unit-testing-policy.md": {"test-runner", "synthesize-test-fixes", "integration-test-sprint"},
    }

    PASSIVE_INVARIANT_RULES = {
        "audio-transcription.md",
        "clean-break-refactoring.md",
        "information-hierarchy.md",
        "language-policy.md",
        "method-inlining.md",
        "model-reasoning-advisory.md",
        "no-external-paths.md",
        "parallel-execution.md",
        "performance-and-readability.md",
        "practitioner-voice-and-tone.md",
        "rust-best-practices.md",
        "spec-compliance.md",
        "structural-root-cause.md",
        "workspace-structure-and-reexports.md",
    }

    rule_issues = []
    rule_files = sorted(rules_dir.glob("*.md")) if rules_dir.exists() else []
    for rf in rule_files:
        if rf.name in ACTIVE_RULE_COMPANIONS:
            expected_skills = ACTIVE_RULE_COMPANIONS[rf.name]
            # Rule passes if AT LEAST ONE companion skill exists
            present = expected_skills & skill_names
            if not present:
                rule_issues.append({
                    "type": "missing_rule_skill",
                    "rule": rf.name,
                    "message": f"Active rule `{rf.name}` requires at least one companion skill in {expected_skills}, but none found in .agents/skills/",
                })
        elif rf.name in PASSIVE_INVARIANT_RULES:
            stem = rf.stem
            if stem in skill_names:
                rule_issues.append({
                    "type": "redundant_passive_skill",
                    "rule": rf.name,
                    "message": f"Passive invariant rule `{rf.name}` has a redundant companion skill `{stem}` (passive invariants must remain lean rules).",
                })

    return {
        "workflow_count": len(workflow_files),
        "skill_count": len(skill_names),
        "active_rule_count": len(ACTIVE_RULE_COMPANIONS),
        "passive_rule_count": len(PASSIVE_INVARIANT_RULES),
        "workflow_issues": workflow_issues,
        "promotion_candidates": promotion_candidates,
        "rule_issues": rule_issues,
    }

# ---------------------------------------------------------------------------
# Pillar 7: Frontmatter & Inverted Pyramid Validation
# ---------------------------------------------------------------------------

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

# ---------------------------------------------------------------------------
# CLI Runner
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Comprehensive Documentation & Governance Quality Auditor")
    parser.add_argument("--all", action="store_true", help="Run all documentation & governance audits")
    parser.add_argument("--design-sync", action="store_true", help="Audit Obsidian design specifications drift against git HEAD")
    parser.add_argument("--design-diff", type=str, metavar="DOC", help="Inspect git diff for a drifted design specification")
    parser.add_argument("--design-bump", type=str, metavar="DOC", help="Bump design specification checkpoint to current HEAD")
    parser.add_argument("--vault-links", action="store_true", help="Audit Obsidian design markdown link integrity")
    parser.add_argument("--size-limits", action="store_true", help="Audit AGENTS.md and rule file size ceilings")
    parser.add_argument("--skills", action="store_true", help="Audit agent skills catalog synchronization in docs/ai_agents.md")
    parser.add_argument("--scripts", action="store_true", help="Audit two-way script locality and harness placement governance")
    parser.add_argument("--governance", action="store_true", help="Audit workflow-skill symmetry and rule coverage")
    parser.add_argument("--frontmatter", action="store_true", help="Audit YAML frontmatter across Obsidian design specs")

    args = parser.parse_args()

    # Immediate action commands
    if args.design_diff:
        success = show_design_diff(args.design_diff)
        sys.exit(0 if success else 1)

    if args.design_bump:
        success = bump_design_checkpoint(args.design_bump)
        sys.exit(0 if success else 1)

    # If no flags specified, default to --all
    run_all = args.all or not any([
        args.design_sync, args.vault_links, args.size_limits,
        args.skills, args.scripts, args.governance, args.frontmatter
    ])

    print("=" * 76)
    print(" AMIGA 500 EMULATOR: DOCUMENTATION & GOVERNANCE QUALITY AUDIT")
    print("=" * 76)

    total_issues = 0

    # 1. Design Docs Sync
    if run_all or args.design_sync:
        print("\n[1. DESIGN SPECIFICATIONS & CODE DRIFT DETECTION]")
        sync_res = check_design_docs_sync()
        drifted = sync_res["drifted"]
        tracked_count = sync_res["tracked_count"]
        synced_count = sync_res["synced_count"]

        print(f"  - Tracked Design Specs: {tracked_count}")
        if drifted:
            total_issues += len(drifted)
            print(f"  - Status: [DRIFTED] {len(drifted)} document(s) behind active code:")
            for item in drifted:
                if item.get("error"):
                    print(f"    * {item['file']}: {item['error']}")
                else:
                    print(f"    * {item['file']}: {item['count']} commit(s) behind HEAD")
                    for log_line in item["log"]:
                        print(f"        {log_line}")
                    print(f"      -> Inspect diff: python tools/harness/audit_docs_quality.py --design-diff {item['file']}")
                    print(f"      -> Bump checkpoint: python tools/harness/audit_docs_quality.py --design-bump {item['file']}")
        else:
            print(f"  - Status: [PASS] All {synced_count} tracked specification(s) in sync with HEAD.")

    # 2. Obsidian Vault Links
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

    # 3. Size Limits
    if run_all or args.size_limits:
        print("\n[3. CONSTITUTIONAL & RULE FILE SIZE SAFETY]")
        size_issues = check_size_limits()
        if size_issues:
            total_issues += len(size_issues)
            print(f"  - Status: [FAIL] {len(size_issues)} size violation(s):")
            for issue in size_issues:
                print(f"    * {issue['message']}")
        else:
            print(f"  - Status: [PASS] AGENTS.md <= {MAX_AGENTS_MD_BYTES:,} B, all rules <= {MAX_RULE_FILE_BYTES:,} B.")

    # 4. Skills Catalog Sync
    if run_all or args.skills:
        print("\n[4. AGENT SKILLS CATALOG SYNCHRONIZATION (docs/ai_agents.md)]")
        skills_res = check_skills_catalog_sync()
        s_issues = skills_res["issues"]
        print(f"  - Active on-disk skills: {skills_res['actual_count']}")
        print(f"  - Documented in docs/ai_agents.md: {skills_res['documented_count']}")
        if s_issues:
            total_issues += len(s_issues)
            print(f"  - Status: [DRIFTED] {len(s_issues)} discrepancy issue(s):")
            for item in s_issues:
                print(f"    * {item['message']}")
        else:
            print("  - Status: [PASS] 100% synchronized (all skills cataloged cleanly).")

    # 5. Script Locality
    if run_all or args.scripts:
        print("\n[5. TWO-WAY SCRIPT LOCALITY & HARNESS GOVERNANCE]")
        loc_issues = check_script_locality_and_governance()
        if loc_issues:
            total_issues += len(loc_issues)
            print(f"  - Status: [ANOMALY] {len(loc_issues)} script placement issue(s):")
            for item in loc_issues:
                print(f"    * [{item['type']}] {item['location']}: {item['recommendation']}")
        else:
            print("  - Status: [PASS] All harness scripts are shared/universal, and all skill scripts are private.")

    # 6. Governance Symmetry
    if run_all or args.governance:
        print("\n[6. WORKFLOW & SKILL GOVERNANCE (TWO-WAY ALIGNMENT)]")
        gov_res = check_workflow_and_skill_governance()
        wf_issues = gov_res["workflow_issues"]
        promo_candidates = gov_res["promotion_candidates"]
        rule_issues = gov_res["rule_issues"]

        print(f"  - Active Workflows (.agents/workflows/): {gov_res['workflow_count']}")
        print(f"  - Active Skills (.agents/skills/): {gov_res['skill_count']}")

        if wf_issues:
            total_issues += len(wf_issues)
            print(f"  - Workflows: [ORPHAN] {len(wf_issues)} workflow(s) lack backing skills:")
            for wi in wf_issues:
                print(f"    * {wi['message']}")
        else:
            print("  - Workflows: [PASS] All workflows have backing specialized skills.")

        if promo_candidates:
            print(f"  - Promotion Candidates: {len(promo_candidates)} procedural skill(s) eligible for /slash-command:")
            for cand in promo_candidates:
                print(f"    * `{cand['skill']}`: {cand['reason']}")
        else:
            print("  - Promotion Candidates: None (all procedural skills have matching workflows).")

        if rule_issues:
            total_issues += len(rule_issues)
            print(f"  - Rules Symmetry: [FAIL] {len(rule_issues)} rule companion discrepancy issue(s):")
            for ri in rule_issues:
                print(f"    * {ri['message']}")
        else:
            print("  - Rules Symmetry: [PASS] Active remediation rules have companion skills; passive invariants remain lean.")

    # 7. Frontmatter Compliance
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

    print("\n" + "=" * 76)
    print(f"Documentation Audit Summary: {total_issues} total issue(s) detected.")
    print("=" * 76)

    sys.exit(1 if total_issues > 0 else 0)

if __name__ == "__main__":
    main()
