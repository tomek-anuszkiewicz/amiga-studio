---
name: audit-all
description: Execute comprehensive unified quality audit triad covering Rust code health, Obsidian documentation sync, and physical hardware compliance in one coordinated command.
---

# Recipe: Unified Quality Audit Triad Playbook (audit-all)

This skill provides an overarching execution procedure for the comprehensive Amiga 500 emulator quality audit triad. It orchestrates the three specialized quality suites:
1. **`audit-code-quality`**: Rust code health, dead code, zombies, visibility leaks, SRP cohesion, and test parity.
2. **`audit-docs-quality`**: Obsidian design spec drift/sync, vault links, rule sizes, skills catalog, and workflow symmetry.
3. **`audit-hardware-quality`**: Motherboard bus topology, Agnus DMA mastership, passive latching, CCK stepping, open bus `$FF`, and Tier 2 integration tests.

---

## 1. When to Trigger This Skill

- **Milestone & Sprint Closures:** Mandatory pre-commit / pre-release verification before tagging releases or completing major roadmap milestones.
- **Whole-Repository Health Check:** Whenever verifying overall repository integrity after large cross-cutting refactorings.
- **Continuous Integration & Pre-Commit Gates:** Serving as the master quality gate for the entire emulator project.

---

## 2. CLI Execution Commands

### A. Full Triad Run (Default)
```powershell
python tools/harness/audit_all.py --all
```

### B. Quiet Executive Dashboard
```powershell
python tools/harness/audit_all.py --quiet
```

### C. Triad with Pre-Flight Quality Gate
```powershell
python tools/harness/audit_all.py --pre-flight
```

### D. Selective Sub-Suite Execution
```powershell
# Code Quality Only
python tools/harness/audit_all.py --code

# Documentation & Governance Only
python tools/harness/audit_all.py --docs

# Hardware Silicon Compliance Only
python tools/harness/audit_all.py --hardware
```

---

## 3. Pillar Reference & Delegation

When issues are reported by `audit-all`, delegate remediation to the dedicated specialized playbooks:

1. **Rust Code Quality Issues:**
   - Follow [`audit-code-quality`](../audit-code-quality/SKILL.md) and [`/audit-code-quality`](../../workflows/audit-code-quality.md).
2. **Documentation & Governance Issues:**
   - Follow [`audit-docs-quality`](../audit-docs-quality/SKILL.md) and [`/audit-docs-quality`](../../workflows/audit-docs-quality.md).
   - For auto-bumping design specs: `python tools/harness/audit_docs_quality.py --design-bump`.
3. **Hardware Silicon Compliance Issues:**
   - Follow [`audit-hardware-quality`](../audit-hardware-quality/SKILL.md) and [`/audit-hardware-quality`](../../workflows/audit-hardware-quality.md).
