---
name: audit-all
description: Execute unified quality audit across all three pillars - Rust code health, Obsidian documentation sync, and physical hardware compliance
---

# Workflow: Unified Quality Audit Triad (audit-all)

Use this workflow to execute the entire comprehensive triad of quality auditing systems across the emulator in one unified command:
1. **Rust Code Quality** (`tools/harness/audit_code_quality.py`)
2. **Documentation & Agent Governance** (`tools/harness/audit_docs_quality.py`)
3. **Hardware Silicon Compliance** (`tools/harness/audit_hardware_quality.py`)
4. **Pre-Flight Gate** (`tools/harness/pre_flight.py`)

---

## 1. Zero-Parameter Run (`/audit-all`)
When invoked without parameters:
1. **Execute Full Quality Audit Triad:**
   ```powershell
   python tools/harness/audit_all.py --all
   ```
2. **Execute Pre-Flight Quality Gate:**
   ```powershell
   python tools/harness/pre_flight.py
   ```

---

## 2. Targeted Quality Runs
- **Quiet Executive Dashboard:**
  ```powershell
  python tools/harness/audit_all.py --quiet
  ```
- **Code Quality Only:**
  ```powershell
  python tools/harness/audit_code_quality.py --all
  ```
- **Documentation & Governance Only:**
  ```powershell
  python tools/harness/audit_docs_quality.py --all
  ```
- **Hardware Silicon Compliance Only:**
  ```powershell
  python tools/harness/audit_hardware_quality.py --all
  ```
- **Triad + Pre-Flight Gate Combined:**
  ```powershell
  python tools/harness/audit_all.py --pre-flight
  ```

---

## 3. Pillar Coverage Breakdown

| Audit System | Tool | Core Invariants Verified |
| :--- | :--- | :--- |
| **Rust Code Quality** | `audit_code_quality.py` | Dead code, zombie tests, visibility leaks, SRP file size limits ($\le 800$ lines), method inlining, macro/generic prohibitions, 1:1 test parity |
| **Docs & Governance** | `audit_docs_quality.py` | Obsidian design sync, vault dual-layer linking, document ceilings (`AGENTS.md` $\le 14$ KB), skills catalog sync, script locality, workflow symmetry |
| **Hardware Compliance** | `audit_hardware_quality.py` | Motherboard bus topology, Agnus DMA mastership, passive latching, zero signal smuggling, CCK stepping, open bus `$FF`, Big-Endian safety, Tier 2 integration tests |

---

## 4. Remediation Protocols
If any pillar reports failures:
1. **Code Quality Failures:** Consult `.agents/workflows/audit-code-quality.md`.
2. **Documentation Failures:** Consult `.agents/workflows/audit-docs-quality.md` or execute `python tools/harness/audit_docs_quality.py --design-bump` for automatic spec bumping.
3. **Hardware Compliance Failures:** Consult `.agents/workflows/audit-hardware-quality.md` and verify physical timing/bus topology.
