"""Expand decoder structural groups into provisional benchmark case candidates.

Every row still needs an executable fixture and path validation before timing.
Run the benchmark target with --inventory first to regenerate the input.
"""

import argparse
import csv
import json
from collections import Counter
from itertools import product
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DIRECTORY = ROOT / ".agent" / "tmp" / "benchmarks" / "inventory"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--directory", type=Path, default=DEFAULT_DIRECTORY)
args = parser.parse_args()
directory = args.directory.resolve()
with (directory / "structural_groups.csv").open(encoding="utf-8-sig", newline="") as source:
    MICROSTEP_GROUPS = list(csv.DictReader(source))
with (directory / "decoder_inventory.csv").open(encoding="utf-8-sig", newline="") as source:
    decoder_rows = list(csv.DictReader(source))
legal_opcodes = {
    int(row["opcode"], 16)
    for row in decoder_rows
    if row["decoder_status"] == "legal"
}


def semantic_subgroups(group):
    opcodes = [int(value, 16) for value in group["opcodes"].split("|")]
    first = opcodes[0]
    partitions = {}
    for opcode in opcodes:
        if 0x6200 <= first <= 0x6FFF or (first & 0xF0F8) == 0x50C8 or (
            (first & 0xF0C0) == 0x50C0 and ((first >> 3) & 7) != 1
        ):
            key = (opcode >> 8) & 15  # Condition-code mnemonic.
        elif 0x4880 <= first <= 0x48FF or 0x4C80 <= first <= 0x4CFF:
            if first in (0x4880, 0x48C0):  # EXT, not MOVEM.
                key = 0
            else:
                key = opcode & 0x0440  # MOVEM direction and size.
        else:
            key = 0
        partitions.setdefault(key, []).append(opcode)
    return [(f"{group['group_id']}-{key:03X}", values) for key, values in sorted(partitions.items())]


GROUPS = [subgroup for group in MICROSTEP_GROUPS for subgroup in semantic_subgroups(group)]
all_group_opcodes = [opcode for _, opcodes in GROUPS for opcode in opcodes]
assert len(all_group_opcodes) == len(set(all_group_opcodes)), "duplicate legal opcode"
assert set(all_group_opcodes) == legal_opcodes, "structural groups omit or add legal opcodes"


def ea_dimensions(opcode):
    family = opcode >> 12
    if family in (1, 2, 3):
        return [((opcode >> 3) & 7, opcode & 7), ((opcode >> 6) & 7, (opcode >> 9) & 7)]
    if family in (6, 7):
        return []
    if (opcode & 0xFF00) == 0x4E00 and opcode < 0x4E90:
        return []
    if family == 14 and ((opcode >> 6) & 3) != 3:
        return []
    return [((opcode >> 3) & 7, opcode & 7)]


def is_div(opcode):
    return (opcode & 0xF1C0) in (0x80C0, 0x81C0)


def is_chk(opcode):
    return (opcode & 0xF1C0) == 0x4180


def is_movem(opcode):
    return (opcode & 0xFB80) == 0x4880 and ((opcode >> 3) & 7) >= 2


def is_privileged(opcode):
    return (
        opcode in (0x4E70, 0x4E72, 0x4E73, 0x007C, 0x027C, 0x0A7C)
        or 0x4E60 <= opcode <= 0x4E6F
        or 0x46C0 <= opcode <= 0x46FF
    )


def direct_a7_selectors(opcode):
    family = opcode >> 12
    if family in (2, 3) and ((opcode >> 6) & 7) == 1:  # MOVEA
        return (9,)
    if family in (9, 13) and ((opcode >> 6) & 3) == 3:  # SUBA/ADDA
        return (9,)
    if family == 5 and ((opcode >> 6) & 3) != 3 and ((opcode >> 3) & 7) == 1:
        return (0,)  # ADDQ/SUBQ An
    if (opcode & 0xF1C0) == 0x41C0:  # LEA
        return (9,)
    if 0x4E50 <= opcode <= 0x4E5F or 0x4E60 <= opcode <= 0x4E6F:
        return (0,)  # LINK/UNLK/MOVE USP
    if (opcode & 0xF1F8) == 0xC148:  # EXG Ax, Ay
        return (9, 0)
    if (opcode & 0xF1F8) == 0xC188:  # EXG Dx, Ay
        return (0,)
    return ()


def a7_selectors(opcode):
    selectors = []
    family = opcode >> 12
    mode = (opcode >> 3) & 7
    size = (opcode >> 6) & 3
    direction = (opcode >> 8) & 1
    if mode == 1 and direction == 1 and size < 3 and family in (9, 11, 13):
        # SUBX, CMPM, ADDX use two predecrement/postincrement An operands.
        return [0, 9]
    if (opcode & 0xF1F8) in (0x8108, 0xC108):
        # SBCD/ABCD memory forms use two predecrement An operands.
        return [0, 9]
    for index, (mode, _) in enumerate(ea_dimensions(opcode)):
        if mode in (3, 4):
            selectors.append(0 if index == 0 else 9)
    selectors.extend(direct_a7_selectors(opcode))
    return selectors


def implicit_stack(opcode):
    return (
        (opcode & 0xFF00) == 0x6100  # BSR
        or 0x4E90 <= opcode <= 0x4EBF  # JSR
        or 0x4850 <= opcode <= 0x487F  # PEA, excluding SWAP
        or opcode in (0x4E75, 0x4E77, 0x4E73)  # RTS/RTR/RTE
        or 0x4E40 <= opcode <= 0x4E4F  # TRAP
        or 0x4E50 <= opcode <= 0x4E5F  # LINK/UNLK
    )


def paths(opcode):
    if 0x6200 <= opcode <= 0x6FFF:
        return ("taken", "untaken")
    if (opcode & 0xF0F8) == 0x50C8:
        condition = (opcode >> 8) & 15
        if condition == 0:  # DBT: condition is always true.
            return ("condition_true",)
        if condition == 1:  # DBF: condition is always false.
            return ("branch", "expired")
        return ("condition_true", "branch", "expired")
    if (opcode & 0xF0C0) == 0x50C0 and ((opcode >> 3) & 7) != 1:
        condition = (opcode >> 8) & 15
        if condition == 0:  # ST
            return ("condition_true",)
        if condition == 1:  # SF
            return ("condition_false",)
        return ("condition_true", "condition_false")
    if is_div(opcode):
        if (opcode & 0xF1C0) == 0x81C0:
            return ("valid_fast", "valid_slow", "overflow_positive", "overflow_negative", "divide_zero")
        return ("valid_fast", "valid_slow", "overflow", "divide_zero")
    if is_chk(opcode):
        return ("in_range", "above_bound", "negative")
    if opcode == 0x4E76:
        return ("no_trap", "trap")
    if (opcode & 0xF1C0) in (0xC0C0, 0xC1C0):
        return ("sparse_operand", "dense_operand")
    if opcode in (0x0140, 0x0180, 0x01C0, 0x0840, 0x0880, 0x08C0):
        return ("low_bit", "high_bit")
    if is_movem(opcode):
        return (
            "empty_mask",
            "single_non_a7",
            "single_a7",
            "multiple_non_a7",
            "multiple_a7",
        )
    if opcode >> 12 == 14 and ((opcode >> 6) & 3) != 3:
        if opcode & 0x20:
            return ("zero_count", "one_count", "large_count")
        return ("one_count", "eight_count")
    if opcode in (0x4E72, 0x4E73) or 0x46C0 <= opcode <= 0x46FF:
        return ("remain_supervisor", "enter_user")
    if opcode in (0x027C, 0x0A7C):
        return ("remain_supervisor", "enter_user")
    return ("normal",)


def path_raises_exception(opcode, path):
    return (
        is_div(opcode) and path == "divide_zero"
        or is_chk(opcode) and path in ("above_bound", "negative")
        or opcode == 0x4E76 and path == "trap"
        or 0x4E40 <= opcode <= 0x4E4F
    )


rows = []
summary = Counter()
for group_id, opcodes in GROUPS:
    opcode = opcodes[0]
    dimensions = ea_dimensions(opcode)
    index_count = sum(mode == 6 or mode == 7 and reg == 3 for mode, reg in dimensions)
    selectors = a7_selectors(opcode)
    for path in paths(opcode):
        mask_uses_a7 = is_movem(opcode) and path in ("single_a7", "multiple_a7")
        a7_roles = len(selectors) + int(mask_uses_a7)
        for index_choices in product(range(4), repeat=index_count):
            for a7_choices in product(range(2), repeat=len(selectors)):
                matching_opcodes = [
                    candidate for candidate in opcodes
                    if all(((candidate >> shift) & 7 == 7) == bool(chosen) for shift, chosen in zip(selectors, a7_choices))
                ]
                if not matching_opcodes:
                    raise ValueError(f"No opcode for {group_id} A7 choices {a7_choices}")
                representative_opcode = min(
                    matching_opcodes,
                    key=lambda candidate: (
                        # A short branch must use an even displacement and must
                        # not branch into its own extension or an odd address.
                        abs((candidate & 0xFF) - 2) if candidate >> 12 == 6 else 0,
                        (
                            abs(((candidate >> 9) & 7) - (1 if path == "one_count" else 0))
                            if candidate >> 12 == 14
                            and not candidate & 0x20
                            and path in ("one_count", "eight_count")
                            else 0
                        ),
                        ((candidate & 7) - 3) ** 2 + (((candidate >> 9) & 7) - 4) ** 2,
                        candidate,
                    ),
                )
                if mask_uses_a7:
                    a7_choices += (1,)
                if is_privileged(opcode):
                    modes = ("supervisor",)
                    category = "privileged_allowed"
                elif implicit_stack(opcode) or path_raises_exception(opcode, path):
                    modes = ("supervisor", "user")
                    category = "stack_or_exception"
                elif any(a7_choices):
                    modes = ("supervisor", "user")
                    category = "ordinary"
                else:
                    modes = ("supervisor",)
                    category = "ordinary"
                for mode in modes:
                    case_id = f"{group_id}:{path}:I{''.join(map(str,index_choices)) or '-'}:A{''.join(map(str,a7_choices)) or '-'}:{mode[0].upper()}"
                    rows.append((case_id, group_id, f"{representative_opcode:04X}", path, str(index_choices), str(a7_choices), mode, category))
                    summary[category] += 1
    if is_privileged(opcode):
        case_id = f"{group_id}:privilege_violation:I-:A-:U"
        rows.append((case_id, group_id, f"{opcode:04X}", "privilege_violation", "()", "()", "user", "privilege_violation"))
        summary["privilege_violation"] += 1

out = directory / "case_candidates.csv"
assert len({row[0] for row in rows}) == len(rows)
with out.open("w", encoding="utf-8", newline="") as file:
    writer = csv.writer(file)
    writer.writerow(("case_id", "semantic_group", "representative_opcode", "path", "index_choices", "a7_choices", "cpu_mode", "category"))
    writer.writerows(rows)

print("static groups", len(MICROSTEP_GROUPS))
print("semantic subgroups", len(GROUPS))
print("case candidates", sum(summary.values()))
print("breakdown", dict(summary))
print("candidate rows", len(rows))
print("artifact", out)
json_path = directory / "case_candidates.json"
columns = ("case_id", "semantic_group", "representative_opcode", "path", "index_choices", "a7_choices", "cpu_mode", "category")
json_path.write_text(
    json.dumps([dict(zip(columns, row)) for row in rows], indent=2) + "\n",
    encoding="utf-8",
)
print("artifact", json_path)

opcode_group = {
    opcode: group_id
    for group_id, opcodes in GROUPS
    for opcode in opcodes
}
cases_by_group = Counter(row[1] for row in rows)
selected_opcodes = {int(row[2], 16) for row in rows}
coverage_path = directory / "opcode_coverage.csv"
with coverage_path.open("w", encoding="utf-8", newline="") as file:
    writer = csv.writer(file)
    writer.writerow(("opcode", "decoder_status", "semantic_group", "candidate_count", "coverage_role"))
    for row in decoder_rows:
        opcode = int(row["opcode"], 16)
        if opcode in legal_opcodes:
            group_id = opcode_group[opcode]
            assert cases_by_group[group_id] > 0, f"legal opcode {opcode:04X} lacks a benchmark group"
            role = "selected_representative" if opcode in selected_opcodes else "same_semantic_group_register_or_operand_alias"
            writer.writerow((f"{opcode:04X}", "legal", group_id, cases_by_group[group_id], role))
        else:
            writer.writerow((f"{opcode:04X}", row["decoder_status"], "", 0, "decoder_exception_excluded"))
assert len(selected_opcodes) <= len(rows)
print("legal opcode coverage", len(opcode_group), "selected encodings", len(selected_opcodes))
print("artifact", coverage_path)
