---
name: audit-all
description: Execute comprehensive quality audit triad sequentially across Rust code health, Obsidian documentation sync, and physical hardware compliance
---

# Workflow: Comprehensive Quality Audit Triad (audit-all)

Use this workflow to execute the entire triad of quality auditing systems across the emulator in one coordinated sequence:
1. **Rust Code Quality:** `python tools/harness/audit_code_quality.py --all`
2. **Documentation & Agent Governance:** `python tools/harness/audit_docs_quality.py --all`
3. **Hardware Silicon Compliance:** `python tools/harness/audit_hardware_quality.py --all`
4. **Pre-Flight Quality Gate:** `python tools/harness/pre_flight.py`

---

## 1. Zero-Parameter Run (`/audit-all`)

When invoked without parameters, the agent executes the full triad in order:

### Step 1: Execute Rust Code Quality Audit
```powershell
python tools/harness/audit_code_quality.py --all
```
*Validates:* Dead code, test zombies, visibility leaks, SRP source file limits ($\le 800$ lines), method inlining, macro/generic prohibitions, and 1:1 test parity.

### Step 2: Execute Documentation & Governance Quality Audit
```powershell
python tools/harness/audit_docs_quality.py --all
```
*Validates:* Obsidian design spec sync, vault link integrity, document size ceilings (`AGENTS.md` $\le 14$ KB), skills catalog sync, script locality, workflow symmetry, YAML frontmatter, and design spec reflection in agent rules.

### Step 3: Execute Hardware Silicon Compliance Audit
```powershell
python tools/harness/audit_hardware_quality.py --all
```
*Validates:* Motherboard bus topology, Agnus DMA mastership, passive custom chip latching, zero signal smuggling, CCK phase execution, floating open bus `$FF`, Big-Endian safety, and Tier 2 integration test coverage.

### Step 4: Execute Pre-Flight Quality Gate
```powershell
python tools/harness/pre_flight.py
```
*Validates:* Code formatting (`cargo fmt`), change-coupling gate, public API coverage, and all 20 automated architecture tests in `test_architecture_rules.rs`.

---

## 2. Pillar Coverage Matrix

| Quality Audit System | CLI Tool | Primary Invariants Verified |
| :--- | :--- | :--- |
| **Rust Code Quality** | `tools/harness/audit_code_quality.py` | Dead code, zombie tests, visibility leaks, SRP limits, inlining annotations, macro/generic bans, dedicated test parity |
| **Docs & Governance** | `tools/harness/audit_docs_quality.py` | Obsidian design sync, vault link integrity, rule size ceilings, skills catalog sync, script locality, rules delegation |
| **Hardware Compliance** | `tools/harness/audit_hardware_quality.py` | Motherboard bus topology, Agnus DMA mastership, passive latching, CCK stepping, open bus `$FF`, Big-Endian safety, Tier 2 tests |

---

## 3. Consolidated Reporting Contract

Conclude execution with the unified quality dashboard:

```markdown
### 🛡️ Comprehensive Quality Audit Dashboard (audit-all)
- **Rust Code Quality (`audit_code_quality.py`):** [PASS | <count> issues]
- **Documentation & Governance (`audit_docs_quality.py`):** [PASS | <count> issues]
- **Hardware Silicon Compliance (`audit_hardware_quality.py`):** [PASS | <count> issues]
- **Pre-Flight Quality Gate (`pre_flight.py`):** [PASS | <count> issues]
- **Overall Verdict:** [PASS - 0 violations detected | REMEDIATION REQUIRED]
```

---

## 4. Remediation Protocols
If any pillar reports failures:
1. **Code Quality Failures:** Consult [`.agents/workflows/audit-code-quality.md`](audit-code-quality.md).
2. **Documentation Failures:** Consult [`.agents/workflows/audit-docs-quality.md`](audit-docs-quality.md) or execute `python tools/harness/audit_docs_quality.py --design-bump` for automatic spec bumping.
3. **Hardware Compliance Failures:** Consult [`.agents/workflows/audit-hardware-quality.md`](audit-hardware-quality.md) to inspect bus wiring and CCK timing.
