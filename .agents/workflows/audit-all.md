---
name: audit-all
description: Execute comprehensive quality audit master suite sequentially across Rust code health, Obsidian documentation sync, physical hardware compliance, and bidirectional semantic parity
---

# Workflow: Comprehensive Quality Audit Suite (audit-all)

Use this workflow to execute the complete suite of quality auditing systems across the emulator in one coordinated sequence:
1. **Rust Code Quality:** `python tools/harness/audit_code_quality.py --all`
2. **Documentation & Agent Governance:** `python tools/harness/audit_docs_quality.py --all`
3. **Hardware Silicon Compliance:** `python tools/harness/audit_hardware_quality.py --all`
4. **Bidirectional Semantic Parity:** `/audit-semantic-parity` (inference audit on modified or coordinator subsystems)
5. **Pre-Flight Quality Gate:** `python tools/harness/pre_flight.py`

---

## 1. Zero-Parameter Run (`/audit-all`)

When invoked without parameters, the agent executes the full suite in order:

### Step 1: Execute Rust Code Quality Audit
```powershell
python tools/harness/audit_code_quality.py --all
```
*Validates:* Dead code, test zombies, visibility leaks, SRP source file limits ($\le 800$ lines), method inlining, macro/generic prohibitions, and 1:1 test parity.

### Step 2: Execute Documentation & Governance Quality Audit
```powershell
python tools/harness/audit_docs_quality.py --all
```
*Validates:* Obsidian design spec sync, vault link integrity, document size ceilings (`AGENTS.md` $\le 14$ KB), skills catalog sync, script locality, workflow symmetry, YAML frontmatter, design spec rules delegation, and semantic documentation-to-code parity (registers, memory map, crate topology, signals, quirks).

### Step 3: Execute Hardware Silicon Compliance Audit
```powershell
python tools/harness/audit_hardware_quality.py --all
```
*Validates:* Motherboard bus topology, Agnus DMA mastership, passive custom chip latching, zero signal smuggling, CCK phase execution, floating open bus `$FF`, Big-Endian safety, and Tier 2 integration test coverage.

### Step 4: Execute Bidirectional Semantic Parity Audit
```text
/audit-semantic-parity [subsystem]
```
*Validates:* 3-vector bidirectional semantic alignment between living Rust implementation (`crates/<crate>/src/`) and Obsidian design specifications (`Obsidian/Amiga/Design/<Spec>.md`):
- **Forward Parity (Blind Spots):** Identifies code mechanics, registers, bitfields, counters, or edge cases missing from documentation.
- **Reverse Parity (Ghost Features):** Identifies speculative claims, obsolete pseudo-code, or unsimulated hardware modes in specifications.
- **Silicon Rigor:** Evaluates technical depth (hex bitmasks, CCK1/CCK2 phase timing, Inverted Pyramid structure).
*Target Selection:* Audits recently modified subsystems from `git status` / `git diff --stat`, or the primary subsystem coordinator (`agnus`, `paula`, or `interrupts`).

### Step 5: Execute Pre-Flight Quality Gate
```powershell
python tools/harness/pre_flight.py
```
*Validates:* Code formatting (`cargo fmt`), change-coupling gate, public API coverage, and all 20 automated architecture tests in `test_architecture_rules.rs`.

### Step 6: Execute Verbal Double-Check (Anti-Drift Conscience)
Before delivering the final verdict, explicitly review the **5 Heuristic Conscience Questions**:
1. 🧠 **Spec Freshness:** Did recent code changes alter chip behavior or registers without updating `Obsidian/Amiga/Design/*.md`?
2. 🚫 **Anti-Nudge (`structural-root-cause.md`):** Are all beam coordinates and delays silicon-verified rather than empirical $\pm 1$ / $\pm 2$ symptom patches?
3. 🔬 **Assertion Integrity (`unit-testing-policy.md`):** Do tests genuinely exercise register side-effects, or are they vacuous assertions?
4. 📢 **Spec Escalation (`spec-compliance.md`):** Were conflicts between external test vectors and internal specs escalated to the user before editing code?
5. 🧹 **Clean-Break Refactoring (`clean-break-refactoring.md`):** Were old methods and legacy shims completely deleted?

---

## 2. Pillar Coverage Matrix

| Quality Audit System | CLI / Command | Primary Invariants Verified |
| :--- | :--- | :--- |
| **Rust Code Quality** | `tools/harness/audit_code_quality.py` | Dead code, zombie tests, visibility leaks, SRP limits, inlining annotations, macro/generic bans, dedicated test parity |
| **Docs & Governance** | `tools/harness/audit_docs_quality.py` | Obsidian design sync, vault link integrity, rule size ceilings, skills catalog sync, script locality, rules delegation, semantic doc-to-code parity |
| **Hardware Compliance** | `tools/harness/audit_hardware_quality.py` | Motherboard bus topology, Agnus DMA mastership, passive latching, CCK stepping, open bus `$FF`, Big-Endian safety, Tier 2 tests |
| **Semantic Parity** | `/audit-semantic-parity` | Bidirectional 3-vector inference parity: forward blind spots (undocumented code), reverse ghost features (spec drift), and silicon depth score |

---

## 3. Consolidated Reporting Contract

Conclude execution with the unified quality dashboard:

```markdown
### 🛡️ Comprehensive Quality Audit Dashboard (audit-all)
- **Rust Code Quality (`audit_code_quality.py`):** [PASS | <count> issues]
- **Documentation & Governance (`audit_docs_quality.py`):** [PASS | <count> issues]
  * Semantic Doc-to-Code Double-Check: [PASS - registers, memory map, crate topology, signals, quirks]
- **Hardware Silicon Compliance (`audit_hardware_quality.py`):** [PASS | <count> issues]
- **Bidirectional Semantic Parity (`audit-semantic-parity`):** [PASS | <score>% - 0 blind spots, 0 ghost features]
- **Pre-Flight Quality Gate (`pre_flight.py`):** [PASS | <count> issues]
- **Verbal Double-Check Conscience Review:** [CONFIRMED - 5/5 heuristics verified]
- **Overall Verdict:** [PASS - 0 violations detected | REMEDIATION REQUIRED]
```

---

## 4. Remediation Protocols
If any pillar reports failures:
1. **Code Quality Failures:** Consult [`.agents/workflows/audit-code-quality.md`](audit-code-quality.md).
2. **Documentation Failures:** Consult [`.agents/workflows/audit-docs-quality.md`](audit-docs-quality.md) or execute `python tools/harness/audit_docs_quality.py --design-bump` for automatic spec bumping.
3. **Hardware Compliance Failures:** Consult [`.agents/workflows/audit-hardware-quality.md`](audit-hardware-quality.md) to inspect bus wiring and CCK timing.
4. **Semantic Parity Failures:** Consult [`.agents/workflows/audit-semantic-parity.md`](audit-semantic-parity.md). Enrich design specs for blind spots (`NEEDS_DOCS_ENRICHMENT`) or escalate spec-to-code conflicts to the user per `spec-compliance.md`.
