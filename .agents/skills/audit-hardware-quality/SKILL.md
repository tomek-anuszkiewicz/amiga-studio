---
name: audit-hardware-quality
description: Comprehensive hardware architectural and silicon fidelity audit covering bus topology, Agnus DMA mastership, passive chip latching, CCK timing, and silicon invariants.
---

# Recipe: Hardware Architecture & Silicon Fidelity Auditor Playbook

This skill provides a systematic audit of the emulator core against the physical Commodore Amiga 500 motherboard architecture and custom chip silicon contracts.

---

## 1. When to Trigger This Skill

- **Custom Chip Implementation & Refactoring:** Run when adding or modifying logic in `crates/agnus/`, `crates/denise/`, `crates/paula/`, `crates/cia/`, or `crates/machine_loop/`.
- **Bus Routing Changes:** Run when altering DMA allocation, interrupt line routing, or register latching.
- **Pre-Review Quality Gate:** Verify physical circuit fidelity before milestone sign-off.

---

## 2. The Five Hardware Quality Pillars

### Pillar 1: Hardware Bus Topology & Inter-Chip Signal Isolation
- **No Signal Smuggling:** Custom chips (`Agnus`, `Denise`, `Paula`, `CiaA`, `CiaB`, `Cpu`) must **never** hold direct pointers or invoke mutating methods directly on each other per [`.agents/rules/hardware-bus-topology.md`](../../rules/hardware-bus-topology.md).
- **Strict Flow Model:** Execution flow is strictly:
  $$\text{Execute Cycle} \longrightarrow \text{Motherboard Polls Outputs (`poll_*`)} \longrightarrow \text{Motherboard Drives Inputs}$$
- **Mandatory Poll Interfaces:**
  - `Agnus`: `poll_blitter_irq()`, `poll_vblank_irq()`, `poll_copper_write()`, `poll_bpl_dma()`
  - `Floppy`: `poll_dskblk_irq()`, `poll_dsksyn_irq()`
  - `CIAs`: `irq_pending()`, `poll_pra_output()`

### Pillar 2: Agnus DMA Address Mastership & Passive Custom Chip Latching
- **Sole DMA Address Master:** Agnus exclusively masters and increments all Chip RAM DMA pointers (`BPLxPT`, `SPRxPT`, `AUDxPT`, `DSKPT`, `COPxLC`, `BLTxPT`).
- **Passive Bus Latching:** `Denise` and `Paula` must **never** call `memory.read_*()` or hold slices to `ChipRam`. They receive DMA words purely by latching data off the shared data bus upon matching RGA strobes (`BPLxDAT`, `SPRxDAT`, `AUDxDAT`, `DSKDAT`).

### Pillar 3: Color Clock (CCK) Phase Discipline & Bus Cycle Model
- **CCK Synchronization:** Primary synchronization unit is the Color Clock (~3.54 MHz PAL / ~3.58 MHz NTSC).
- **Subsystem Step Methods:** Chips expose `step_cck()` or `step_cck_ram()`.
- **CPU Bus Cycle Relationship:** $1\ \text{M68000 bus cycle} = 4\ \text{CPU clocks} = 2\ \text{CCK cycles}\ (\text{CCK1} + \text{CCK2})$. Wait states stall the CPU without advancing its active micro-step ($C = C_0 + 2 \times \text{wait\_states}$).

### Pillar 4: Silicon Quirks & Memory Architecture
- **Floating Open Bus:** Unmapped memory reads return `$FF` / `$FFFF` (configurable via `set_unmapped_byte()`).
- **Big-Endian Safety:** Motorola 68000 is strictly Big-Endian. Never perform host-endian pointer casting or `transmute` on guest memory buffers.
- **Dual Staging Architecture:** Dual-memory instructions (`CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`) must use dedicated staging registers `state.micro.addr1` and `state.micro.addr2`, with destination address calculation deferred to CCK2 of the source read to preserve Address Error invariance.
- **Zero Host Panics on Guest Code:** Zero `.unwrap()` / `.expect()` in runtime emulation code.

### Pillar 5: Whole-Machine Loop Tier 2 Integration Coverage
- **Integration Mandate:** Every custom chip subsystem must maintain active integration tests under `crates/machine_loop/tests/` verifying multi-chip coordination:
  - `test_copper_machine_integration.rs`
  - `test_blitter_machine_integration.rs`
  - `test_denise_bitplane_integration.rs`
  - `test_cia_machine_integration.rs`
  - `test_audio_machine_integration.rs`
  - `test_dma_contention.rs`
  - `test_interrupt_pipeline_integration.rs`

---

## 3. CLI Audit Workflow

```powershell
# Run full hardware quality audit across all 5 pillars
python tools/harness/audit_hardware_quality.py --all

# Audit only bus topology and inter-chip signal isolation
python tools/harness/audit_hardware_quality.py --topology

# Audit Agnus DMA mastership and passive chip latching
python tools/harness/audit_hardware_quality.py --dma-mastership

# Audit Color Clock (CCK) stepping interfaces
python tools/harness/audit_hardware_quality.py --cck-timing

# Audit silicon quirks (open bus $FF, endianness, dual staging, zero unwraps)
python tools/harness/audit_hardware_quality.py --silicon-invariants

# Audit Tier 2 whole-machine loop integration test coverage
python tools/harness/audit_hardware_quality.py --tier2-coverage
```
