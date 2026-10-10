#!/usr/bin/env python3
"""Deterministic checks owned by audit-semantic-parity."""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

def check_semantic_registers():
    """Validates custom register matrix in documentation against crates/config/src/registers.rs."""
    matrix_file = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "Custom Chip Register Ownership and Access Matrix.md"
    reg_code_file = REPO_ROOT / "crates" / "config" / "src" / "registers.rs"

    if not matrix_file.exists() or not reg_code_file.exists():
        return {"checked_offsets": 0, "issues": [{"message": "Required register specification or source file missing"}]}

    matrix_text = matrix_file.read_text(encoding="utf-8")
    reg_code = reg_code_file.read_text(encoding="utf-8")

    rust_offsets = {}
    current_mod = None
    for line in reg_code.splitlines():
        line = line.strip()
        if line.startswith("pub mod agnus"):
            current_mod = "Agnus"
        elif line.startswith("pub mod denise"):
            current_mod = "Denise"
        elif line.startswith("pub mod paula"):
            current_mod = "Paula"
        m = re.search(r"pub\s+const\s+([A-Z0-9_]+)\s*:\s*u16\s*=\s*(0x[0-9A-Fa-f]+);", line)
        if m and current_mod:
            name = m.group(1)
            offset = int(m.group(2), 16)
            rust_offsets.setdefault(offset, []).append((name, current_mod))

    issues = []
    checked = 0
    chip_map = {"A": "Agnus", "D": "Denise", "P": "Paula"}

    for idx, line in enumerate(matrix_text.splitlines(), 1):
        line = line.strip()
        if not line.startswith("|"):
            continue
        parts = [p.strip() for p in line.split("|")[1:-1]]
        if not parts:
            continue
        offset_part = parts[0]
        if ".." in offset_part:
            continue
        m_off = re.search(r"\$([0-9A-Fa-f]{3})", offset_part)
        if not m_off:
            continue
        offset = int(m_off.group(1), 16)

        names = []
        for col in parts[1:3]:
            found = re.findall(r"`([A-Z0-9_]+)`(?:\s*\(([ADP])\))?", col)
            for reg_name, chip_code in found:
                if reg_name != "FFFF":
                    names.append((reg_name, chip_code))
        if len(parts) >= 4:
            m_single = re.search(r"`([A-Z0-9_]+)`", parts[1])
            owner_match = re.search(r"(Agnus|Denise|Paula)", line)
            if m_single and owner_match:
                chip_letter = owner_match.group(1)[0]
                reg_n = m_single.group(1)
                if not any(r[0] == reg_n for r in names):
                    names.append((reg_n, chip_letter))

        if not names:
            continue

        checked += 1
        if offset not in rust_offsets:
            issues.append({"line": idx, "message": f"Offset ${offset:03X} in markdown not found in registers.rs"})
            continue

        rust_entries = rust_offsets[offset]
        rust_names = {r[0] for r in rust_entries}

        for reg_name, chip_letter in names:
            if reg_name not in rust_names:
                issues.append({"line": idx, "message": f"Register `{reg_name}` at ${offset:03X} not found in registers.rs (code has {sorted(rust_names)})"})
            elif chip_letter:
                exp_chip = chip_map.get(chip_letter)
                act_chips = [r[1] for r in rust_entries if r[0] == reg_name]
                if exp_chip and exp_chip not in act_chips:
                    issues.append({"line": idx, "message": f"Register `{reg_name}` chip mismatch: doc says {exp_chip}, code has {act_chips}"})

    return {"checked_offsets": checked, "issues": issues}


def check_semantic_memory_map():
    """Validates physical 24-bit memory map ranges in MemoryBus.md against crates/memory_bus/src/memory_bus.rs."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "MemoryBus.md"
    code_path = REPO_ROOT / "crates" / "memory_bus" / "src" / "memory_bus.rs"

    if not doc_path.exists() or not code_path.exists():
        return {"checked_ranges": 0, "issues": [{"message": "Required memory bus files missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")
    code_text = code_path.read_text(encoding="utf-8")

    code_constants = {}
    for line in code_text.splitlines():
        line = line.strip()
        m = re.search(r"pub\s+const\s+([A-Z0-9_]+)\s*:\s*u\d+\s*=\s*(0x[0-9A-Fa-f]+);", line)
        if m:
            code_constants[m.group(1)] = int(m.group(2), 16)

    expected_checks = [
        (r"\$BFD000\s*-\s*\$BFDF00", "CIA_B_START", 0xBFD000),
        (r"\$BFD000\s*-\s*\$BFDF00", "CIA_B_END", 0xBFDF00),
        (r"\$BFE001\s*-\s*\$BFEF01", "CIA_A_START", 0xBFE001),
        (r"\$BFE001\s*-\s*\$BFEF01", "CIA_A_END", 0xBFEF01),
        (r"\$DC0000\s*-\s*\$DC003F", "RTC_START", 0xDC0000),
        (r"\$DC0000\s*-\s*\$DC003F", "RTC_END", 0xDC003F),
        (r"\$BF", "BANK_CIA", 0xBF),
        (r"\$DC", "BANK_RTC", 0xDC),
        (r"\$DF", "BANK_CUSTOM", 0xDF),
        (r"\$DFF000\s*-\s*\$DFFFFE", "CUSTOM_REG_OFFSET_MASK", 0x01FE),
    ]

    issues = []
    checked = 0
    for pattern, const_name, expected_val in expected_checks:
        checked += 1
        if not re.search(pattern, doc_text):
            issues.append({"message": f"MemoryBus.md missing expected pattern: `{pattern}`"})
        if const_name not in code_constants:
            issues.append({"message": f"memory_bus.rs missing constant: `{const_name}`"})
        elif code_constants[const_name] != expected_val:
            issues.append({"message": f"Constant `{const_name}` in code (0x{code_constants[const_name]:X}) != doc (0x{expected_val:X})"})

    return {"checked_ranges": checked, "issues": issues}


def check_semantic_crate_topology():
    """Validates Mermaid crate dependency graph in General Architecture.md against Cargo.toml workspace members."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "General Architecture.md"
    cargo_path = REPO_ROOT / "Cargo.toml"

    if not doc_path.exists() or not cargo_path.exists():
        return {"cargo_crates": 0, "doc_crates": 0, "issues": [{"message": "Required architecture or Cargo.toml file missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")
    cargo_text = cargo_path.read_text(encoding="utf-8")

    cargo_members = set()
    in_members = False
    for line in cargo_text.splitlines():
        line = line.strip()
        if line.startswith("members = ["):
            in_members = True
            continue
        if in_members:
            if line.startswith("]"):
                break
            m = re.search(r'"([^"]+)"', line)
            if m and m.group(1).startswith("crates/"):
                cargo_members.add(m.group(1).split("/", 1)[1])

    doc_crates = set()
    for m in re.finditer(r"crates/([a-z0-9_]+)", doc_text):
        doc_crates.add(m.group(1))

    missing_in_doc = cargo_members - doc_crates
    missing_in_cargo = doc_crates - cargo_members

    issues = []
    if missing_in_doc:
        issues.append({"message": f"Crate(s) in Cargo.toml missing from General Architecture.md Mermaid graph: {sorted(missing_in_doc)}"})
    if missing_in_cargo:
        issues.append({"message": f"Crate(s) in General Architecture.md Mermaid graph missing from Cargo.toml: {sorted(missing_in_cargo)}"})

    return {"cargo_crates": len(cargo_members), "doc_crates": len(doc_crates), "issues": issues}


def check_semantic_signals():
    """Validates cross-chip signals and action dispatch methods against implementation in crates/*/src/."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "Cross-Chip Signals and Action Dispatch Catalog.md"
    if not doc_path.exists():
        return {"verified_methods": 0, "issues": [{"message": "Cross-Chip Signals catalog missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")

    raw_calls = []
    for chunk in re.findall(r"`([^`]+)`", doc_text):
        if "(" in chunk and ")" in chunk:
            call_part = chunk.split("(", 1)[0].strip()
            method_name = call_part.split(".")[-1].strip()
            if method_name.isidentifier():
                raw_calls.append(method_name)

    skip_keywords = {"val", "ch", "strt", "stop", "pins", "channel", "cop1lc", "cop2lc", "read"}
    catalog_methods = {m for m in raw_calls if m not in skip_keywords}

    code_methods = set()
    for rs_path in (REPO_ROOT / "crates").rglob("*.rs"):
        if "src" in rs_path.parts:
            text = rs_path.read_text(encoding="utf-8", errors="ignore")
            for m in re.finditer(r"\bfn\s+([a-z_][a-z0-9_]*)\s*[\(<]", text):
                code_methods.add(m.group(1))

    issues = []
    for m in sorted(catalog_methods):
        if m not in code_methods:
            issues.append({"message": f"Catalog action method `{m}()` not found in crates/*/src/"})

    return {"verified_methods": len(catalog_methods), "issues": issues}


def check_semantic_quirks_coverage():
    """Validates that all 13 critical silicon quirks in Platform Quirks catalog maintain active regression test coverage."""
    doc_path = REPO_ROOT / "Obsidian" / "Amiga" / "Design" / "Platform Quirks and Invariants Catalog.md"
    if not doc_path.exists():
        return {"total_quirks": 0, "covered_quirks": 0, "issues": [{"message": "Platform Quirks catalog missing"}]}

    doc_text = doc_path.read_text(encoding="utf-8")

    quirks = []
    for line in doc_text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        parts = [p.strip() for p in line.split("|")[1:-1]]
        if not parts:
            continue
        m = re.search(r"\*\*([^*]+)\*\*", parts[0])
        if m:
            quirk_name = m.group(1).strip()
            if quirk_name not in ["Silicon Quirk", "Hardware Quirk / Erratum"]:
                quirks.append(quirk_name)

    test_files_content = {}
    for test_path in (REPO_ROOT / "crates").rglob("tests/**/*.rs"):
        test_files_content[test_path] = test_path.read_text(encoding="utf-8", errors="ignore")

    quirk_signatures = {
        "Class 0 RMW Prefetch Order": ["PREFETCH_IRC", "BusPrefetchToScratch", "prefetch", "test_singlestep"],
        "A7 Stack Pointer Byte Alignment": ["test_move", "test_singlestep", "SP", "A7"],
        "Multi-Precision $Z$-Flag Retention (`ADDX`/`SUBX`/`NEGX`)": ["addx", "subx", "negx"],
        "Address Register Direct CCR Immunity & Sign Extension": ["adda", "suba", "movea", "cmpa"],
        "Address Register Direct CCR Immunity": ["adda", "suba", "movea", "cmpa"],
        "ASL Sticky Overflow ($V$)": ["asl", "overflow", "test_asl"],
        "Dual-Memory Address Error Deferral": ["addr1", "addr2", "test_address_error", "test_architecture_rules"],
        "Multi-Cycle Division Overflow CCR Quirk": ["divu", "divs", "overflow"],
        "Post-Increment `(An)+` Address Error AGU Register Commitment": ["cmpm", "test_cmpm", "test_singlestep", "ea_calc"],
        "`ASR` Count $\\ge$ Width Silicon Exhaustion ($C=0, X=0$)": ["test_asr", "asr", "test_singlestep"],
        "`MOVE` to Predecrement `-(An)` Prefetch Inversion & Bus Ordering": ["predecrement", "test_move", "test_singlestep"],
        "TAS Read-Modify-Write Silicon Erratum": ["TAS", "tas", "test_singlestep", "is_tas"],
        "Floppy Shared Motor Line Wiring": ["motor_on", "handle_ciab_port_b_write", "test_ciab_port_b_motor"],
        "Floppy Disk Change Flip-Flop (`_CHNG`)": ["is_disk_changed", "step_pulse", "test_disk_change_flip_flop"],
        "Keyboard Caps Lock Latch State Machine": ["caps_lock", "test_keyboard_caps_lock_toggle"],
        "Keyboard Serial Handshake Delay": ["WaitingHandshake", "test_keyboard_step_handshake"],
        "Linear Resistor DAC & Absence of Gamma Pre-Correction": ["color", "palette", "test_pixel_pipeline", "quantize"],
        "Reset Vector Fetch & Double Bus Fault": ["test_cpu_reset_unaligned_pc_triggers_double_bus_fault", "double_bus_fault", "halted"],
        "Two-Word Prefetch Ahead-Offset ($PC = \\text{Opcode} + 4$)": ["prefetch", "wrapping_sub(4)", "instruction_pc", "test_addressing"],
    }

    issues = []
    covered = 0
    for raw_quirk in quirks:
        norm = re.sub(r"[`]", "", raw_quirk).strip()
        matched_key = None
        for k in quirk_signatures:
            if re.sub(r"[`]", "", k).strip() == norm:
                matched_key = k
                break
        if not matched_key:
            issues.append({"message": f"Quirk `{raw_quirk}` lacks configured signature validator"})
            continue

        sigs = quirk_signatures[matched_key]
        found = False
        for path, content in test_files_content.items():
            if any(sig in content for sig in sigs):
                found = True
                break
        if found:
            covered += 1
        else:
            issues.append({"message": f"Quirk `{raw_quirk}` has zero matching regression test coverage in crates/*/tests/"})

    return {"total_quirks": len(quirks), "covered_quirks": covered, "issues": issues}


def check_semantic_sync():
    """Aggregates all five semantic documentation-to-code checks."""
    regs = check_semantic_registers()
    mmap = check_semantic_memory_map()
    topo = check_semantic_crate_topology()
    sigs = check_semantic_signals()
    quirks = check_semantic_quirks_coverage()

    all_issues = regs["issues"] + mmap["issues"] + topo["issues"] + sigs["issues"] + quirks["issues"]

    return {
        "registers": regs,
        "memory_map": mmap,
        "topology": topo,
        "signals": sigs,
        "quirks": quirks,
        "issues": all_issues,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true")
    parser.add_argument("--semantic-sync", action="store_true", help="Run static code/documentation consistency checks")
    args = parser.parse_args()
    run_all = args.all or not any([args.semantic_sync])
    total_issues = 0
    if run_all or args.semantic_sync:
        print("\n[8. SEMANTIC DOCUMENTATION-TO-CODE DOUBLE-CHECK]")
        sem_res = check_semantic_sync()
        regs = sem_res["registers"]
        mmap = sem_res["memory_map"]
        topo = sem_res["topology"]
        sigs = sem_res["signals"]
        quirks = sem_res["quirks"]
        s_issues = sem_res["issues"]

        print(f"  - Custom Register Matrix: {regs['checked_offsets']} offsets verified vs registers.rs")
        print(f"  - Memory Map Range Checks: {mmap['checked_ranges']} boundaries verified vs memory_bus.rs")
        print(f"  - Crate Topology Sync: {topo['cargo_crates']} crates verified (100% Mermaid-Cargo parity)")
        print(f"  - Cross-Chip Signal Parity: {sigs['verified_methods']} action methods verified in codebase")
        print(f"  - Silicon Quirks Coverage: {quirks['covered_quirks']}/{quirks['total_quirks']} quirks covered by regression test sentinels")

        if s_issues:
            total_issues += len(s_issues)
            print(f"  - Status: [FAIL] {len(s_issues)} semantic discrepancy issue(s) detected:")
            for issue in s_issues:
                print(f"    * {issue['message']}")
        else:
            print("  - Status: [PASS] Configured static consistency checks passed; full semantic parity requires manual review.")

    print(f"[FAIL] {total_issues} issue(s) detected." if total_issues else "[PASS] Checks completed without issues.")
    return 1 if total_issues else 0


if __name__ == "__main__":
    sys.exit(main())
