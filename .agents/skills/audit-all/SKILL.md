---
name: audit-all
description: Execute comprehensive quality audit master suite covering Rust code health, Obsidian documentation sync, physical hardware compliance, and bidirectional semantic parity sequentially across workspace crates.
---

# Recipe: Unified Quality Audit Master Playbook (audit-all)

This skill provides an overarching execution procedure for the comprehensive Amiga 500 emulator quality audit suite. It orchestrates the specialized quality suites directly in sequence:
1. **`audit-code-quality`**: Rust code health, dead code, zombies, visibility leaks, SRP cohesion, and test parity.
2. **`audit-docs-quality`**: Obsidian design spec drift/sync, vault links, rule sizes, skills catalog, workflow symmetry, and Pillar 9 Semantic Doc-to-Code Double-Check (registers, memory map, crate topology, signals, quirks).
3. **`audit-hardware-quality`**: Motherboard bus topology, Agnus DMA mastership, passive latching, CCK stepping, open bus `$FF`, and Tier 2 integration tests.
4. **`audit-semantic-parity`**: Inference-driven bidirectional code-to-docs parity (blind spots, undocumented code) and docs-to-code parity (hallucinations, ghost features, spec drift).
5. **`verbal-double-check`**: Mandatory 5-point heuristic self-audit against rule forgetting, spec drift, and symptom nudging.

---

## 1. When to Trigger This Skill

- **Milestone & Sprint Closures:** Mandatory pre-commit / pre-release verification before tagging releases or completing major roadmap milestones.
- **Whole-Repository Health Check:** Whenever verifying overall repository integrity after large cross-cutting refactorings.
- **Continuous Integration & Pre-Commit Gates:** Serving as the master quality gate for the entire emulator project.

---

## 2. Sequential CLI Execution & Double-Check Protocol

Execute the quality suites, semantic parity evaluation, and the verbal double-check in order:

```powershell
# 1. Rust Code Quality
python tools/harness/audit_code_quality.py --all

# 2. Documentation & Agent Governance (includes Pillar 9 Semantic Double-Check)
python tools/harness/audit_docs_quality.py --all

# 3. Hardware Silicon Compliance
python tools/harness/audit_hardware_quality.py --all

# 4. Bidirectional Semantic Parity (Inference Audit)
# Run /audit-semantic-parity [subsystem] targeting recently modified crates or primary coordinators (agnus, paula, interrupts)

# 5. Pre-Flight Repository Gate
python tools/harness/pre_flight.py
```

### Step 6: The Verbal Double-Check (Anti-Drift Conscience)
Conclude every run by affirming the **5 Heuristic Questions**:
1. 🧠 **Spec Freshness:** Did recent code changes alter chip behavior or registers without updating `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge (`structural-root-cause.md`):** Are all beam coordinates and delays silicon-verified rather than empirical $\pm 1$ / $\pm 2$ symptom patches?
3. 🔬 **Assertion Integrity (`unit-testing-policy.md`):** Do tests genuinely exercise register side-effects, or are they vacuous assertions?
4. 📢 **Spec Escalation (`spec-compliance.md`):** Were conflicts between external test vectors and internal specs escalated to the user before editing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods and legacy shims completely deleted?

---

## 3. Pillar Reference & Delegation

When issues are reported during any stage of `audit-all`, delegate remediation to the dedicated specialized playbooks:

1. **Rust Code Quality Issues:**
   - Follow [`audit-code-quality`](../audit-code-quality/SKILL.md) and [`/audit-code-quality`](../../workflows/audit-code-quality.md).
2. **Documentation & Governance Issues:**
   - Follow [`audit-docs-quality`](../audit-docs-quality/SKILL.md) and [`/audit-docs-quality`](../../workflows/audit-docs-quality.md).
   - For auto-bumping design specs: `python tools/harness/audit_docs_quality.py --design-bump <doc>`.
   - For semantic discrepancies: `python tools/harness/audit_docs_quality.py --semantic-sync`.
3. **Hardware Silicon Compliance Issues:**
   - Follow [`audit-hardware-quality`](../audit-hardware-quality/SKILL.md) and [`/audit-hardware-quality`](../../workflows/audit-hardware-quality.md).
4. **Bidirectional Semantic Parity Issues:**
   - Follow [`audit-semantic-parity`](../audit-semantic-parity/SKILL.md) and [`/audit-semantic-parity`](../../workflows/audit-semantic-parity.md).
   - For documentation enrichment: add missing register bitfields, phase timing, and errata to `Obsidian/Amiga/Design/<Spec>.md`.
   - For code alignment / spec divergence: escalate conflicts to the user before modifying code per `spec-compliance.md`.
