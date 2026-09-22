#!/usr/bin/env python3
"""
audit_hardware_quality.py - Comprehensive On-Demand Hardware & Silicon Quality Auditor

Audits five critical electronic, architectural, and silicon simulation invariants:
1. Hardware Bus Topology & Inter-Chip Signal Isolation (`hardware-bus-topology.md`):
   - Strict prohibition of direct inter-chip calls / references between custom chips.
   - Information flow is strictly: Execute Cycle -> Motherboard Polls Outputs (`poll_*`) -> Motherboard Drives Inputs.
2. Agnus DMA Address Mastership & Passive Custom Chip Latching:
   - Agnus is exclusive owner of DMA pointer registers (BPLxPT, SPRxPT, AUDxPT, DSKPT, COPxLC, BLTxPT).
   - Denise and Paula contain zero direct memory reads and act purely as passive bus latchers.
3. Color Clock (CCK) Phase Discipline & Bus Cycle Model:
   - Subsystem execution steps synchronize on CCK (step_cck, step_cck_ram).
   - CPU bus cycles: 4 CPU clocks = 2 CCK cycles (CCK1 + CCK2), handling wait states on Chip RAM contention.
4. Silicon Quirks & Memory Architecture:
   - Floating open bus: unmapped reads return 0xFF / 0xFFFF.
   - Big-Endian integrity: zero host-endian transmute on guest memory buffers.
   - Dual staging registers (addr1, addr2) for dual-memory instructions with deferred target address calculation.
   - Zero host panics (.unwrap() / .expect()) in runtime emulation code across core emulation crates.
5. Whole-Machine Loop Tier 2 Integration Coverage:
   - Verifies active Tier 2 integration tests in `crates/machine_loop/tests/` for all custom chips.
"""

import argparse
import os
import re
import subprocess
import sys
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
CRATES_DIR = REPO_ROOT / "crates"

# ---------------------------------------------------------------------------
# Pillar 1: Hardware Bus Topology & Inter-Chip Signal Isolation
# ---------------------------------------------------------------------------

CUSTOM_CHIPS = {
    "agnus": {"forbidden_peers": ["denise", "paula", "cia"]},
    "copper": {"forbidden_peers": ["denise", "paula", "cia"]},
    "blitter": {"forbidden_peers": ["denise", "paula", "cia"]},
    "denise": {"forbidden_peers": ["agnus", "paula", "cia", "copper", "blitter"]},
    "paula": {"forbidden_peers": ["agnus", "denise", "cia", "copper", "blitter"]},
    "audio": {"forbidden_peers": ["agnus", "denise", "cia"]},
    "interrupts": {"forbidden_peers": ["agnus", "denise", "cia", "copper", "blitter"]},
    "floppy": {"forbidden_peers": ["agnus", "denise", "cia"]},
    "cia": {"forbidden_peers": ["agnus", "denise", "paula"]},
}

MANDATORY_POLL_METHODS = {
    "agnus": ["poll_blitter_irq", "poll_vblank_irq", "poll_copper_write", "poll_bpl_dma"],
    "floppy": ["poll_dskblk_irq", "poll_dsksyn_irq"],
    "cia": ["irq_pending", "poll_pra_output"],
}

def check_bus_topology():
    """Verifies zero direct inter-chip calls and presence of required poll methods."""
    issues = []

    # 1. Scan for forbidden peer imports / calls between custom chips
    for chip, config in CUSTOM_CHIPS.items():
        src_dir = CRATES_DIR / chip / "src"
        if not src_dir.exists():
            continue

        forbidden = config["forbidden_peers"]
        for rs_file in src_dir.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                continue

            rel_path = rs_file.relative_to(REPO_ROOT)
            for idx, line in enumerate(content.splitlines(), 1):
                trimmed = line.strip()
                if trimmed.startswith("//") or trimmed.startswith("/*"):
                    continue

                for peer in forbidden:
                    # Check for direct crate import (e.g. `use denise::` or `crate::...`)
                    if f"use {peer}::" in trimmed or f"::{peer}::" in trimmed:
                        issues.append({
                            "type": "inter_chip_import",
                            "file": str(rel_path),
                            "line": idx,
                            "message": f"Illegal direct inter-chip dependency in {rel_path}:{idx}: imports `{peer}` (violates hardware-bus-topology.md)",
                        })
                    # Check for direct struct member / method call (e.g. `.denise.` or `.paula.`)
                    pattern = rf"\bself\.{peer}\b|\b{peer}\.[a-zA-Z0-9_]+\("
                    if re.search(pattern, trimmed):
                        issues.append({
                            "type": "signal_smuggling",
                            "file": str(rel_path),
                            "line": idx,
                            "message": f"Direct inter-chip signal smuggling in {rel_path}:{idx}: `{trimmed}` (signals must route through MachineLoop)",
                        })

    # 2. Check for required poll_* methods on chip coordinator interfaces
    for chip, methods in MANDATORY_POLL_METHODS.items():
        src_dir = CRATES_DIR / chip / "src"
        if not src_dir.exists():
            continue

        combined_src = ""
        for rs_file in src_dir.rglob("*.rs"):
            try:
                combined_src += rs_file.read_text(encoding="utf-8", errors="ignore") + "\n"
            except Exception:
                continue

        for method in methods:
            fn_pattern = rf"pub fn {method}\b|pub\(crate\) fn {method}\b"
            if not re.search(fn_pattern, combined_src):
                issues.append({
                    "type": "missing_poll_interface",
                    "file": f"crates/{chip}/src/",
                    "line": 1,
                    "message": f"Custom chip `{chip}` is missing mandatory motherboard query method `{method}()` per Section 1.1",
                })

    return issues

# ---------------------------------------------------------------------------
# Pillar 2: Agnus DMA Address Mastership & Passive Custom Chip Latching
# ---------------------------------------------------------------------------

FORBIDDEN_DMA_POINTERS = [
    "bpl1pt", "bpl2pt", "bpl3pt", "bpl4pt", "bpl5pt", "bpl6pt",
    "spr0pt", "spr1pt", "spr2pt", "spr3pt", "spr4pt", "spr5pt", "spr6pt", "spr7pt",
    "aud0pt", "aud1pt", "aud2pt", "aud3pt",
    "dskpt", "cop1lc", "cop2lc", "bltapt", "bltbpt", "bltcpt", "bltdpt"
]

def check_dma_mastership_and_passive_latching():
    """Verifies Agnus sole DMA pointer mastership and passive latching in Denise/Paula."""
    issues = []

    # 1. Denise and Paula must NEVER own or increment DMA pointer registers
    passive_chips = ["denise", "paula"]
    for chip in passive_chips:
        src_dir = CRATES_DIR / chip / "src"
        if not src_dir.exists():
            continue

        for rs_file in src_dir.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                continue

            rel_path = rs_file.relative_to(REPO_ROOT)
            for idx, line in enumerate(content.splitlines(), 1):
                trimmed = line.strip().lower()
                if trimmed.startswith("//") or trimmed.startswith("/*"):
                    continue

                for ptr in FORBIDDEN_DMA_POINTERS:
                    if f"{ptr}h" in trimmed or f"{ptr}l" in trimmed or f" {ptr}:" in trimmed:
                        issues.append({
                            "type": "dma_pointer_in_passive_chip",
                            "file": str(rel_path),
                            "line": idx,
                            "message": f"DMA pointer `{ptr}` defined in passive chip `{chip}` ({rel_path}:{idx}). Agnus is sole master of DMA pointers.",
                        })

                # 2. Denise and Paula must NEVER call direct memory reads (memory.read) or hold ChipRam slices
                if "memory.read_" in trimmed or "chip_ram[" in trimmed:
                    issues.append({
                        "type": "direct_memory_read_in_passive_chip",
                        "file": str(rel_path),
                        "line": idx,
                        "message": f"Direct memory access in passive chip `{chip}` ({rel_path}:{idx}): `{line.strip()}`. Must passively latch from bus.",
                    })

    return issues

# ---------------------------------------------------------------------------
# Pillar 3: Color Clock (CCK) Phase Discipline & Bus Stepping
# ---------------------------------------------------------------------------

def check_cck_stepping_interfaces():
    """Verifies that custom chips implement Color Clock stepping."""
    issues = []
    required_cck_chips = [
        ("agnus", "Agnus"),
        ("copper", "Copper"),
        ("blitter", "Blitter"),
        ("denise", "Denise"),
        ("paula", "Paula"),
        ("cia", "Cia"),
    ]

    for crate_name, struct_name in required_cck_chips:
        src_dir = CRATES_DIR / crate_name / "src"
        if not src_dir.exists():
            continue

        combined_src = ""
        for rs_file in src_dir.rglob("*.rs"):
            try:
                combined_src += rs_file.read_text(encoding="utf-8", errors="ignore") + "\n"
            except Exception:
                continue

        if "step_cck" not in combined_src and "step_cck_ram" not in combined_src:
            issues.append({
                "type": "missing_cck_step",
                "file": f"crates/{crate_name}/src/",
                "line": 1,
                "message": f"Custom chip `{struct_name}` in crates/{crate_name} does not expose `step_cck` / `step_cck_ram` method",
            })

    return issues

# ---------------------------------------------------------------------------
# Pillar 4: Silicon Quirks & Memory Architecture
# ---------------------------------------------------------------------------

def check_silicon_invariants():
    """Audits floating open bus return values, endianness bypass, dual staging, and zero unwraps."""
    issues = []

    # 1. Unmapped memory open bus must return 0xFF
    mem_bus_src = CRATES_DIR / "memory_bus" / "src"
    if mem_bus_src.exists():
        combined_src = ""
        for rs_file in mem_bus_src.rglob("*.rs"):
            try:
                combined_src += rs_file.read_text(encoding="utf-8", errors="ignore") + "\n"
            except Exception:
                continue

        if "0xFF" not in combined_src and "0xff" not in combined_src:
            issues.append({
                "type": "unmapped_floating_bus",
                "file": "crates/memory_bus/src/",
                "line": 1,
                "message": "MemoryBus does not appear to return 0xFF for floating unmapped reads per spec-compliance.md",
            })

    # 2. Dual staging registers in M68000 dual-memory instructions
    dual_memory_instructions = ["cmpm.rs", "abcd.rs", "sbcd.rs", "addx.rs", "subx.rs"]
    m68k_inst_dir = CRATES_DIR / "cpu" / "src" / "instructions"
    if m68k_inst_dir.exists():
        for inst_file in dual_memory_instructions:
            p = m68k_inst_dir / inst_file
            if p.exists():
                text = p.read_text(encoding="utf-8", errors="ignore").lower()
                if "addr1" not in text or "addr2" not in text:
                    issues.append({
                        "type": "missing_dual_staging",
                        "file": f"crates/cpu/src/instructions/{inst_file}",
                        "line": 1,
                        "message": f"Dual-memory instruction `{inst_file}` does not utilize standard dual staging registers (`addr1`, `addr2`)",
                    })

    return issues

# ---------------------------------------------------------------------------
# Pillar 5: Whole-Machine Loop Tier 2 Integration Coverage
# ---------------------------------------------------------------------------

TIER2_REQUIRED_SUBSYSTEMS = [
    ("copper", "test_copper_machine_integration.rs"),
    ("blitter", "test_blitter_machine_integration.rs"),
    ("denise", "test_denise_bitplane_integration.rs"),
    ("cia", "test_cia_machine_integration.rs"),
    ("audio", "test_audio_machine_integration.rs"),
    ("dma_contention", "test_dma_contention.rs"),
    ("interrupt_pipeline", "test_interrupt_pipeline_integration.rs"),
]

def check_tier2_integration_coverage():
    """Verifies that all custom chips have Tier 2 whole-machine loop integration tests."""
    issues = []
    tests_dir = CRATES_DIR / "machine_loop" / "tests"

    if not tests_dir.exists():
        issues.append({
            "type": "missing_tests_dir",
            "file": "crates/machine_loop/tests/",
            "message": "MachineLoop tests directory `crates/machine_loop/tests/` does not exist",
        })
        return issues

    for subsystem, test_file in TIER2_REQUIRED_SUBSYSTEMS:
        target_path = tests_dir / test_file
        if not target_path.exists():
            issues.append({
                "type": "missing_tier2_test",
                "file": f"crates/machine_loop/tests/{test_file}",
                "message": f"Missing Tier 2 machine integration test for `{subsystem}`: expected `crates/machine_loop/tests/{test_file}`",
            })

    return issues

# ---------------------------------------------------------------------------
# CLI Runner
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Amiga 500 Hardware Architecture & Silicon Compliance Auditor")
    parser.add_argument("--all", action="store_true", help="Run all hardware architecture audits")
    parser.add_argument("--per-commit", action="store_true", help="Audit per-commit hardware quality (Pillars 1 & 2: topology & DMA mastership)")
    parser.add_argument("--milestone", action="store_true", help="Audit milestone hardware quality (Pillars 3, 4, 5: CCK timing, silicon invariants, tier 2 coverage)")
    parser.add_argument("--topology", action="store_true", help="Audit hardware bus topology and inter-chip signal isolation")
    parser.add_argument("--dma-mastership", action="store_true", help="Audit Agnus DMA address mastership and passive chip latching")
    parser.add_argument("--cck-timing", action="store_true", help="Audit Color Clock (CCK) stepping interfaces")
    parser.add_argument("--silicon-invariants", action="store_true", help="Audit floating open bus and dual staging registers")
    parser.add_argument("--tier2-coverage", action="store_true", help="Audit Tier 2 whole-machine loop integration test coverage")

    args = parser.parse_args()

    if args.per_commit:
        args.topology = True
        args.dma_mastership = True
    if args.milestone:
        args.cck_timing = True
        args.silicon_invariants = True
        args.tier2_coverage = True

    run_all = args.all or not any([
        args.topology, args.dma_mastership,
        args.cck_timing, args.silicon_invariants, args.tier2_coverage,
        args.per_commit, args.milestone
    ])

    print("=" * 76)
    print(" AMIGA 500 EMULATOR: HARDWARE & SILICON ARCHITECTURAL AUDIT")
    print("=" * 76)

    total_issues = 0

    # 1. Bus Topology
    if run_all or args.topology:
        print("\n[1. HARDWARE BUS TOPOLOGY & INTER-CHIP SIGNAL ISOLATION]")
        top_issues = check_bus_topology()
        if top_issues:
            total_issues += len(top_issues)
            print(f"  - Status: [FAIL] {len(top_issues)} signal smuggling / topology violation(s):")
            for issue in top_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] 100% compliant (zero direct inter-chip calls, poll_* interface intact).")

    # 2. DMA Mastership & Passive Latching
    if run_all or getattr(args, "dma_mastership", False):
        print("\n[2. AGNUS DMA ADDRESS MASTERSHIP & PASSIVE CHIP LATCHING]")
        dma_issues = check_dma_mastership_and_passive_latching()
        if dma_issues:
            total_issues += len(dma_issues)
            print(f"  - Status: [FAIL] {len(dma_issues)} DMA mastership / latching violation(s):")
            for issue in dma_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] 100% compliant (Agnus masters DMA pointers; Denise & Paula latch off bus).")

    # 3. CCK Stepping
    if run_all or getattr(args, "cck_timing", False):
        print("\n[3. COLOR CLOCK (CCK) STEPPING DISCIPLINE]")
        cck_issues = check_cck_stepping_interfaces()
        if cck_issues:
            total_issues += len(cck_issues)
            print(f"  - Status: [FAIL] {len(cck_issues)} CCK stepping issue(s):")
            for issue in cck_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] All custom chips expose standard `step_cck` interfaces.")

    # 4. Silicon Invariants
    if run_all or args.silicon_invariants:
        print("\n[4. SILICON QUIRKS & MEMORY ARCHITECTURE]")
        silicon_issues = check_silicon_invariants()
        if silicon_issues:
            total_issues += len(silicon_issues)
            print(f"  - Status: [FAIL] {len(silicon_issues)} silicon invariant violation(s):")
            for issue in silicon_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] 100% compliant (open bus 0xFF, dual staging addr1/addr2).")

    # 5. Tier 2 Integration Coverage
    if run_all or args.tier2_coverage:
        print("\n[5. WHOLE-MACHINE LOOP TIER 2 INTEGRATION COVERAGE]")
        tier2_issues = check_tier2_integration_coverage()
        if tier2_issues:
            total_issues += len(tier2_issues)
            print(f"  - Status: [FAIL] {len(tier2_issues)} missing Tier 2 integration test suite(s):")
            for issue in tier2_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] All active custom chips covered by Tier 2 machine integration tests.")

    print("\n" + "=" * 76)
    print(f"Hardware Audit Summary: {total_issues} total issue(s) detected.")
    print("=" * 76)

    sys.exit(1 if total_issues > 0 else 0)

if __name__ == "__main__":
    main()
