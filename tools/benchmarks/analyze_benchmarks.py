#!/usr/bin/env python3
"""
M68000 CPU Instruction Benchmark Analysis & Reporting Tool

Performs deep comparative analysis across benchmark results per
Obsidian/Amiga/Design/CPU Benchmark Analysis Guide.md:
1. Functional Family Symmetry (ADD vs SUB vs AND vs OR vs EOR vs CMP)
2. Addressing Mode Latency Ladder & Decomposition (AGU + Bus delta)
3. Operand Size Scaling (.B vs .W vs .L)
4. Heavy Instruction Efficiency & Emulation Tax (R_norm = Host ns / Amiga CCK)
5. Linear Regression (Host ns vs Amiga CCK, R^2 correlation)
6. Master Instruction Table
"""

import csv
import io
import math
import sys
from pathlib import Path
from typing import List, Dict, Any, Tuple

if sys.platform == "win32":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")


class BenchmarkRecord:
    def __init__(self, row: Dict[str, str]):
        self.timestamp = row.get("timestamp", "")
        self.mnemonic = row.get("mnemonic", "")
        self.variant = row.get("variant", "")
        self.mode = row.get("addressing_mode", row.get("mode", ""))
        self.category = row.get("category", "")
        self.opcode = row.get("opcode_hex", row.get("opcode", ""))
        self.amiga_cck = int(row.get("amiga_cck_cycles", row.get("amiga_cck", "0")))
        self.total_ops = int(row.get("total_guest_instructions", row.get("total_ops", "0")))
        self.host_ms_median = float(row.get("host_duration_median_ms", row.get("host_ms_median", "0.0")))
        self.host_ns_op = float(row.get("host_ns_per_instruction", row.get("host_ns_op", "0.0")))
        self.host_ns_cck = float(row.get("host_ns_per_guest_cck", row.get("host_ns_cck", "0.0")))
        self.host_mips = float(row.get("host_mips", "0.0"))
        self.jitter_pct = float(row.get("host_jitter_pct", row.get("jitter_pct", "0.0")))
        self.anomaly = (row.get("anomaly_flag", row.get("anomaly", "false"))).lower() == "true"


def load_records(csv_path: Path) -> List[BenchmarkRecord]:
    records = []
    with open(csv_path, mode="r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            records.append(BenchmarkRecord(row))
    return records


def linear_regression(records: List[BenchmarkRecord]) -> Tuple[float, float, float]:
    """Computes linear regression: Host_ns = alpha * CCK + beta, returns (alpha, beta, r_squared)"""
    x = [r.amiga_cck for r in records]
    y = [r.host_ns_op for r in records]
    n = len(x)
    if n < 2:
        return 0.0, 0.0, 0.0

    mean_x = sum(x) / n
    mean_y = sum(y) / n

    ss_xx = sum((val - mean_x) ** 2 for val in x)
    ss_yy = sum((val - mean_y) ** 2 for val in y)
    ss_xy = sum((x[i] - mean_x) * (y[i] - mean_y) for i in range(n))

    if ss_xx == 0:
        return 0.0, mean_y, 0.0

    alpha = ss_xy / ss_xx
    beta = mean_y - alpha * mean_x

    if ss_yy == 0:
        r_squared = 1.0
    else:
        r_squared = (ss_xy ** 2) / (ss_xx * ss_yy)

    return alpha, beta, r_squared


def analyze_symmetry(records: List[BenchmarkRecord]) -> str:
    """Compares matching 16-bit register ALU operations (ADD, SUB, AND, OR, EOR, CMP)"""
    target_variants = [
        "ADD.W D1  D0",
        "SUB.W D1  D0",
        "AND.W D1  D0",
        "OR.W D1  D0",
        "EOR.W D1  D0",
        "CMP.W D1  D0",
        "ADDQ.W #4  D0",
        "SUBQ.W #4  D0",
    ]
    matched = [r for r in records if r.variant in target_variants]
    if not matched:
        return ""

    lines = []
    lines.append("### 1. Functional Family Symmetry (16-bit Register ALU)")
    lines.append("")
    lines.append("| Instruction | Variant | Mode | Amiga CCK | Host ns/op | Host MIPS | R_norm (ns/CCK) | Delta vs ADD.W |")
    lines.append("| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: |")

    add_w = next((r for r in matched if "ADD.W" in r.variant), None)
    base_ns = add_w.host_ns_op if add_w else matched[0].host_ns_op

    for r in matched:
        delta_pct = ((r.host_ns_op - base_ns) / base_ns) * 100.0 if base_ns > 0 else 0.0
        lines.append(
            f"| `{r.mnemonic}` | `{r.variant}` | {r.mode} | {r.amiga_cck} | "
            f"**{r.host_ns_op:.2f} ns** | {r.host_mips:.1f} | {r.host_ns_cck:.3f} | {delta_pct:+.1f}% |"
        )

    lines.append("")
    return "\n".join(lines)


def analyze_addressing_modes(records: List[BenchmarkRecord]) -> str:
    """Decomposes addressing mode ladder for MOVE.W and ADD.W"""
    lines = []
    lines.append("### 2. Addressing Mode Latency Ladder & Decomposition")
    lines.append("")
    lines.append("Isolates the effective addressing penalty: `ΔMode = Host_ns(Mode) - Host_ns(DataRegDirect)`.")
    lines.append("")
    lines.append("| Opcode & Mode | Syntax | Amiga CCK | Host ns/op | Isolated ΔMode (ns) | R_norm (ns/CCK) | Efficiency |")
    lines.append("| :--- | :--- | :---: | :---: | :---: | :---: | :--- |")

    # Target MOVE.W variants
    move_records = [r for r in records if r.mnemonic == "MOVE" and ".W" in r.variant]
    move_reg = next((r for r in move_records if r.mode == "DataRegDirect"), None)
    reg_ns = move_reg.host_ns_op if move_reg else 16.0

    for r in move_records:
        delta = r.host_ns_op - reg_ns
        delta_str = f"+{delta:.2f} ns" if delta > 0 else "0.00 ns (base)"
        eff = "Optimal" if r.host_ns_cck < 3.5 else "Moderate" if r.host_ns_cck < 4.2 else "Watch"
        lines.append(
            f"| `MOVE.W` {r.mode} | `{r.variant}` | {r.amiga_cck} | **{r.host_ns_op:.2f} ns** | {delta_str} | {r.host_ns_cck:.3f} | {eff} |"
        )

    lines.append("")
    return "\n".join(lines)


def analyze_size_scaling(records: List[BenchmarkRecord]) -> str:
    """Analyzes size scaling (.B vs .W vs .L) across operations"""
    lines = []
    lines.append("### 3. Operand Size Scaling Matrix (.B vs .W vs .L)")
    lines.append("")
    lines.append("| Operation Family | .B (Byte) | .W (Word) | .L (Long) | Word/Byte Ratio | Long/Word Ratio |")
    lines.append("| :--- | :---: | :---: | :---: | :---: | :---: |")

    families = ["MOVE", "ADD", "SUB", "AND", "OR", "CMP", "CLR", "TST", "NEG"]
    for fam in families:
        b_rec = next((r for r in records if r.mnemonic == fam and ".B" in r.variant and r.mode == "DataRegDirect"), None)
        w_rec = next((r for r in records if r.mnemonic == fam and ".W" in r.variant and r.mode == "DataRegDirect"), None)
        l_rec = next((r for r in records if r.mnemonic == fam and ".L" in r.variant and r.mode == "DataRegDirect"), None)

        if b_rec and w_rec and l_rec:
            wb_ratio = (w_rec.host_ns_op / b_rec.host_ns_op) if b_rec.host_ns_op > 0 else 1.0
            lw_ratio = (l_rec.host_ns_op / w_rec.host_ns_op) if w_rec.host_ns_op > 0 else 1.0
            lines.append(
                f"| `{fam}` Register | {b_rec.host_ns_op:.2f} ns ({b_rec.amiga_cck}c) | "
                f"{w_rec.host_ns_op:.2f} ns ({w_rec.amiga_cck}c) | "
                f"{l_rec.host_ns_op:.2f} ns ({l_rec.amiga_cck}c) | "
                f"{wb_ratio:.2f}x | {lw_ratio:.2f}x |"
            )

    lines.append("")
    return "\n".join(lines)


def analyze_heavy_instructions(records: List[BenchmarkRecord]) -> str:
    """Analyzes complex multicycle operations (TRAP, RTE, RTS, MOVEM, MUL, DIV)"""
    lines = []
    lines.append("### 4. Complex & System Instruction Diagnostics (Emulation Tax)")
    lines.append("")
    lines.append("Evaluates whether heavy instructions are host bottlenecks or simply reflect expected Amiga cycle density.")
    lines.append("")
    lines.append("| Instruction | Variant | Mode | Amiga CCK | Host Duration | Host ns/op | R_norm (ns/CCK) | Assessment |")
    lines.append("| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :--- |")

    target_heavy = ["TRAP", "RTE", "RTS", "RTR", "MOVEM", "MULU", "MULS", "DIVU", "DIVS", "CHK", "LINK", "UNLK"]
    matched = [r for r in records if r.mnemonic in target_heavy]

    # Sort descending by raw host duration
    matched.sort(key=lambda r: r.host_ns_op, reverse=True)

    for r in matched:
        if r.host_ns_cck <= 3.5:
            diag = "✅ Ideal (Cycle-Proportional)"
        elif r.host_ns_cck <= 4.2:
            diag = "⚡ Normal Host Dispersion"
        else:
            diag = "⚠️ Elevated Emulation Tax"

        lines.append(
            f"| `{r.mnemonic}` | `{r.variant}` | {r.mode} | {r.amiga_cck} | "
            f"{r.host_ms_median:.2f} ms | **{r.host_ns_op:.2f} ns** | **{r.host_ns_cck:.3f}** | {diag} |"
        )

    lines.append("")
    return "\n".join(lines)


def analyze_overall_statistics(records: List[BenchmarkRecord]) -> str:
    """Computes global linear regression, MIPS statistics, and distribution"""
    alpha, beta, r_squared = linear_regression(records)
    mips_values = [r.host_mips for r in records]
    r_norm_values = [r.host_ns_cck for r in records]

    median_mips = sorted(mips_values)[len(mips_values) // 2]
    max_mips = max(mips_values)
    min_mips = min(mips_values)

    median_r_norm = sorted(r_norm_values)[len(r_norm_values) // 2]
    mean_r_norm = sum(r_norm_values) / len(r_norm_values)

    lines = []
    lines.append("### 5. Global Linear Regression & Correlation Model")
    lines.append("")
    lines.append(f"- **Linear Model:** `Host_ns = {alpha:.3f} × Amiga_CCK + {beta:.2f}`")
    lines.append(f"- **Pearson Correlation ($R^2$):** **{r_squared:.4f}** ({r_squared * 100.0:.2f}% variance explained by hardware cycles)")
    lines.append(f"- **Marginal Cost per Amiga CCK (α):** **{alpha:.3f} ns / CCK**")
    lines.append(f"- **Fixed Dispatch Overhead (β):** **{beta:.2f} ns / instruction**")
    lines.append(f"- **Normalized Emulation Efficiency (R_norm):** Median = **{median_r_norm:.3f} ns/CCK**, Mean = **{mean_r_norm:.3f} ns/CCK**")
    lines.append(f"- **Host MIPS Throughput:** Median = **{median_mips:.1f} MIPS**, Range = **[{min_mips:.1f} - {max_mips:.1f}] MIPS**")
    lines.append(f"- **Total Specifications Evaluated:** **{len(records)}**")
    lines.append("")
    return "\n".join(lines)


def generate_master_table(records: List[BenchmarkRecord]) -> str:
    """Generates the full 108-instruction catalog table sorted by mnemonic and mode"""
    lines = []
    lines.append("### 6. Master Benchmark Catalog (All Evaluated Instructions)")
    lines.append("")
    lines.append("| ID | Mnemonic | Variant | Addressing Mode | Category | CCK | Host ns/op | Host MIPS | R_norm (ns/CCK) | Jitter |")
    lines.append("| :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: |")

    for i, r in enumerate(records, 1):
        lines.append(
            f"| {i:03d} | `{r.mnemonic}` | `{r.variant}` | {r.mode} | {r.category} | "
            f"{r.amiga_cck} | {r.host_ns_op:.2f} | {r.host_mips:.1f} | {r.host_ns_cck:.3f} | {r.jitter_pct:.2f}% |"
        )

    lines.append("")
    return "\n".join(lines)


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_benchmarks.py <PATH_TO_CSV> [--output <OUTPUT_MD>]")
        sys.exit(1)

    csv_path = Path(sys.argv[1])
    if not csv_path.exists():
        print(f"Error: File not found: {csv_path}")
        sys.exit(1)

    records = load_records(csv_path)
    if not records:
        print("Error: No benchmark records found in CSV.")
        sys.exit(1)

    report_sections = [
        f"# M68000 Instruction Benchmark Analysis Report\n\n**Source Dataset:** `{csv_path.name}` ({len(records)} specifications)\n",
        analyze_overall_statistics(records),
        analyze_symmetry(records),
        analyze_addressing_modes(records),
        analyze_size_scaling(records),
        analyze_heavy_instructions(records),
        generate_master_table(records),
    ]

    full_report = "\n".join(report_sections)

    if "--output" in sys.argv:
        out_idx = sys.argv.index("--output") + 1
        if out_idx < len(sys.argv):
            out_path = Path(sys.argv[out_idx])
            out_path.parent.mkdir(parents=True, exist_ok=True)
            out_path.write_text(full_report, encoding="utf-8")
            print(f"[*] Analysis report written to: {out_path}")

    # Also print to stdout
    print(full_report)


if __name__ == "__main__":
    main()
