#!/usr/bin/env python3
"""
tools/scaffold_crate.py
Deterministic Workspace Crate Scaffolder for Amiga 500 Emulator.

Generates a fully compliant workspace crate adhering strictly to:
- .agents/rules/workspace-structure-and-reexports.md (flat on disk, 3-tier layout)
- .agents/rules/rust-best-practices.md (decoupled state struct, serde derived)
- .agents/rules/unit-testing-policy.md (external tests/ harness, zero inline tests in src/)
"""

import sys
import os
import re
import time
import argparse
import subprocess
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parent.parent
CRATES_DIR = REPO_ROOT / "crates"
ROOT_CARGO_TOML = REPO_ROOT / "Cargo.toml"

def to_pascal_case(snake_str):
    return "".join(word.capitalize() for word in snake_str.split("_"))

def to_title_case(snake_str):
    return " ".join(word.capitalize() for word in snake_str.split("_"))

def validate_crate_name(name):
    if not re.match(r"^[a-z][a-z0-9_]*$", name):
        raise ValueError(
            f"Invalid crate name '{name}'. Must start with a lowercase letter and contain only lowercase letters, numbers, and underscores."
        )

def update_root_cargo_toml(crate_name, dry_run=False):
    with open(ROOT_CARGO_TOML, "r", encoding="utf-8") as f:
        content = f.read()

    crate_entry = f'    "crates/{crate_name}",'
    dep_entry = f'{crate_name} = {{ path = "crates/{crate_name}" }}'

    if f'"crates/{crate_name}"' in content:
        print(f"  [Info] 'crates/{crate_name}' already registered in root workspace members.")
    else:
        # Insert into members array before tools/ or test_runner
        members_match = re.search(r'(members\s*=\s*\[)([^\]]+)(\])', content, re.DOTALL)
        if not members_match:
            raise RuntimeError("Could not locate 'members = [...]' in root Cargo.toml")
        
        pre, inner, post = members_match.groups()
        # Find position to insert
        lines = inner.splitlines()
        # Insert before tools/blep_generator or test_runner, or at end of crate entries
        insert_idx = len(lines)
        for idx, line in enumerate(lines):
            if "tools/" in line or "test_runner" in line:
                insert_idx = idx
                break
        
        lines.insert(insert_idx, f'    "crates/{crate_name}",')
        new_inner = "\n".join(lines)
        content = content[:members_match.start(2)] + new_inner + content[members_match.end(2):]

    if f'{crate_name} =' in content:
        print(f"  [Info] '{crate_name}' already declared in root [workspace.dependencies].")
    else:
        # Insert into [workspace.dependencies]
        dep_section_match = re.search(r'(\[workspace\.dependencies\]\n)', content)
        if not dep_section_match:
            raise RuntimeError("Could not locate '[workspace.dependencies]' in root Cargo.toml")
        
        # Insert at the end of [workspace.dependencies] before the next section
        next_section_match = re.search(r'\n(\[[^\]]+\])', content[dep_section_match.end():])
        if next_section_match:
            insert_pos = dep_section_match.end() + next_section_match.start()
            content = content[:insert_pos] + dep_entry + "\n" + content[insert_pos:]
        else:
            content += "\n" + dep_entry + "\n"

    if not dry_run:
        with open(ROOT_CARGO_TOML, "w", encoding="utf-8", newline="\n") as f:
            f.write(content)

def update_parent_crate(parent_name, child_name, child_pascal, dry_run=False):
    parent_dir = CRATES_DIR / parent_name
    if not parent_dir.exists():
        raise RuntimeError(f"Parent crate '{parent_name}' not found under crates/")

    parent_cargo = parent_dir / "Cargo.toml"
    with open(parent_cargo, "r", encoding="utf-8") as f:
        cargo_content = f.read()

    dep_line = f"{child_name} = {{ workspace = true }}"
    if dep_line not in cargo_content:
        # Add under [dependencies]
        if "[dependencies]" in cargo_content:
            cargo_content = cargo_content.replace("[dependencies]\n", f"[dependencies]\n{dep_line}\n")
        else:
            cargo_content += f"\n[dependencies]\n{dep_line}\n"

        if not dry_run:
            with open(parent_cargo, "w", encoding="utf-8", newline="\n") as f:
                f.write(cargo_content)

    parent_lib = parent_dir / "src" / "lib.rs"
    if parent_lib.exists():
        with open(parent_lib, "r", encoding="utf-8") as f:
            lib_content = f.read()

        reexport_mod = f"pub use {child_name};"
        reexport_struct = f"pub use {child_name}::{{{child_pascal}, {child_pascal}State}};"

        additions = []
        if reexport_mod not in lib_content:
            additions.append(reexport_mod)
        if reexport_struct not in lib_content:
            additions.append(reexport_struct)

        if additions:
            lib_content = "\n".join(additions) + "\n\n" + lib_content
            if not dry_run:
                with open(parent_lib, "w", encoding="utf-8", newline="\n") as f:
                    f.write(lib_content)

def scaffold_crate(name, tier, parent=None, extra_deps=None, dry_run=False):
    validate_crate_name(name)
    target_dir = CRATES_DIR / name

    if target_dir.exists():
        raise RuntimeError(f"Crate directory '{target_dir}' already exists.")

    pascal_name = to_pascal_case(name)
    title_name = to_title_case(name)

    deps = ["serde = { workspace = true, features = [\"derive\"] }"]
    if tier in (2, 3) or (extra_deps and "config" in extra_deps):
        deps.append("config = { workspace = true }")

    if extra_deps:
        for dep in extra_deps:
            dep = dep.strip()
            if dep and dep != "config" and dep != "serde":
                deps.append(f"{dep} = {{ workspace = true }}")

    dependencies_toml = "\n".join(deps)

    cargo_toml_content = f"""[package]
name = "{name}"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
{dependencies_toml}
"""

    config_import = "use config::*;\n" if tier in (2, 3) or (extra_deps and "config" in extra_deps) else ""

    lib_rs_content = f"""//! {title_name} Subsystem Emulation
//!
//! Part of the cycle-exact Amiga 500 emulator architecture.

{config_import}use serde::{{Deserialize, Serialize}};

/// Decoupled hardware state snapshot for {title_name}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct {pascal_name}State {{
    /// Cycle accumulator or status latch
    pub status: u32,
}}

/// {title_name} subsystem handle
#[derive(Debug, Clone, Default)]
pub struct {pascal_name} {{
    /// Read-only observable state
    pub state: {pascal_name}State,
}}

impl {pascal_name} {{
    /// Creates a new {title_name} subsystem instance
    pub fn new() -> Self {{
        Self::default()
    }}

    /// Resets subsystem to power-on default state
    pub fn reset(&mut self) {{
        self.state = {pascal_name}State::default();
    }}
}}
"""

    test_rs_content = f"""//! External unit and integration tests for {name} subsystem
//!
//! Validates state initialization, reset dynamics, and serialization.

use {name}::{{{pascal_name}, {pascal_name}State}};

#[test]
fn test_{name}_default_initialization() {{
    let instance = {pascal_name}::new();
    assert_eq!(instance.state, {pascal_name}State::default());
}}

#[test]
fn test_{name}_reset_dynamics() {{
    let mut instance = {pascal_name}::new();
    instance.state.status = 0xDEADBEEF;
    instance.reset();
    assert_eq!(instance.state.status, 0);
}}

#[test]
fn test_{name}_state_serialization_roundtrip() {{
    let instance = {pascal_name}::new();
    let serialized = serde_json::to_string(&instance.state)
        .expect("Failed to serialize {pascal_name}State");
    let deserialized: {pascal_name}State = serde_json::from_str(&serialized)
        .expect("Failed to deserialize {pascal_name}State");
    assert_eq!(instance.state, deserialized);
}}
"""

    if dry_run:
        print(f">> [DRY-RUN] Scaffolding for 'crates/{name}' (Tier {tier}):")
        print(f"\n--- crates/{name}/Cargo.toml ---")
        print(cargo_toml_content)
        print(f"\n--- crates/{name}/src/lib.rs ---")
        print(lib_rs_content)
        print(f"\n--- crates/{name}/tests/test_{name}.rs ---")
        print(test_rs_content)
        update_root_cargo_toml(name, dry_run=True)
        if parent:
            print(f"\n[Info] Would register with parent '{parent}'")
        return

    # Create directories and files
    src_dir = target_dir / "src"
    tests_dir = target_dir / "tests"
    src_dir.mkdir(parents=True, exist_ok=True)
    tests_dir.mkdir(parents=True, exist_ok=True)

    with open(target_dir / "Cargo.toml", "w", encoding="utf-8", newline="\n") as f:
        f.write(cargo_toml_content)

    with open(src_dir / "lib.rs", "w", encoding="utf-8", newline="\n") as f:
        f.write(lib_rs_content)

    with open(tests_dir / f"test_{name}.rs", "w", encoding="utf-8", newline="\n") as f:
        f.write(test_rs_content)

    # Update root workspace Cargo.toml
    update_root_cargo_toml(name, dry_run=False)

    # If Tier 3 with parent, update parent crate
    if parent:
        update_parent_crate(parent, name, pascal_name, dry_run=False)

def main():
    parser = argparse.ArgumentParser(
        description="Deterministic Workspace Crate Scaffolder for Amiga 500 Emulator."
    )
    parser.add_argument("name", help="Snake_case crate name (e.g. 'custom_chips')")
    parser.add_argument(
        "--tier",
        type=int,
        choices=[1, 2, 3],
        default=2,
        help="Crate tier: 1 (Foundational), 2 (Peer Subsystem), 3 (Contained Sub-Component)"
    )
    parser.add_argument("--parent", help="Parent peer crate name for Tier 3 containment (e.g. 'memory_bus')")
    parser.add_argument("--deps", help="Comma-separated workspace dependencies (e.g. 'dma,blitter')")
    parser.add_argument("--dry-run", action="store_true", help="Print generated files without writing to disk")
    parser.add_argument("--check", action="store_true", help="Run 'cargo check' on the newly scaffolded crate")

    args = parser.parse_args()

    start_time = time.time()
    try:
        extra_deps = [d.strip() for d in args.deps.split(",")] if args.deps else []
        scaffold_crate(
            name=args.name,
            tier=args.tier,
            parent=args.parent,
            extra_deps=extra_deps,
            dry_run=args.dry_run
        )

        if not args.dry_run and args.check:
            print(f"  Running 'cargo check -p {args.name} --quiet'...")
            res = subprocess.run(
                ["cargo", "check", "-p", args.name, "--quiet"],
                cwd=REPO_ROOT,
                capture_output=True,
                text=True
            )
            if res.returncode != 0:
                print(f"[FAIL] cargo check failed:\n{res.stderr}", file=sys.stderr)
                sys.exit(1)

        elapsed = time.time() - start_time
        mode_str = "[DRY-RUN] " if args.dry_run else ""
        print(f"[PASS] {mode_str}Scaffolding complete for 'crates/{args.name}' (Tier {args.tier}) in {elapsed:.2f}s.")

    except Exception as e:
        print(f"[ERROR] Scaffolding failed: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
