#!/usr/bin/env python3
"""
audit_code_quality.py - Comprehensive On-Demand Code Quality Auditor

Audits three critical architectural dimensions across workspace crates:
1. Dead Code & Test-Only Zombies:
   - Completely dead symbols (0 callers anywhere).
   - Test-only zombie symbols (0 callers in production crates/*/src/, only called in tests/).
2. Principle of Least Visibility (Information Hiding):
   - Over-exposed `pub` items that are only called within their own defining crate (should be `pub(crate)`).
   - Over-exposed `pub`/`pub(crate)` items that are only called within their own defining file (should be private).
   - Over-exposed `pub mod` declarations whose items are never referenced externally.
3. Single Responsibility Principle & Cohesion:
   - Source files exceeding 800 lines in crates/*/src/ (referencing architecture rule exceptions).
   - Structs with excessive public fields (> 12) indicating potential "God Structs" or unencapsulated state.
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

def find_repo_root() -> Path:
    curr = Path(__file__).resolve().parent
    for p in [curr] + list(curr.parents):
        if (p / "Cargo.toml").exists() and (p / ".git").exists():
            return p
    return curr.parents[3]

REPO_ROOT = find_repo_root()
CRATES_DIR = REPO_ROOT / "crates"

# Crates or modules with special execution models (e.g. 65,536-entry function pointer dispatch tables)
EXEMPT_CRATES = {
    # m68000 instruction handlers are referenced via compile-time function pointer table
    "m68000",
    # test_runner is a dedicated test harness and verification crate, not production code
    "test_runner",
}

# Hardware register specification catalogs whose constants reflect physical silicon memory maps
EXEMPT_FILES = {
    "crates/config/src/registers.rs",
}

# External Host I/O boundary methods and hardware spec symbols per SKILL.md Section 4 Step 1
HOST_IO_AND_SPEC_SYMBOLS = {
    # Host Keyboard Input & Protocol
    "key_down", "key_up", "poll_reset", "queue_powerup_stream", "has_pending_scancodes",
    "SCANCODE_LOST_SYNC", "SCANCODE_SELF_TEST_FAILED",
    # Host Game Port / Mouse / Joystick Plugging
    "plug_port1", "plug_port2", "port1_mut", "port2_mut",
    # Host Parallel Port
    "write_data", "read_data",
    # Host Floppy Disk Drive Insertion
    "insert_disk", "eject_disk", "FORMATTED_DISK_BYTES", "decode_amiga_sector",
    # Host Debugger Controls & Breakpoints
    "DEFAULT_TARGET_ADDRESS", "toggle_pc_breakpoint", "has_pc_breakpoint", "check_watchpoint",
    "total_count", "save_state_to_json", "load_state_from_json", "run_until_breakpoint",
    # Host Display & Frame Buffer Extraction
    "is_in_display_window", "get_pixel", "frame_buffer", "frame_buffer_mut", "begin_frame", "end_frame", "extract_vamiga_raw_viewport",
    # Host Audio Sample Ring Buffer
    "pop_sample", "samples_available",
    # Host Real-Time Clock
    "set_time",
    # System Topologies & Save States / Synchronous Stepping
    "bare_512k", "expanded_power_user", "to_json", "step_cycles", "execute_blit",
    # Bus Query & Overlay Status
    "is_wait", "is_low_memory_overlay_active",
    # HRM Figure 6-9 DMA Slot Timing Specifications
    "HPOS_REFRESH_SLOTS", "HPOS_DISK_SLOTS", "HPOS_AUDIO_SLOTS", "HPOS_SPRITE_START", "HPOS_SPRITE_END",
    # Hardware Chip Interface Signals
    "trigger_flag_pin", "poll_pra_output", "led_transition", "stage_write",
    "set_disk_byte", "poll_dsksyn_irq",
    # Sprite pipeline evaluation
    "evaluate_pixel",
}

# Recognized architectural file-size exceptions per test_architecture_rules.rs
LINE_COUNT_EXCEPTIONS = {
    "crates/m68000/src/instructions/move_b.rs",
    "crates/m68000/src/instructions/move_w.rs",
    "crates/m68000/src/instructions/move_l.rs",
    "crates/m68000/src/micro/dispatch_table.rs",
    "crates/m68000/src/micro/step_execution.rs",
}

# Standard trait and lifecycle boilerplate methods to ignore
BOILERPLATE_NAMES = {
    "new", "default", "clone", "fmt", "eq", "ne", "drop",
    "serialize", "deserialize", "from", "into", "as_ref",
    "step", "step_cck", "reset", "reset_warm",
}

RE_PUB_FN = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\b")
RE_PUB_CRATE_FN = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s*\(\s*crate\s*\)\s+(?:const\s+|unsafe\s+)?fn\s+([a-zA-Z0-9_]+)\b")
RE_PUB_CONST = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+const\s+([A-Z0-9_]+)\b")
RE_PUB_STRUCT = re.compile(r"^\s*(?:#\[.*?\]\s*)*pub\s+struct\s+([a-zA-Z0-9_]+)\b")
RE_PUB_MOD = re.compile(r"^\s*pub\s+mod\s+([a-zA-Z0-9_]+)\s*;")


def build_file_corpus():
    """Indexes all production (.rs in crates/*/src/) and test (.rs in crates/*/tests/) files."""
    prod_files = []
    test_files = []

    for rs_file in CRATES_DIR.rglob("*.rs"):
        parts = rs_file.parts
        if "tests" in parts:
            test_files.append(rs_file)
        elif "src" in parts:
            prod_files.append(rs_file)

    return prod_files, test_files


def collect_declared_symbols(crate_dir):
    """Collects declared public and pub(crate) functions, constants, and structs."""
    src_dir = crate_dir / "src"
    if not src_dir.exists():
        return []

    declared = []
    for rs_file in src_dir.rglob("*.rs"):
        rel_path = str(rs_file.relative_to(REPO_ROOT)).replace("\\", "/")
        if rel_path in EXEMPT_FILES:
            continue
        try:
            content = rs_file.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        for line_num, line in enumerate(content.splitlines(), 1):
            # pub fn
            m_pub_fn = RE_PUB_FN.match(line)
            if m_pub_fn:
                name = m_pub_fn.group(1)
                if name not in BOILERPLATE_NAMES and not name.startswith("_"):
                    declared.append({
                        "kind": "fn",
                        "visibility": "pub",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

            # pub(crate) fn
            m_crate_fn = RE_PUB_CRATE_FN.match(line)
            if m_crate_fn:
                name = m_crate_fn.group(1)
                if name not in BOILERPLATE_NAMES and not name.startswith("_"):
                    declared.append({
                        "kind": "fn",
                        "visibility": "pub(crate)",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

            # pub const
            m_const = RE_PUB_CONST.match(line)
            if m_const:
                name = m_const.group(1)
                if not name.startswith("_"):
                    declared.append({
                        "kind": "const",
                        "visibility": "pub",
                        "name": name,
                        "file": rs_file,
                        "line": line_num,
                        "crate": crate_dir.name,
                    })
                continue

    return declared


def count_symbol_references(symbol_name, files, def_file, def_line):
    """Counts references to symbol_name across files, excluding definition and re-exports."""
    pattern = re.compile(rf"\b{re.escape(symbol_name)}\b")
    callers = []

    for file_path in files:
        try:
            text = file_path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        if symbol_name not in text:
            continue

        for line_num, line in enumerate(text.splitlines(), 1):
            if file_path == def_file and line_num == def_line:
                continue
            if "pub use " in line and symbol_name in line:
                continue
            if pattern.search(line):
                callers.append((file_path, line_num))

    return callers


def scan_dead_and_zombie_code(target_crate=None):
    """Detects completely dead code (0 callers) and test-only zombie code."""
    prod_files, test_files = build_file_corpus()
    all_declared = []

    for crate_dir in sorted(CRATES_DIR.iterdir()):
        if not crate_dir.is_dir() or not (crate_dir / "Cargo.toml").exists():
            continue
        if target_crate and crate_dir.name != target_crate:
            continue
        if crate_dir.name in EXEMPT_CRATES:
            continue

        all_declared.extend(collect_declared_symbols(crate_dir))

    completely_dead = []
    test_only_zombies = []

    for sym in all_declared:
        name = sym["name"]
        def_file = sym["file"]
        def_line = sym["line"]

        prod_callers = count_symbol_references(name, prod_files, def_file, def_line)
        test_callers = count_symbol_references(name, test_files, def_file, def_line)

        sym_info = {
            "name": name,
            "kind": sym["kind"],
            "visibility": sym["visibility"],
            "crate": sym["crate"],
            "file": str(def_file.relative_to(REPO_ROOT)),
            "line": def_line,
            "prod_callers_count": len(prod_callers),
            "test_callers_count": len(test_callers),
            "test_callers": [(str(f.relative_to(REPO_ROOT)), l) for f, l in test_callers[:3]],
        }

        if len(prod_callers) == 0 and len(test_callers) == 0:
            if name not in HOST_IO_AND_SPEC_SYMBOLS:
                completely_dead.append(sym_info)
        elif len(prod_callers) == 0 and len(test_callers) > 0:
            if name not in HOST_IO_AND_SPEC_SYMBOLS:
                test_only_zombies.append(sym_info)

    return completely_dead, test_only_zombies


def scan_least_visibility(target_crate=None):
    """Detects symbols with over-broad visibility (pub instead of pub(crate) or private)."""
    prod_files, test_files = build_file_corpus()
    all_declared = []

    for crate_dir in sorted(CRATES_DIR.iterdir()):
        if not crate_dir.is_dir() or not (crate_dir / "Cargo.toml").exists():
            continue
        if target_crate and crate_dir.name != target_crate:
            continue
        if crate_dir.name in EXEMPT_CRATES:
            continue

        all_declared.extend(collect_declared_symbols(crate_dir))

    visibility_leaks = []

    for sym in all_declared:
        if sym["visibility"] != "pub":
            continue

        name = sym["name"]
        def_file = sym["file"]
        def_line = sym["line"]
        defining_crate = sym["crate"]

        prod_callers = count_symbol_references(name, prod_files, def_file, def_line)
        test_callers = count_symbol_references(name, test_files, def_file, def_line)

        if len(prod_callers) == 0:
            continue

        # Check if callers are strictly within the defining crate
        external_callers = []
        internal_callers = []
        same_file_callers = []

        for f, l in prod_callers:
            crate_name = f.parts[f.parts.index("crates") + 1] if "crates" in f.parts else None
            if crate_name != defining_crate:
                external_callers.append((f, l))
            else:
                internal_callers.append((f, l))
                if f == def_file:
                    same_file_callers.append((f, l))

        if len(external_callers) == 0:
            # If callers exist only within the same file, recommend private fn
            if len(internal_callers) == len(same_file_callers):
                recommended = "private (fn)"
                reason = "Only called within defining file"
            else:
                recommended = "pub(crate)"
                reason = f"Only called within crate `{defining_crate}`"

            visibility_leaks.append({
                "name": name,
                "kind": sym["kind"],
                "crate": defining_crate,
                "file": str(def_file.relative_to(REPO_ROOT)),
                "line": def_line,
                "recommended": recommended,
                "reason": reason,
                "caller_count": len(internal_callers),
                "external_callers": len(external_callers),
            })

    return visibility_leaks


def scan_srp_and_cohesion(target_crate=None):
    """Scans for file-size and structural cohesion anomalies."""
    violations = []

    for crate_dir in sorted(CRATES_DIR.iterdir()):
        if not crate_dir.is_dir() or not (crate_dir / "Cargo.toml").exists():
            continue
        if target_crate and crate_dir.name != target_crate:
            continue

        src_dir = crate_dir / "src"
        if not src_dir.exists():
            continue

        for rs_file in src_dir.rglob("*.rs"):
            rel_path = str(rs_file.relative_to(REPO_ROOT)).replace("\\", "/")
            try:
                lines = rs_file.read_text(encoding="utf-8", errors="ignore").splitlines()
            except Exception:
                continue

            line_count = len(lines)

            # File size check (> 800 lines)
            if line_count > 800 and rel_path not in LINE_COUNT_EXCEPTIONS:
                violations.append({
                    "type": "file_size",
                    "crate": crate_dir.name,
                    "file": rel_path,
                    "metric": f"{line_count} lines (> 800 limit)",
                    "recommendation": "Decompose into cohesive submodules per file-size-and-cohesion.md",
                })

            # Check for structs with excessive public fields (> 12)
            struct_name = None
            struct_pub_fields = 0
            in_struct = False

            for line in lines:
                m_st = RE_PUB_STRUCT.match(line)
                if m_st:
                    if struct_name and struct_pub_fields > 12:
                        violations.append({
                            "type": "excessive_pub_fields",
                            "crate": crate_dir.name,
                            "file": rel_path,
                            "metric": f"`{struct_name}` has {struct_pub_fields} public fields",
                            "recommendation": "Encapsulate fields with methods or group into domain sub-structs",
                        })
                    struct_name = m_st.group(1)
                    struct_pub_fields = 0
                    in_struct = "{" in line
                    continue

                if in_struct:
                    if line.strip().startswith("}"):
                        if struct_name and struct_pub_fields > 12:
                            violations.append({
                                "type": "excessive_pub_fields",
                                "crate": crate_dir.name,
                                "file": rel_path,
                                "metric": f"`{struct_name}` has {struct_pub_fields} public fields",
                                "recommendation": "Encapsulate fields with methods or group into domain sub-structs",
                            })
                        in_struct = False
                        struct_name = None
                        struct_pub_fields = 0
                    elif re.match(r"^\s*pub\s+[a-zA-Z0-9_]+\s*:", line):
                        struct_pub_fields += 1

    return violations


def check_skills_catalog_sync():
    """Verifies that all skills in .agents/skills/ are documented in docs/ai_agents.md."""
    skills_dir = REPO_ROOT / ".agents" / "skills"
    ai_agents_doc = REPO_ROOT / "docs" / "ai_agents.md"

    if not skills_dir.exists() or not ai_agents_doc.exists():
        return {"actual_count": 0, "documented_count": 0, "issues": []}

    actual_skills = {p.name for p in skills_dir.iterdir() if p.is_dir() and (p / "SKILL.md").exists()}
    doc_content = ai_agents_doc.read_text(encoding="utf-8")
    documented_skills = set(re.findall(r"\[`([a-zA-Z0-9_-]+)`\]\(\.\./\.agents/skills/\1/SKILL\.md\)", doc_content))

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


def check_script_locality_and_governance():
    """
    Audits two-way script placement governance:
    1. Harness-to-Skill Locality: Scripts in tools/harness/ referenced by <= 1 skill/workflow
       (and not part of global pre-commit/pre-flight/rules) should be relocated to skills/<skill>/scripts/.
    2. Skill-to-Harness Promotion: Scripts inside a skill's scripts/ directory referenced by > 1
       distinct skill or workflow should be promoted to tools/harness/ to avoid cross-skill leakage.
    """
    harness_dir = REPO_ROOT / "tools" / "harness"
    skills_dir = REPO_ROOT / ".agents" / "skills"
    workflows_dir = REPO_ROOT / ".agents" / "workflows"

    UNIVERSAL_HARNESS_SCRIPTS = {
        "pre_flight.py",
        "run_tests.py",
        "check_polish.py",
        "check_test_coupling.py",
        "audit_api_coverage.py",
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


def check_workflow_and_skill_governance():
    """
    Audits two-way alignment between workflows, skills, and rules:
    1. Workflow-to-Skill Backing: Workflows in .agents/workflows/ must have a corresponding
       backing skill in .agents/skills/ or explicitly document valid skill references.
    2. Skill-to-Workflow Promotion Candidates: Top-level milestone and procedural skills
       that lack a user-facing slash command (/command) in .agents/workflows/ are flagged.
    3. Rule-to-Skill Governance: Active remediation rules must have companion skills,
       while passive invariant rules must remain lean without redundant skills.
    """
    workflows_dir = REPO_ROOT / ".agents" / "workflows"
    skills_dir = REPO_ROOT / ".agents" / "skills"
    rules_dir = REPO_ROOT / ".agents" / "rules"

    workflow_files = sorted(workflows_dir.glob("*.md")) if workflows_dir.exists() else []
    skill_dirs = [p for p in sorted(skills_dir.iterdir()) if p.is_dir() and (p / "SKILL.md").exists()] if skills_dir.exists() else []
    skill_names = {p.name for p in skill_dirs}

    workflow_issues = []
    # 1. Audit each workflow
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

    # 2. Audit skills for workflow promotion candidates
    # Milestone/procedural skills that orchestrate cross-cutting or multi-step operations
    PROCEDURAL_MILESTONE_SKILLS = {
        "compact-diary": "Milestone diary synthesis and compaction procedure",
        "sync-design-docs": "Design specification synchronization with git commit history",
        "roadmap-maintenance": "Milestone scorecard pruning and substrate-first roadmap update",
        "index-amiga-rag": "Qdrant vector database re-indexing and offline sidecar maintenance",
    }

    active_workflow_stems = {wf.stem for wf in workflow_files}
    promotion_candidates = []
    for skill_name, reason in sorted(PROCEDURAL_MILESTONE_SKILLS.items()):
        if skill_name in skill_names and skill_name not in active_workflow_stems:
            promotion_candidates.append({
                "skill": skill_name,
                "reason": reason,
            })

    # 3. Rule-to-Skill Governance
    ACTIVE_RULE_COMPANIONS = {
        "amiga-rag.md": {"index-amiga-rag"},
        "asset-descriptions.md": {"describe-diagram-assets"},
        "diary-maintenance.md": {"compact-diary"},
        "docs-maintenance.md": {"sync-design-docs", "obsidian-vault-linking"},
        "vault-linking-and-graph-integrity.md": {"obsidian-vault-linking"},
        "egui-best-practices.md": {"egui-vision-debugger", "capture-gui-screenshot"},
        "file-size-and-cohesion.md": {"refactor-split-module"},
        "git-commits.md": {"git-resolve-merge", "git-worktree"},
        "git-merge-commits.md": {"git-resolve-merge", "git-worktree"},
        "graphify.md": {"graphify"},
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
            missing = expected_skills - skill_names
            if missing:
                rule_issues.append({
                    "type": "missing_rule_skill",
                    "rule": rf.name,
                    "message": f"Active rule `{rf.name}` requires companion skill(s) {missing}, which are missing in .agents/skills/",
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

    new_content = "---\n" + "\n".join(new_fm_lines) + "\n---" + parts[2]
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
    """Updates last_synced_commit to HEAD and last_synced_date to today in the design doc."""
    design_dir = REPO_ROOT / "Obsidian" / "Amiga" / "Design"
    doc_path = design_dir / doc_name
    if not doc_path.exists() and not doc_name.endswith(".md"):
        doc_path = design_dir / f"{doc_name}.md"
    if not doc_path.exists():
        print(f"Error: Design document not found: {doc_name}", file=sys.stderr)
        return False

    h_res = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=REPO_ROOT, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    head_commit = h_res.stdout.strip()
    import datetime
    today = datetime.date.today().isoformat()

    if bump_markdown_checkpoint(doc_path, head_commit, today):
        print(f"[OK] Bumped checkpoint for {doc_path.name}: last_synced_commit = '{head_commit}', last_synced_date = '{today}'")
        return True
    else:
        print(f"Error: Failed to update frontmatter in {doc_path.name}", file=sys.stderr)
        return False


def main():
    parser = argparse.ArgumentParser(description="Audit dead code, minimum visibility leaks, SRP cohesion, skill catalog sync, and design doc sync.")
    parser.add_argument("--all", action="store_true", help="Run all audits (dead code, visibility, SRP, skills sync, script locality, design sync)")
    parser.add_argument("--dead-code", action="store_true", help="Run dead code & zombie scanner")
    parser.add_argument("--visibility", action="store_true", help="Run least visibility scanner")
    parser.add_argument("--srp", action="store_true", help="Run SRP and file cohesion checks")
    parser.add_argument("--skills", action="store_true", help="Audit skill catalog sync in docs/ai_agents.md")
    parser.add_argument("--scripts", action="store_true", help="Audit two-way script locality and harness governance")
    parser.add_argument("--governance", action="store_true", help="Audit workflow-skill symmetry and rule companion coverage")
    parser.add_argument("--design-sync", action="store_true", help="Audit design documentation sync with code commits")
    parser.add_argument("--design-diff", help="Show code diff since last_synced_commit for a design doc")
    parser.add_argument("--design-bump", help="Bump last_synced_commit to current HEAD for a design doc")
    parser.add_argument("--crate", help="Filter audit to a specific crate")
    parser.add_argument("--json", action="store_true", help="Output results as JSON")
    args = parser.parse_args()

    # Handle targeted actions first
    if args.design_diff:
        show_design_diff(args.design_diff)
        return

    if args.design_bump:
        bump_design_checkpoint(args.design_bump)
        return

    # Default to --all if no specific mode selected
    if not (args.all or args.dead_code or args.visibility or args.srp or args.skills or args.scripts or args.design_sync or args.governance):
        args.all = True

    dead, zombies = ([], [])
    if args.all or args.dead_code:
        dead, zombies = scan_dead_and_zombie_code(args.crate)

    vis_leaks = []
    if args.all or args.visibility:
        vis_leaks = scan_least_visibility(args.crate)

    srp_issues = []
    if args.all or args.srp:
        srp_issues = scan_srp_and_cohesion(args.crate)

    skill_audit = {"actual_count": 0, "documented_count": 0, "issues": []}
    if args.all or args.skills:
        skill_audit = check_skills_catalog_sync()

    script_issues = []
    if args.all or args.scripts:
        script_issues = check_script_locality_and_governance()

    gov_audit = {
        "workflow_count": 0,
        "skill_count": 0,
        "active_rule_count": 0,
        "passive_rule_count": 0,
        "workflow_issues": [],
        "promotion_candidates": [],
        "rule_issues": [],
    }
    if args.all or args.governance:
        gov_audit = check_workflow_and_skill_governance()

    design_audit = {"tracked_count": 0, "synced_count": 0, "drifted": [], "synced": []}
    if args.all or args.design_sync:
        design_audit = check_design_docs_sync()

    if args.json:
        out = {
            "dead_code": dead,
            "test_only_zombies": zombies,
            "visibility_leaks": vis_leaks,
            "srp_cohesion_issues": srp_issues,
            "skills_sync": skill_audit,
            "script_locality_issues": script_issues,
            "workflow_and_skill_governance": gov_audit,
            "design_docs_sync": design_audit,
        }
        print(json.dumps(out, indent=2))
        return

    if sys.stdout.encoding != "utf-8":
        try:
            sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass

    print("=" * 76)
    print(" AMIGA 500 EMULATOR: DEEP ARCHITECTURAL CODE QUALITY AUDIT")
    print("=" * 76)

    if args.all or args.dead_code:
        print(f"\n[1. DEAD CODE & ZOMBIE SCANNER]")
        print(f"  - Completely Dead: {len(dead)} symbol(s)")
        for s in dead[:10]:
            print(f"    * [{s['kind']}] {s['crate']}::{s['name']} -> {s['file']}:{s['line']}")
        if len(dead) > 10:
            print(f"    * ... and {len(dead) - 10} more")

        print(f"  - Test-Only Zombies: {len(zombies)} symbol(s) (unused by production code)")
        for s in zombies[:10]:
            callers_str = ", ".join(f"{c[0]}:{c[1]}" for c in s["test_callers"][:2])
            print(f"    * [{s['kind']}] {s['crate']}::{s['name']} -> {s['file']}:{s['line']} (tested in {callers_str})")
        if len(zombies) > 10:
            print(f"    * ... and {len(zombies) - 10} more")

    if args.all or args.visibility:
        print(f"\n[2. PRINCIPLE OF MINIMUM VISIBILITY (LEAST PRIVILEGE)]")
        print(f"  - Over-exposed `pub` items: {len(vis_leaks)} symbol(s)")
        for v in vis_leaks[:15]:
            print(f"    * {v['crate']}::{v['name']} ({v['file']}:{v['line']}) -> recommend `{v['recommended']}` ({v['reason']})")
        if len(vis_leaks) > 15:
            print(f"    * ... and {len(vis_leaks) - 15} more")

    if args.all or args.srp:
        print(f"\n[3. SINGLE RESPONSIBILITY & COHESION]")
        print(f"  - Structural anomalies: {len(srp_issues)} issue(s)")
        for issue in srp_issues:
            print(f"    * [{issue['type']}] {issue['file']}: {issue['metric']}")
            print(f"      -> {issue['recommendation']}")

    if args.all or args.skills:
        print(f"\n[4. AGENT SKILLS CATALOG SYNCHRONIZATION (docs/ai_agents.md)]")
        print(f"  - Active on-disk skills: {skill_audit['actual_count']}")
        print(f"  - Documented in docs/ai_agents.md: {skill_audit['documented_count']}")
        if not skill_audit["issues"]:
            print("  - Status: [PASS] 100% synchronized (all skills cataloged cleanly).")
        else:
            print(f"  - Status: [FAIL] {len(skill_audit['issues'])} discrepancy/ies found:")
            for iss in skill_audit["issues"]:
                print(f"    * {iss['message']}")

    if args.all or args.scripts:
        print(f"\n[5. TWO-WAY SCRIPT LOCALITY & HARNESS GOVERNANCE]")
        if not script_issues:
            print("  - Status: [PASS] All harness scripts are shared/universal, and all skill scripts are private.")
        else:
            print(f"  - Status: [WARN] {len(script_issues)} placement anomaly/ies detected:")
            for s_issue in script_issues:
                print(f"    * [{s_issue['type']}] {s_issue['location']}")
                print(f"      -> {s_issue['recommendation']}")

    if args.all or args.design_sync:
        print(f"\n[6. DESIGN DOCUMENTATION & CODE DRIFT DETECTION]")
        print(f"  - Tracked Design Specs: {design_audit['tracked_count']}")
        if not design_audit["drifted"]:
            print(f"  - Status: [PASS] All {design_audit['synced_count']} tracked specification(s) are in sync with HEAD.")
        else:
            print(f"  - Status: [DRIFT] {len(design_audit['drifted'])} specification(s) behind code HEAD:")
            for d in design_audit["drifted"]:
                if d.get("error"):
                    print(f"    * {d['file']}: {d['error']}")
                else:
                    paths_str = ", ".join(d["paths"])
                    print(f"    * {d['file']}: {d['count']} commit(s) behind in [{paths_str}] (synced at: {d['commit']})")
                    for c_log in d.get("log", []):
                        print(f"        - {c_log}")
            print("\n  -> Remediation: Inspect diff with --design-diff <doc>, update spec, then run --design-bump <doc>")

    if args.all or args.governance:
        print(f"\n[7. WORKFLOW & SKILL GOVERNANCE (TWO-WAY ALIGNMENT)]")
        print(f"  - Active Workflows (.agents/workflows/): {gov_audit['workflow_count']}")
        print(f"  - Active Skills (.agents/skills/): {gov_audit['skill_count']}")
        if gov_audit["workflow_issues"]:
            print(f"  - Workflow Issues: {len(gov_audit['workflow_issues'])}")
            for w_issue in gov_audit["workflow_issues"]:
                print(f"    * [{w_issue['type']}] {w_issue['message']}")
        else:
            print("  - Workflows: [PASS] All workflows have backing specialized skills.")

        if gov_audit["promotion_candidates"]:
            print(f"  - Skill -> Workflow Candidates ({len(gov_audit['promotion_candidates'])}):")
            for cand in gov_audit["promotion_candidates"]:
                print(f"    * `{cand['skill']}`: {cand['reason']}")
                print(f"      -> Recommendation: Provide slash command `/{cand['skill']}` in .agents/workflows/{cand['skill']}.md")
        else:
            print("  - Promotion Candidates: None (all procedural skills have matching workflows).")

        if gov_audit["rule_issues"]:
            print(f"  - Rule Coverage Issues: {len(gov_audit['rule_issues'])}")
            for r_issue in gov_audit["rule_issues"]:
                print(f"    * [{r_issue['type']}] {r_issue['message']}")
        else:
            print(f"  - Rules Symmetry: [PASS] All {gov_audit['active_rule_count']} active remediation rules have companion skills; {gov_audit['passive_rule_count']} passive invariant rules remain lean.")

    gov_issues_total = len(gov_audit['workflow_issues']) + len(gov_audit['rule_issues'])
    print("\n" + "=" * 76)
    print(f"Audit Summary: {len(dead)} dead, {len(zombies)} zombies, {len(vis_leaks)} visibility leaks, {len(srp_issues)} cohesion issues, {len(skill_audit['issues'])} skill sync issues, {len(script_issues)} script locality issues, {len(design_audit['drifted'])} design drift issues, {gov_issues_total} governance issues ({len(gov_audit['promotion_candidates'])} workflow candidates).")
    print("=" * 76)


if __name__ == "__main__":
    main()

