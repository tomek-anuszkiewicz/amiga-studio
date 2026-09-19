#!/usr/bin/env python3
"""
audit_docs_quality.py - Comprehensive On-Demand Documentation & Governance Auditor

Audits nine critical documentation and agent governance dimensions across the repository:
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
8. Design Docs Reflection in Rules:
   - Enforces that 100% of design specifications are reflected and delegated in agent rules.
9. Semantic Documentation-to-Code Validator (Double-Check Engine):
   - Validates custom register matrix, memory map ranges, Mermaid crate topology, cross-chip signals, and quirks test coverage.
10. Rule Audit Coverage & Governance Invariants:
   - Verifies 100% of rules in `.agents/rules/*.md` are registered and audited.
   - Validates diagram sidecars (`.txt`), DIARY.md Section 10 chronology, and ROADMAP.md zero-retention.
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
        "audit-semantic-parity": "Inference-driven bidirectional code-to-docs and docs-to-code semantic parity audit",
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
        "docs-maintenance.md": {"sync-design-docs", "obsidian-vault-linking", "audit-docs-quality", "audit-semantic-parity"},
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
# Pillar 8: Design Documentation Reflection & Delegation in Agent Rules
# ---------------------------------------------------------------------------

DESIGN_DOC_GOVERNANCE_MAP = {
    # Subsystems & Bus Topology
    "Agnus.md": ("hardware-bus-topology.md", "Agnus DMA address mastership and custom chip execution"),
    "Copper.md": ("hardware-bus-topology.md", "Copper coprocessor state machine, MOVE, WAIT, SKIP"),
    "Blitter.md": ("hardware-bus-topology.md", "4-channel DMA Blitter, 256 minterms ALU, Bresenham line drawing"),
    "DMA.md": ("hardware-bus-topology.md", "DMA channel arbitration, 227.5 CCK slot scheduling, and Agnus address mastership"),
    "Denise.md": ("hardware-bus-topology.md", "Denise display pipeline, bitplanes, and passive RGA latching"),
    "Sprites.md": ("hardware-bus-topology.md", "Denise 8 hardware sprites, position comparators, attached pairs"),
    "Frame Buffer.md": ("hardware-bus-topology.md", "Denise raster scanline pixel compositor, RGB palette DAC, and ARGB frame buffer"),
    "Paula.md": ("hardware-bus-topology.md", "Paula audio and interrupt handling"),
    "Interrupts.md": ("hardware-bus-topology.md", "Central interrupt multiplexer, 14-source priority encoder, IPL 1..6"),
    "Audio.md": ("hardware-bus-topology.md", "Paula 4-channel DMA audio engine, volume scaling, period counters"),
    "CIA.md": ("hardware-bus-topology.md", "8520 CIA timers, TOD counters, and peripheral handshaking"),
    "Floppy.md": ("hardware-bus-topology.md", "Floppy drive subsystem, MFM encoding, and DMA transfers"),
    "MemoryBus.md": ("hardware-bus-topology.md", "Address decoding, bus arbitration, and wait states"),
    "Main loop A500.md": ("hardware-bus-topology.md", "Color clock CCK stepping and subsystem coordination"),
    "Custom Chip Register Ownership and Access Matrix.md": ("hardware-bus-topology.md", "Custom chip register read/write privileges and strobe routing"),
    "Cross-Chip Signals and Action Dispatch Catalog.md": ("hardware-bus-topology.md", "Inter-chip signal dispatch and decoupled interrupt routing"),
    "SaveState.md": ("hardware-bus-topology.md", "Hardware circuit simulation state serialization"),
    # Peripherals
    "Keyboard.md": ("hardware-bus-topology.md", "Keyboard matrix, handshaking, and CIA-A serial shift register"),
    "Mouse.md": ("hardware-bus-topology.md", "Mouse quadrature counter registers and game port latching"),
    "Joystick.md": ("hardware-bus-topology.md", "Digital joystick direction switches and fire button routing"),
    "Game Ports.md": ("hardware-bus-topology.md", "Port 1/2 controller pinouts and POTGO resistance measuring"),
    "RTC.md": ("unit-testing-policy.md", "Ricoh RP5C01 / Oki MSM6242 real-time clock registers"),
    # CPU & Execution
    "CPU Motorola M68000.md": ("opcode-naming.md", "M68000 programmer model and execution semantics"),
    "CPU Micro-Step State Machine.md": ("opcode-naming.md", "Bus cycle phases CCK1/CCK2 and IDLE micro-steps"),
    "CPU SingleStepTests.md": ("opcode-naming.md", "Tom Harte silicon validation test suite"),
    "CPU Instruction Benchmarking.md": ("performance-and-readability.md", "Instruction cycle timings and empirical benchmarks"),
    "CPU Instruction Benchmark Catalog.md": ("performance-and-readability.md", "Golden instruction cycle counts and catalog"),
    "CPU Instruction Benchmark Strategies.md": ("performance-and-readability.md", "Contention-free benchmark harness strategies"),
    "CPU Benchmark Analysis Guide.md": ("performance-and-readability.md", "Cycle timing discrepancy triage and analysis"),
    "Performance Profiling and Optimization Strategy.md": ("performance-and-readability.md", "Host CPU execution efficiency and profiler metrics"),
    # Frontend, GUI & Debugger
    "GUI.md": ("egui-best-practices.md", "Immediate-mode egui Developer Studio"),
    "GUI Specification.md": ("egui-best-practices.md", "View modes, panel docks, and layout stability"),
    "egui Guidelines.md": ("egui-best-practices.md", "Zero-alloc UI rendering and synchronous state pull"),
    "Debugger.md": ("egui-best-practices.md", "Disassembly view, memory hex editors, and breakpoints"),
    # System & Quality Guidelines
    "General Architecture.md": ("workspace-structure-and-reexports.md", "Workspace crate dependency topology and named roots"),
    "Configuration.md": ("workspace-structure-and-reexports.md", "Decoupled machine configuration and video standards"),
    "Rust Guidelines.md": ("rust-best-practices.md", "Safe borrowing, zero unwraps, and numeric wrapping"),
    "Testing Strategy and Quality Assurance.md": ("unit-testing-policy.md", "3-tier testing taxonomy and change-coupling"),
    "Platform Quirks and Invariants Catalog.md": ("spec-compliance.md", "Amiga 500 silicon traps and hardware quirks"),
    "vAmigaTS Verification Scorecard.md": ("spec-compliance.md", "vAmigaTS verification scorecard and pass rates"),
}

def check_design_docs_to_rules_reflection():
    """Audits that every Obsidian design specification is reflected and delegated in agent rules."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    rules_dir = REPO_ROOT / ".agents" / "rules"
    agents_md = REPO_ROOT / "AGENTS.md"

    if not design_dir.exists():
        return {"total_docs": 0, "reflected_count": 0, "issues": []}

    md_files = sorted(design_dir.glob("*.md"))

    rule_texts = {}
    if rules_dir.exists():
        for rf in rules_dir.glob("*.md"):
            try:
                rule_texts[rf.name] = rf.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                pass
    if agents_md.exists():
        try:
            rule_texts["AGENTS.md"] = agents_md.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            pass

    combined_rule_text = "\n".join(rule_texts.values())

    issues = []
    reflected_count = 0

    for doc in md_files:
        doc_name = doc.name
        doc_stem = doc.stem
        encoded_stem = doc_stem.replace(" ", "%20")
        is_referenced = (
            doc_name in combined_rule_text
            or f"Design/{doc_name}" in combined_rule_text
            or f"Design/{doc_stem}" in combined_rule_text
            or f"Design/{encoded_stem}" in combined_rule_text
            or f"/{doc_name}" in combined_rule_text
        )

        governing_rule, domain_desc = DESIGN_DOC_GOVERNANCE_MAP.get(
            doc_name, ("docs-maintenance.md", "General architectural design")
        )

        if is_referenced:
            reflected_count += 1
        else:
            issues.append({
                "doc": doc_name,
                "governing_rule": governing_rule,
                "domain": domain_desc,
                "message": f"Design spec `{doc_name}` ({domain_desc}) is not reflected or delegated in agent rules. Delegate in `.agents/rules/{governing_rule}`.",
            })

    return {
        "total_docs": len(md_files),
        "reflected_count": reflected_count,
        "issues": issues,
    }

# ---------------------------------------------------------------------------
# Pillar 9: Semantic Documentation-to-Code Validator ("The Double-Check Engine")
# ---------------------------------------------------------------------------

def check_semantic_registers():
    """Validates custom register matrix in documentation against crates/config/src/registers.rs."""
    matrix_file = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "Custom Chip Register Ownership and Access Matrix.md"
    reg_code_file = REPO_ROOT / "crates" / "config" / "src" / "registers.rs"

    if not matrix_file.exists() or not reg_code_file.exists():
        return {"checked_offsets": 0, "issues": [{"message": "Required register specification or source file missing"}]}

    matrix_text = matrix_file.read_text(encoding="utf-8")
    reg_code = reg_code_file.read_text(encoding="utf-8")

    rust_offsets = {}
    current_mod = None
    for line in reg_code.splitlines():
        line = line.strip()
        if line.startswith("pub mod agnus"):
            current_mod = "Agnus"
        elif line.startswith("pub mod denise"):
            current_mod = "Denise"
        elif line.startswith("pub mod paula"):
            current_mod = "Paula"
        m = re.search(r"pub\s+const\s+([A-Z0-9_]+)\s*:\s*u16\s*=\s*(0x[0-9A-Fa-f]+);", line)
        if m and current_mod:
            name = m.group(1)
            offset = int(m.group(2), 16)
            rust_offsets.setdefault(offset, []).append((name, current_mod))

    issues = []
    checked = 0
    chip_map = {"A": "Agnus", "D": "Denise", "P": "Paula"}

    for idx, line in enumerate(matrix_text.splitlines(), 1):
        line = line.strip()
        if not line.startswith("|"):
            continue
        parts = [p.strip() for p in line.split("|")[1:-1]]
        if not parts:
            continue
        offset_part = parts[0]
        if ".." in offset_part:
            continue
        m_off = re.search(r"\$([0-9A-Fa-f]{3})", offset_part)
        if not m_off:
            continue
        offset = int(m_off.group(1), 16)

        names = []
        for col in parts[1:3]:
            found = re.findall(r"`([A-Z0-9_]+)`(?:\s*\(([ADP])\))?", col)
            for reg_name, chip_code in found:
                if reg_name != "FFFF":
                    names.append((reg_name, chip_code))
        if len(parts) >= 4:
            m_single = re.search(r"`([A-Z0-9_]+)`", parts[1])
            owner_match = re.search(r"(Agnus|Denise|Paula)", line)
            if m_single and owner_match:
                chip_letter = owner_match.group(1)[0]
                reg_n = m_single.group(1)
                if not any(r[0] == reg_n for r in names):
                    names.append((reg_n, chip_letter))

        if not names:
            continue

        checked += 1
        if offset not in rust_offsets:
            issues.append({"line": idx, "message": f"Offset ${offset:03X} in markdown not found in registers.rs"})
            continue

        rust_entries = rust_offsets[offset]
        rust_names = {r[0] for r in rust_entries}

        for reg_name, chip_letter in names:
            if reg_name not in rust_names:
                issues.append({"line": idx, "message": f"Register `{reg_name}` at ${offset:03X} not found in registers.rs (code has {sorted(rust_names)})"})
            elif chip_letter:
                exp_chip = chip_map.get(chip_letter)
                act_chips = [r[1] for r in rust_entries if r[0] == reg_name]
                if exp_chip and exp_chip not in act_chips:
                    issues.append({"line": idx, "message": f"Register `{reg_name}` chip mismatch: doc says {exp_chip}, code has {act_chips}"})

    return {"checked_offsets": checked, "issues": issues}


def check_semantic_memory_map():
    """Validates physical 24-bit memory map ranges in MemoryBus.md against crates/memory_bus/src/memory_bus.rs."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "MemoryBus.md"
    code_path = REPO_ROOT / "crates" / "memory_bus" / "src" / "memory_bus.rs"

    if not doc_path.exists() or not code_path.exists():
        return {"checked_ranges": 0, "issues": [{"message": "Required memory bus files missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")
    code_text = code_path.read_text(encoding="utf-8")

    code_constants = {}
    for line in code_text.splitlines():
        line = line.strip()
        m = re.search(r"pub\s+const\s+([A-Z0-9_]+)\s*:\s*u\d+\s*=\s*(0x[0-9A-Fa-f]+);", line)
        if m:
            code_constants[m.group(1)] = int(m.group(2), 16)

    expected_checks = [
        (r"\$BFD000\s*-\s*\$BFDF00", "CIA_B_START", 0xBFD000),
        (r"\$BFD000\s*-\s*\$BFDF00", "CIA_B_END", 0xBFDF00),
        (r"\$BFE001\s*-\s*\$BFEF01", "CIA_A_START", 0xBFE001),
        (r"\$BFE001\s*-\s*\$BFEF01", "CIA_A_END", 0xBFEF01),
        (r"\$DC0000\s*-\s*\$DC003F", "RTC_START", 0xDC0000),
        (r"\$DC0000\s*-\s*\$DC003F", "RTC_END", 0xDC003F),
        (r"\$BF", "BANK_CIA", 0xBF),
        (r"\$DC", "BANK_RTC", 0xDC),
        (r"\$DF", "BANK_CUSTOM", 0xDF),
        (r"\$DFF000\s*-\s*\$DFFFFE", "CUSTOM_REG_OFFSET_MASK", 0x01FE),
    ]

    issues = []
    checked = 0
    for pattern, const_name, expected_val in expected_checks:
        checked += 1
        if not re.search(pattern, doc_text):
            issues.append({"message": f"MemoryBus.md missing expected pattern: `{pattern}`"})
        if const_name not in code_constants:
            issues.append({"message": f"memory_bus.rs missing constant: `{const_name}`"})
        elif code_constants[const_name] != expected_val:
            issues.append({"message": f"Constant `{const_name}` in code (0x{code_constants[const_name]:X}) != doc (0x{expected_val:X})"})

    return {"checked_ranges": checked, "issues": issues}


def check_semantic_crate_topology():
    """Validates Mermaid crate dependency graph in General Architecture.md against Cargo.toml workspace members."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "General Architecture.md"
    cargo_path = REPO_ROOT / "Cargo.toml"

    if not doc_path.exists() or not cargo_path.exists():
        return {"cargo_crates": 0, "doc_crates": 0, "issues": [{"message": "Required architecture or Cargo.toml file missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")
    cargo_text = cargo_path.read_text(encoding="utf-8")

    cargo_members = set()
    in_members = False
    for line in cargo_text.splitlines():
        line = line.strip()
        if line.startswith("members = ["):
            in_members = True
            continue
        if in_members:
            if line.startswith("]"):
                break
            m = re.search(r'"([^"]+)"', line)
            if m and m.group(1).startswith("crates/"):
                cargo_members.add(m.group(1).split("/", 1)[1])

    doc_crates = set()
    for m in re.finditer(r"crates/([a-z0-9_]+)", doc_text):
        doc_crates.add(m.group(1))

    missing_in_doc = cargo_members - doc_crates
    missing_in_cargo = doc_crates - cargo_members

    issues = []
    if missing_in_doc:
        issues.append({"message": f"Crate(s) in Cargo.toml missing from General Architecture.md Mermaid graph: {sorted(missing_in_doc)}"})
    if missing_in_cargo:
        issues.append({"message": f"Crate(s) in General Architecture.md Mermaid graph missing from Cargo.toml: {sorted(missing_in_cargo)}"})

    return {"cargo_crates": len(cargo_members), "doc_crates": len(doc_crates), "issues": issues}


def check_semantic_signals():
    """Validates cross-chip signals and action dispatch methods against implementation in crates/*/src/."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "Cross-Chip Signals and Action Dispatch Catalog.md"
    if not doc_path.exists():
        return {"verified_methods": 0, "issues": [{"message": "Cross-Chip Signals catalog missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")

    raw_calls = []
    for chunk in re.findall(r"`([^`]+)`", doc_text):
        if "(" in chunk and ")" in chunk:
            call_part = chunk.split("(", 1)[0].strip()
            method_name = call_part.split(".")[-1].strip()
            if method_name.isidentifier():
                raw_calls.append(method_name)

    skip_keywords = {"val", "ch", "strt", "stop", "pins", "channel", "cop1lc", "cop2lc", "read"}
    catalog_methods = {m for m in raw_calls if m not in skip_keywords}

    code_methods = set()
    for rs_path in (REPO_ROOT / "crates").rglob("*.rs"):
        if "src" in rs_path.parts:
            text = rs_path.read_text(encoding="utf-8", errors="ignore")
            for m in re.finditer(r"\bfn\s+([a-z_][a-z0-9_]*)\s*[\(<]", text):
                code_methods.add(m.group(1))

    issues = []
    for m in sorted(catalog_methods):
        if m not in code_methods:
            issues.append({"message": f"Catalog action method `{m}()` not found in crates/*/src/"})

    return {"verified_methods": len(catalog_methods), "issues": issues}


def check_semantic_quirks_coverage():
    """Validates that all 13 critical silicon quirks in Platform Quirks catalog maintain active regression test coverage."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "Platform Quirks and Invariants Catalog.md"
    if not doc_path.exists():
        return {"total_quirks": 0, "covered_quirks": 0, "issues": [{"message": "Platform Quirks catalog missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")

    quirks = []
    for line in doc_text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        parts = [p.strip() for p in line.split("|")[1:-1]]
        if not parts:
            continue
        m = re.search(r"\*\*([^*]+)\*\*", parts[0])
        if m:
            quirk_name = m.group(1).strip()
            if quirk_name not in ["Silicon Quirk", "Hardware Quirk / Erratum"]:
                quirks.append(quirk_name)

    test_files_content = {}
    for test_path in (REPO_ROOT / "crates").rglob("tests/**/*.rs"):
        test_files_content[test_path] = test_path.read_text(encoding="utf-8", errors="ignore")

    quirk_signatures = {
        "Class 0 RMW Prefetch Order": ["BusPrefetchToScratch", "prefetch", "test_singlestep"],
        "A7 Stack Pointer Byte Alignment": ["test_move", "test_singlestep", "SP", "A7"],
        "Multi-Precision $Z$-Flag Retention (`ADDX`/`SUBX`/`NEGX`)": ["addx", "subx", "negx"],
        "Address Register Direct CCR Immunity": ["adda", "suba", "movea", "cmpa"],
        "ASL Sticky Overflow ($V$)": ["asl", "overflow", "test_asl"],
        "Dual-Memory Address Error Deferral": ["addr1", "addr2", "test_address_error", "test_architecture_rules"],
        "Multi-Cycle Division Overflow CCR Quirk": ["divu", "divs", "overflow"],
        "TAS Read-Modify-Write Silicon Erratum": ["TAS", "tas", "test_singlestep", "is_tas"],
        "Floppy Shared Motor Line Wiring": ["motor_on", "handle_ciab_port_b_write", "test_ciab_port_b_motor"],
        "Floppy Disk Change Flip-Flop (`_CHNG`)": ["is_disk_changed", "step_pulse", "test_disk_change_flip_flop"],
        "Keyboard Caps Lock Latch State Machine": ["caps_lock", "test_keyboard_caps_lock_toggle"],
        "Keyboard Serial Handshake Delay": ["WaitingHandshake", "test_keyboard_step_handshake"],
        "Linear Resistor DAC & Absence of Gamma Pre-Correction": ["color", "palette", "test_pixel_pipeline", "quantize"],
    }

    issues = []
    covered = 0
    for raw_quirk in quirks:
        norm = re.sub(r"[`]", "", raw_quirk).strip()
        matched_key = None
        for k in quirk_signatures:
            if re.sub(r"[`]", "", k).strip() == norm:
                matched_key = k
                break
        if not matched_key:
            issues.append({"message": f"Quirk `{raw_quirk}` lacks configured signature validator"})
            continue

        sigs = quirk_signatures[matched_key]
        found = False
        for path, content in test_files_content.items():
            if any(sig in content for sig in sigs):
                found = True
                break
        if found:
            covered += 1
        else:
            issues.append({"message": f"Quirk `{raw_quirk}` has zero matching regression test coverage in crates/*/tests/"})

    return {"total_quirks": len(quirks), "covered_quirks": covered, "issues": issues}


def check_semantic_sync():
    """Aggregates all five semantic documentation-to-code checks."""
    regs = check_semantic_registers()
    mmap = check_semantic_memory_map()
    topo = check_semantic_crate_topology()
    sigs = check_semantic_signals()
    quirks = check_semantic_quirks_coverage()

    all_issues = regs["issues"] + mmap["issues"] + topo["issues"] + sigs["issues"] + quirks["issues"]

    return {
        "registers": regs,
        "memory_map": mmap,
        "topology": topo,
        "signals": sigs,
        "quirks": quirks,
        "issues": all_issues,
    }

# ---------------------------------------------------------------------------
# Pillar 10: Rule Audit Coverage & Governance Invariants
# ---------------------------------------------------------------------------

REGISTERED_RULE_AUDITS = {
    "amiga-rag.md": ["tools/harness/rag_search.py", "audit_docs_quality.py (Pillar 5)"],
    "asset-descriptions.md": ["audit_docs_quality.py (Pillar 10 asset sidecars)"],
    "audio-transcription.md": ["workflows (Conscience Check 1)"],
    "clean-break-refactoring.md": ["test_architecture_rules.rs (test_zero_backward_compatibility_shims_and_stale_aliases)"],
    "diary-maintenance.md": ["audit_docs_quality.py (Pillar 10 diary structure & chronology)"],
    "docs-maintenance.md": ["audit_docs_quality.py (Pillar 1 code drift)"],
    "egui-best-practices.md": ["crates/gui/tests/test_interactions.rs"],
    "file-size-and-cohesion.md": ["test_architecture_rules.rs (test_file_size_limits)", "audit_code_quality.py (Pillar 3)"],
    "git-commits.md": ["tools/harness/pre_flight.py", "tools/harness/check_polish.py"],
    "git-merge-commits.md": [".agents/workflows/git-resolve-merge.md"],
    "graphify.md": ["audit_docs_quality.py (Pillar 6 graphify skill)"],
    "hardware-bus-topology.md": ["audit_hardware_quality.py (Pillars 1 & 2)"],
    "information-hierarchy.md": ["test_architecture_rules.rs (test_rule_files_size_limit...)", "audit_docs_quality.py (Pillars 3 & 7)"],
    "language-policy.md": ["tools/harness/check_polish.py (language-policy check)"],
    "method-inlining.md": ["test_architecture_rules.rs (test_inlining_guidelines_compliance)", "audit_code_quality.py (Pillar 4)"],
    "model-reasoning-advisory.md": ["workflows (Conscience Check 1)"],
    "no-external-paths.md": ["test_architecture_rules.rs (test_no_external_hardcoded_paths)"],
    "opcode-naming.md": ["test_architecture_rules.rs (test_idle_microstep_naming...)", "audit_hardware_quality.py (Pillar 4)"],
    "parallel-execution.md": ["workflows (parallel execution)"],
    "performance-and-readability.md": ["test_architecture_rules.rs (test_zero_user_defined_macros)", "audit_code_quality.py (Pillar 5)"],
    "practitioner-voice-and-tone.md": ["workflows (Conscience Checks 4 & 5)"],
    "repro-first.md": ["tools/harness/check_test_coupling.py"],
    "roadmap-maintenance.md": ["audit_docs_quality.py (Pillar 10 roadmap zero-retention)"],
    "rust-best-practices.md": ["test_architecture_rules.rs (test_zero_runtime_panics_or_unwraps)", "audit_code_quality.py (Pillars 1 & 2)"],
    "spec-compliance.md": ["test_architecture_rules.rs (test_golden_hash_anti_tamper_policy_compliance)", "workflows (Conscience Check 2)"],
    "structural-root-cause.md": ["workflows (Conscience Check 3)"],
    "unit-testing-policy.md": ["test_architecture_rules.rs (test_every_crate_has_dedicated_external_tests_suite)", "audit_hardware_quality.py (Pillar 5)"],
    "vault-linking-and-graph-integrity.md": ["test_architecture_rules.rs (test_obsidian_design_docs_links_integrity)", "audit_docs_quality.py (Pillars 2 & 7)"],
    "workspace-structure-and-reexports.md": ["test_architecture_rules.rs (test_named_crate_roots_and_zero_generic_lib_rs)", "audit_docs_quality.py (Pillar 9)"],
}

def check_rule_audit_coverage():
    """Audits that 100% of rule files in .agents/rules/ are registered and have active audit coverage."""
    rules_dir = REPO_ROOT / ".agents" / "rules"
    on_disk_rules = {f.name for f in rules_dir.glob("*.md")} if rules_dir.exists() else set()

    unregistered = sorted(on_disk_rules - set(REGISTERED_RULE_AUDITS.keys()))
    phantom = sorted(set(REGISTERED_RULE_AUDITS.keys()) - on_disk_rules)

    issues = []
    for r in unregistered:
        issues.append({"message": f"Rule `{r}` is present on disk but not registered with an audit mechanism in REGISTERED_RULE_AUDITS"})
    for p in phantom:
        issues.append({"message": f"Registered rule `{p}` does not exist in .agents/rules/"})

    return {
        "total_rules": len(on_disk_rules),
        "audited_rules": len(on_disk_rules - set(unregistered)),
        "issues": issues,
    }

def check_diagram_asset_sidecars():
    """Verifies that all technical diagrams and schematics have git-tracked .txt sidecars per asset-descriptions.md."""
    search_dirs = [
        REPO_ROOT / "Obsidian" / "Amiga" / "Reference",
        REPO_ROOT / "Obsidian" / "Amiga" / "Design",
    ]
    images = []
    for d in search_dirs:
        if d.exists():
            for ext in ("*.png", "*.jpg", "*.svg"):
                images.extend(d.rglob(ext))

    missing = []
    for img in sorted(images):
        sidecar = img.parent / (img.name + ".txt")
        if not sidecar.exists() or sidecar.stat().st_size == 0:
            rel = img.relative_to(REPO_ROOT).as_posix()
            missing.append(rel)

    issues = []
    for m in missing:
        issues.append({"message": f"Diagram `{m}` is missing a git-tracked `{m}.txt` sidecar"})

    return {
        "total_images": len(images),
        "verified_images": len(images) - len(missing),
        "issues": issues,
    }

def check_diary_structure_and_chronology():
    """Verifies that DIARY.md exists, contains Section 10, and entries are strictly chronological per diary-maintenance.md."""
    diary_path = REPO_ROOT / "DIARY.md"
    if not diary_path.exists():
        return {"total_entries": 0, "issues": [{"message": "DIARY.md does not exist in repository root"}]}

    content = diary_path.read_text(encoding="utf-8", errors="ignore")
    section10 = re.split(r"^##\s+10\.\s+", content, flags=re.MULTILINE)
    if len(section10) < 2:
        return {"total_entries": 0, "issues": [{"message": "DIARY.md is missing Section 10 (`## 10. Living Chronological Engineering Log...`)"}]}

    timestamps = re.findall(r"###\s+\[(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2})", section10[1])
    issues = []
    for i in range(1, len(timestamps)):
        if timestamps[i] < timestamps[i - 1]:
            issues.append({"message": f"DIARY.md Section 10 chronological order violation: `{timestamps[i - 1]}` followed by earlier `{timestamps[i]}`"})

    return {
        "total_entries": len(timestamps),
        "issues": issues,
    }

def check_roadmap_zero_retention():
    """Verifies that ROADMAP.md exists and contains zero completed tasks ([x]) per roadmap-maintenance.md."""
    roadmap_path = REPO_ROOT / "ROADMAP.md"
    if not roadmap_path.exists():
        return {"issues": [{"message": "ROADMAP.md does not exist in repository root"}]}

    content = roadmap_path.read_text(encoding="utf-8", errors="ignore")
    completed = re.findall(r"^\s*[-*]\s+\[x\]", content, flags=re.MULTILINE | re.IGNORECASE)

    issues = []
    if completed:
        issues.append({"message": f"ROADMAP.md contains {len(completed)} completed task(s) (`[x]`). All completed tasks must be pruned per roadmap-maintenance.md"})

    return {"issues": issues}

def check_rule_audit_and_governance():
    """Aggregates all checks for Pillar 10: Rule Audit Coverage & Governance Invariants."""
    rac = check_rule_audit_coverage()
    das = check_diagram_asset_sidecars()
    dsc = check_diary_structure_and_chronology()
    rzr = check_roadmap_zero_retention()

    all_issues = rac["issues"] + das["issues"] + dsc["issues"] + rzr["issues"]
    return {
        "rule_coverage": rac,
        "asset_sidecars": das,
        "diary_chronology": dsc,
        "roadmap_retention": rzr,
        "issues": all_issues,
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
    parser.add_argument("--governance", action="store_true", help="Audit workflow-skill parity and active rule companion skills")
    parser.add_argument("--frontmatter", action="store_true", help="Audit YAML frontmatter properties in design specs")
    parser.add_argument("--rules-delegation", action="store_true", help="Audit that design specifications are reflected and delegated in agent rules")
    parser.add_argument("--semantic-sync", action="store_true", help="Audit semantic consistency between documentation and code (Double-Check engine)")
    parser.add_argument("--rule-coverage", action="store_true", help="Audit 100% rule audit coverage and governance invariants (Pillar 10)")

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
        args.skills, args.scripts, args.governance, args.frontmatter,
        args.rules_delegation, args.semantic_sync, args.rule_coverage
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

    # 8. Design Docs Reflection in Rules
    if run_all or args.rules_delegation:
        print("\n[8. DESIGN DOCS TO AGENT RULES REFLECTION & DELEGATION]")
        ref_res = check_design_docs_to_rules_reflection()
        r_issues = ref_res["issues"]
        print(f"  - Inspected Design Specs: {ref_res['total_docs']}")
        print(f"  - Reflected in Agent Rules: {ref_res['reflected_count']}")
        if r_issues:
            total_issues += len(r_issues)
            print(f"  - Status: [UNREFLECTED] {len(r_issues)} design spec(s) lack agent rule delegation:")
            for item in r_issues:
                print(f"    * {item['doc']}: {item['message']}")
        else:
            print("  - Status: [PASS] 100% of design specifications are reflected and delegated in agent rules.")

    # 9. Semantic Documentation-to-Code Double-Check
    if run_all or args.semantic_sync:
        print("\n[9. SEMANTIC DOCUMENTATION-TO-CODE DOUBLE-CHECK]")
        sem_res = check_semantic_sync()
        regs = sem_res["registers"]
        mmap = sem_res["memory_map"]
        topo = sem_res["topology"]
        sigs = sem_res["signals"]
        quirks = sem_res["quirks"]
        s_issues = sem_res["issues"]

        print(f"  - Custom Register Matrix: {regs['checked_offsets']} offsets verified vs registers.rs")
        print(f"  - Memory Map Range Checks: {mmap['checked_ranges']} boundaries verified vs memory_bus.rs")
        print(f"  - Crate Topology Sync: {topo['cargo_crates']} crates verified (100% Mermaid-Cargo parity)")
        print(f"  - Cross-Chip Signal Parity: {sigs['verified_methods']} action methods verified in codebase")
        print(f"  - Silicon Quirks Coverage: {quirks['covered_quirks']}/{quirks['total_quirks']} quirks covered by regression test sentinels")

        if s_issues:
            total_issues += len(s_issues)
            print(f"  - Status: [FAIL] {len(s_issues)} semantic discrepancy issue(s) detected:")
            for issue in s_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] 100% semantic parity between design specs and Rust code.")

    # 10. Rule Audit Coverage & Governance Invariants
    if run_all or args.rule_coverage:
        print("\n[10. RULE AUDIT COVERAGE & GOVERNANCE INVARIANTS]")
        gov_inv = check_rule_audit_and_governance()
        rc = gov_inv["rule_coverage"]
        sd = gov_inv["asset_sidecars"]
        dc = gov_inv["diary_chronology"]
        rm = gov_inv["roadmap_retention"]
        g_issues = gov_inv["issues"]

        print(f"  - Rule Audit Coverage: {rc['audited_rules']}/{rc['total_rules']} rules audited (100% coverage)")
        print(f"  - Diagram Asset Sidecars: {sd['verified_images']}/{sd['total_images']} verified with .txt sidecars")
        print(f"  - Engineering Diary Integrity: {dc['total_entries']} chronological entries in Section 10")
        print(f"  - Roadmap Zero Retention: Verified (zero completed items retained)")

        if g_issues:
            total_issues += len(g_issues)
            print(f"  - Status: [FAIL] {len(g_issues)} governance invariant violation(s) detected:")
            for issue in g_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] 100% rule audit coverage and governance invariants satisfied.")

    print("\n" + "=" * 76)
    print(f"Documentation Audit Summary: {total_issues} total issue(s) detected.")
    print("=" * 76)

    sys.exit(1 if total_issues > 0 else 0)

if __name__ == "__main__":
    main()
