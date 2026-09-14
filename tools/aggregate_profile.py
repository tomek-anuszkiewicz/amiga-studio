#!/usr/bin/env python3
"""
Profile Results Aggregator for Amiga 500 Emulation Hot Paths.

Parses external sampling profiler outputs (samply / Firefox Profiler Gecko JSON,
collapsed/folded stack files, or sample logs) and computes execution time percentages
for canonical chip and module entry methods (e.g. `step_cck` across CPU, Agnus, Denise, Paula, CIAs).

Updates or verifies `tests/benchmarks/chipset_benchmark_baseline.json`.
"""

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

# Canonical entry method registry mapping method patterns to chip/module hierarchy
CANONICAL_ENTRIES = [
    {
        "name": "Cpu::step_cck",
        "module": "cpu",
        "pattern": re.compile(r"(m68000::.*Cpu::step_cck|Cpu::step_cck)", re.IGNORECASE),
        "submethods": [],
    },
    {
        "name": "Agnus::step_cck_ram",
        "module": "agnus",
        "pattern": re.compile(r"(agnus::.*Agnus::step_cck_ram|Agnus::step_cck_ram)", re.IGNORECASE),
        "submethods": [
            ("Copper::step_cck", re.compile(r"(copper::.*Copper::step_cck|Copper::step_cck)", re.IGNORECASE)),
            ("Blitter::step_cck_ram", re.compile(r"(blitter::.*Blitter::step_cck_ram|Blitter::step_cck_ram)", re.IGNORECASE)),
            ("DmaScheduler::arbitrate", re.compile(r"(DmaScheduler::arbitrate|agnus::.*arbitrate)", re.IGNORECASE)),
        ],
    },
    {
        "name": "Denise::step_cck",
        "module": "denise",
        "pattern": re.compile(r"(denise::.*Denise::step_cck|Denise::step_cck)", re.IGNORECASE),
        "submethods": [
            ("FrameBuilder::set_cck_pixels", re.compile(r"(frame_builder::.*set_cck_pixels|FrameBuilder::set_cck_pixels)", re.IGNORECASE)),
            ("SpriteEngine::step_cck", re.compile(r"(SpriteEngine::step_cck|denise::.*sprites)", re.IGNORECASE)),
        ],
    },
    {
        "name": "Paula::step_cck",
        "module": "paula",
        "pattern": re.compile(r"(paula::.*Paula::step_cck|Paula::step_cck)", re.IGNORECASE),
        "submethods": [
            ("Audio::step_cck", re.compile(r"(audio::.*Audio::step_cck|Audio::step_cck)", re.IGNORECASE)),
        ],
    },
    {
        "name": "Cia::step_cck",
        "module": "cia",
        "pattern": re.compile(r"(cia::.*Cia::step_cck|Cia::step_cck)", re.IGNORECASE),
        "submethods": [],
    },
    {
        "name": "FloppyController::step_cck",
        "module": "floppy",
        "pattern": re.compile(r"(floppy::.*FloppyController::step_cck|FloppyController::step_cck)", re.IGNORECASE),
        "submethods": [],
    },
    {
        "name": "Rtc::step_cck",
        "module": "rtc",
        "pattern": re.compile(r"(rtc::.*step_cck|Rtc.*::step_cck)", re.IGNORECASE),
        "submethods": [],
    },
]


def parse_folded_stacks(filepath: Path):
    """
    Parses folded stack format (func1;func2;func3 count).
    Returns dict mapping canonical method names to sample counts and submethods.
    """
    method_counts = defaultdict(float)
    submethod_counts = defaultdict(lambda: defaultdict(float))
    total_samples = 0.0

    with open(filepath, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.rsplit(maxsplit=1)
            if len(parts) != 2:
                continue
            stack, count_str = parts
            try:
                count = float(count_str)
            except ValueError:
                continue

            total_samples += count

            # Check which canonical methods appear in this stack
            frames = stack.split(";")
            for entry in CANONICAL_ENTRIES:
                matched_entry = False
                for frame in frames:
                    if entry["pattern"].search(frame):
                        method_counts[entry["name"]] += count
                        matched_entry = True
                        break

                if matched_entry and entry["submethods"]:
                    for sub_name, sub_pat in entry["submethods"]:
                        for frame in frames:
                            if sub_pat.search(frame):
                                submethod_counts[entry["name"]][sub_name] += count
                                break

    return method_counts, submethod_counts, total_samples


def parse_gecko_profile(filepath: Path):
    """
    Parses Firefox Profiler / Samply Gecko JSON format.
    Extracts threads, string tables, and samples.
    """
    with open(filepath, "r", encoding="utf-8") as f:
        data = json.load(f)

    method_counts = defaultdict(float)
    submethod_counts = defaultdict(lambda: defaultdict(float))
    total_samples = 0.0

    threads = data.get("threads", [])
    for thread in threads:
        string_table = thread.get("stringTable", [])
        frame_table = thread.get("frameTable", {})
        func_indices = frame_table.get("func", [])
        func_table = thread.get("funcTable", {})
        name_indices = func_table.get("name", [])
        samples = thread.get("samples", {})
        stack_indices = samples.get("stack", [])
        stack_table = thread.get("stackTable", {})
        prefix_indices = stack_table.get("prefix", [])
        frame_indices = stack_table.get("frame", [])

        # Helper to resolve frame string
        def resolve_frame_name(frame_idx):
            if frame_idx < len(func_indices):
                f_idx = func_indices[frame_idx]
                if f_idx < len(name_indices):
                    s_idx = name_indices[f_idx]
                    if s_idx < len(string_table):
                        return string_table[s_idx]
            return ""

        # Map each stack index to list of frame names
        stack_cache = {}

        def get_stack_frames(s_idx):
            if s_idx is None or s_idx < 0:
                return []
            if s_idx in stack_cache:
                return stack_cache[s_idx]
            frames = []
            curr = s_idx
            while curr is not None and 0 <= curr < len(frame_indices):
                fr_idx = frame_indices[curr]
                name = resolve_frame_name(fr_idx)
                if name:
                    frames.append(name)
                curr = prefix_indices[curr]
            stack_cache[s_idx] = frames
            return frames

        # Accumulate samples
        for s_idx in stack_indices:
            total_samples += 1.0
            frames = get_stack_frames(s_idx)
            for entry in CANONICAL_ENTRIES:
                matched_entry = False
                for frame in frames:
                    if entry["pattern"].search(frame):
                        method_counts[entry["name"]] += 1.0
                        matched_entry = True
                        break

                if matched_entry and entry["submethods"]:
                    for sub_name, sub_pat in entry["submethods"]:
                        for frame in frames:
                            if sub_pat.search(frame):
                                submethod_counts[entry["name"]][sub_name] += 1.0
                                break

    return method_counts, submethod_counts, total_samples


def build_methods_profile(method_counts, submethod_counts, total_samples):
    """
    Computes percentage distribution for methods and nested submethods.
    """
    # Total time accounted for across entry methods
    entry_total = sum(method_counts.values())
    denom = entry_total if entry_total > 0 else (total_samples if total_samples > 0 else 1.0)

    result = {}
    for entry in CANONICAL_ENTRIES:
        name = entry["name"]
        count = method_counts.get(name, 0.0)
        pct = (count / denom) * 100.0 if denom > 0 else 0.0

        item = {
            "percent": round(pct, 1),
            "module": entry["module"],
        }

        if entry["submethods"] and name in submethod_counts:
            sub_dict = {}
            for sub_name, _ in entry["submethods"]:
                sub_count = submethod_counts[name].get(sub_name, 0.0)
                sub_pct = (sub_count / denom) * 100.0 if denom > 0 else 0.0
                sub_dict[sub_name] = round(sub_pct, 1)
            if sub_dict:
                item["submethods"] = sub_dict

        result[name] = item

    return result


def update_baseline_file(baseline_path: Path, target_name: str, methods_profile: dict):
    """
    Updates the target benchmark in the golden baseline JSON with the methods profile.
    """
    if not baseline_path.exists():
        print(f"Error: Baseline file '{baseline_path}' not found.", file=sys.stderr)
        return False

    with open(baseline_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    target_found = False
    for res in data.get("results", []):
        if res.get("name") == target_name:
            res["methods"] = methods_profile
            target_found = True
            break

    if not target_found:
        print(f"Error: Workload '{target_name}' not found in baseline '{baseline_path}'.", file=sys.stderr)
        return False

    with open(baseline_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)

    print(f"[OK] Successfully updated baseline for '{target_name}' with method profile breakdown in: {baseline_path}")
    return True


def print_summary_table(target_name: str, methods_profile: dict):
    """
    Prints a formatted terminal summary table.
    """
    print("\n" + "=" * 76)
    print(f"[PROFILE] CHIPSET METHOD PROFILE BREAKDOWN: {target_name}")
    print("=" * 76)
    print(f"{'Method / Subsystem':<36} {'Module':<12} {'Time Share (%)':>16}")
    print("-" * 76)

    for method, info in methods_profile.items():
        module = info.get("module", "subsystem")
        pct = info.get("percent", 0.0)
        print(f"{method:<36} [{module:<10}] {pct:>15.1f}%")
        if "submethods" in info:
            for sub_name, sub_pct in info["submethods"].items():
                print(f"  +-- {sub_name:<30} {'':<12} {sub_pct:>15.1f}%")

    print("=" * 76 + "\n")


def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")

    parser = argparse.ArgumentParser(description="Amiga 500 External Profile Aggregator")
    parser.add_argument("profile_file", nargs="?", type=Path, help="Path to profile file (.json Gecko profile or .folded stacks)")
    parser.add_argument("--target", default="coptim1", help="Target benchmark workload name (default: coptim1)")
    parser.add_argument("--update-baseline", type=Path, help="Path to chipset_benchmark_baseline.json to update")
    parser.add_argument("--json", action="store_true", help="Output aggregated profile as JSON")

    args = parser.parse_args()

    if not args.profile_file:
        parser.print_help()
        sys.exit(1)

    profile_path = args.profile_file
    if not profile_path.exists():
        print(f"Error: Profile file not found: {profile_path}", file=sys.stderr)
        sys.exit(1)

    if profile_path.suffix == ".json":
        # Check if it's Gecko profile or simple pre-computed map
        with open(profile_path, "r", encoding="utf-8") as f:
            first_chars = f.read(100)
        if '"threads"' in first_chars or '"meta"' in first_chars:
            m_counts, sub_counts, total = parse_gecko_profile(profile_path)
            methods_profile = build_methods_profile(m_counts, sub_counts, total)
        else:
            with open(profile_path, "r", encoding="utf-8") as f:
                methods_profile = json.load(f)
    else:
        m_counts, sub_counts, total = parse_folded_stacks(profile_path)
        methods_profile = build_methods_profile(m_counts, sub_counts, total)

    if args.json:
        print(json.dumps(methods_profile, indent=2))
    else:
        print_summary_table(args.target, methods_profile)

    if args.update_baseline:
        update_baseline_file(args.update_baseline, args.target, methods_profile)


if __name__ == "__main__":
    main()
