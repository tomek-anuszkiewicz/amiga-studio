---
name: audit-hardware-quality
description: Run full-workspace hardware architectural and silicon fidelity audit covering bus topology, Agnus DMA mastership, passive chip latching, CCK timing, and silicon invariants
---

# Workflow: Hardware Architecture & Silicon Fidelity Audit

Use this workflow to audit the emulation core against physical Commodore Amiga 500 electrical bus architecture, silicon timing models, Agnus DMA address mastership, passive custom chip latching, and whole-machine integration test coverage.

---

## 1. Zero-Parameter Run (`/audit-hardware-quality`)
When invoked without parameters:
1. **Execute Universal Hardware Quality Audit:**
   ```powershell
   python tools/harness/audit_hardware_quality.py --all
   ```
2. **Execute Pre-Flight Quality Gate (includes automated architecture tests):**
   ```powershell
   python tools/harness/pre_flight.py
   ```

---

## 2. Targeted Audit Commands
- **Audit Hardware Bus Topology & Inter-Chip Signal Isolation:**
   ```powershell
   python tools/harness/audit_hardware_quality.py --topology
   ```
- **Audit Agnus DMA Address Mastership & Passive Custom Chip Latching:**
   ```powershell
   python tools/harness/audit_hardware_quality.py --dma-mastership
   ```
- **Audit Color Clock (CCK) Phase Stepping Interfaces:**
   ```powershell
   python tools/harness/audit_hardware_quality.py --cck-timing
   ```
- **Audit Silicon Memory Quirks (Open Bus `$FF`, Endianness, Dual Staging, Zero Panics):**
   ```powershell
   python tools/harness/audit_hardware_quality.py --silicon-invariants
   ```
- **Audit Whole-Machine Loop Tier 2 Integration Coverage:**
   ```powershell
   python tools/harness/audit_hardware_quality.py --tier2-coverage
   ```

---

## 3. Remediation Procedure
Follow the detailed playbooks in [`.agents/skills/audit-hardware-quality/SKILL.md`](../skills/audit-hardware-quality/SKILL.md):
1. **Signal Smuggling & Cross-Chip Calls:** Remove direct struct references or method calls between custom chips (`agnus`, `denise`, `paula`, `cia`). Route all coordination signals strictly through `MachineLoop` and `MemoryBus` via `poll_*` methods per [`.agents/rules/hardware-bus-topology.md`](../rules/hardware-bus-topology.md).
2. **DMA Mastership & Passive Latching Violations:** Ensure DMA pointers (`BPLxPT`, `SPRxPT`, `AUDxPT`, `DSKPT`, `COPxLC`, `BLTxPT`) are exclusively mastered by Agnus. Strip any direct `memory.read` calls or `ChipRam` slices from `Denise` and `Paula`, ensuring they passively latch data from the bus.
3. **Color Clock Phase Disciplines:** Ensure all custom chips advance on Color Clock phases via `step_cck()` or `step_cck_ram()`.
4. **Silicon Architecture Deviations:** Guarantee open bus reads return `$FF`/`$FFFF`, unaligned accesses raise Vector 3 Address Error, dual-memory instructions use `addr1`/`addr2` staging registers, and runtime emulation code is 100% free of `.unwrap()` and `.expect()`.
5. **Tier 2 Test Gaps:** Add missing machine integration tests under `crates/machine_loop/tests/` to verify multi-chip coordination.

---

## 4. Output Contract
Conclude with the standardized summary report:
```markdown
### ⚡ Hardware Architecture & Silicon Fidelity Audit Report
- **Bus Topology & Signal Isolation:** [PASS | <count> violations] (zero direct cross-chip calls)
- **Agnus DMA Mastership & Passive Latching:** [PASS | <count> violations] (Denise & Paula passive)
- **Color Clock (CCK) Stepping:** [PASS | <count> violations]
- **Silicon Quirks & Memory Architecture:** [PASS | <count> violations] (open bus $FF, dual staging addr1/addr2)
- **Whole-Machine Loop Tier 2 Integration:** [PASS | <count> missing suites]
- **Verification:** `pre_flight.py` (PASS)
```
