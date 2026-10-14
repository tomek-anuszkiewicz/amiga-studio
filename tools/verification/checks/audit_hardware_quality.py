#!/usr/bin/env python3
"""
Static checks for hardware bus topology and DMA address ownership.

Checks inter-chip dependency patterns, required motherboard poll interfaces,
and DMA pointer or direct memory access patterns in Denise and Paula.
Run without arguments; commit verification invokes this audit once.
"""

import re
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
    return curr.parents[2]

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
# CLI Runner
# ---------------------------------------------------------------------------

def main():
    if len(sys.argv) > 1:
        print("This auditor accepts no arguments. Run: python tools/verification/checks/audit_hardware_quality.py", file=sys.stderr)
        return 2

    print("=" * 76)
    print(" AMIGA 500 EMULATOR: HARDWARE TOPOLOGY & DMA AUDIT")
    print("=" * 76)

    total_issues = 0

    # 1. Bus Topology
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
    print("\n[2. AGNUS DMA ADDRESS MASTERSHIP & PASSIVE CHIP LATCHING]")
    dma_issues = check_dma_mastership_and_passive_latching()
    if dma_issues:
        total_issues += len(dma_issues)
        print(f"  - Status: [FAIL] {len(dma_issues)} DMA mastership / latching violation(s):")
        for issue in dma_issues:
            print(f"    * {issue['message']}")
    else:
        print("  - Status: [PASS] 100% compliant (Agnus masters DMA pointers; Denise & Paula latch off bus).")

    print("\n" + "=" * 76)
    print(f"Hardware Audit Summary: {total_issues} total issue(s) detected.")
    print("=" * 76)

    return 1 if total_issues > 0 else 0

if __name__ == "__main__":
    sys.exit(main())
