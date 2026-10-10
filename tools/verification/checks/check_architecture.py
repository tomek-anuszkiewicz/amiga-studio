"""Repository architecture checks; source inspection requires no Rust build."""
from pathlib import Path
import sys
import unittest
from urllib.parse import unquote


REPO_ROOT = Path(__file__).resolve().parents[3]

# Preserve the reviewed exceptions; pruning requires explicit user approval.
LINE_COUNT_EXCEPTIONS = (
    "dispatch_table.rs", "move_w.rs", "move_b.rs", "move_l.rs", "add.rs",
    "sub.rs", "and.rs", "or.rs", "cmpi.rs",
)
CORE_EMULATION_CRATES = (
    "cpu", "physical_memory", "memory_bus", "config", "rtc", "disassembler",
    "debugger", "copper", "blitter", "dma", "agnus", "sprites", "frame_builder",
    "mouse", "joystick", "denise", "audio", "floppy", "interrupts", "paula",
    "keyboard", "game_ports", "cia", "machine_loop",
)
MAX_RULE_FILE_BYTES = 23_000
MAX_AGENTS_MD_BYTES = 14_000
REQUIRED_MULTI_MODULE_TESTS = {
    "blitter": ("test_blitter.rs", "test_line.rs", "test_minterm.rs", "test_phase.rs"),
    "disassembler": ("test_align.rs", "test_alu.rs", "test_branch.rs", "test_data.rs", "test_ea.rs"),
    "floppy": ("test_floppy.rs", "test_mfm.rs"),
    "config": ("test_config.rs", "test_mutation.rs"),
    "physical_memory": ("test_address_bus.rs", "test_map.rs", "test_physical_memory.rs", "test_presets.rs"),
    "paula": ("test_paula.rs", "test_paula_registers.rs", "test_serial.rs"),
}


def source_files(directory):
    return sorted(path for path in directory.rglob("*.rs") if path.is_file() and path.suffix == ".rs")


def read_lines(path):
    # Rust str::lines splits on LF, strips CRLF, and omits the final empty line.
    with path.open(encoding="utf-8", newline="") as source:
        text = source.read()
    lines = text.split("\n")
    terminated = len(lines) - 1
    if lines[-1] == "":
        lines.pop()
    return [line.removesuffix("\r") if index < terminated else line for index, line in enumerate(lines)]


class ArchitectureChecks(unittest.TestCase):
    def setUp(self):
        self.root = REPO_ROOT

    def relative(self, path):
        return path.relative_to(self.root).as_posix()

    def production_files(self):
        return [path for path in source_files(self.root / "crates") if "src" in path.parts]

    def crates(self):
        return sorted(path for path in (self.root / "crates").iterdir()
                      if path.is_dir() and (path / "Cargo.toml").exists())

    def no_violations(self, violations, description):
        self.assertFalse(violations, description + "\n" + "\n".join(violations))

    def test_file_size_limits(self):
        violations = []
        for path in self.production_files():
            if path.name not in LINE_COUNT_EXCEPTIONS:
                count = len(read_lines(path))
                if count > 800:
                    violations.append(f"{self.relative(path)} ({count} lines > 800 line limit)")
        self.no_violations(violations, "Architecture Rule Violation: Rust source file size limit")

    def test_no_stale_line_count_exceptions(self):
        violations = []
        files = self.production_files()
        for name in LINE_COUNT_EXCEPTIONS:
            matching = [path for path in files if path.name == name]
            if not matching:
                violations.append(f"Exception '{name}' matches no production file under crates/*/src/")
            for path in matching:
                count = len(read_lines(path))
                if count <= 800:
                    violations.append(f"{self.relative(path)} ({count} lines <= 800 limit; exception is stale)")
        self.no_violations(violations, "Architecture Rule Violation: Stale or missing LINE_COUNT_EXCEPTIONS. "
                           "Notify the user for explicit approval before removing any entry.")

    def test_zero_runtime_panics_or_unwraps(self):
        violations = []
        for crate in CORE_EMULATION_CRATES:
            for path in source_files(self.root / "crates" / crate / "src"):
                in_block_comment = False
                for number, line in enumerate(read_lines(path), 1):
                    trimmed = line.strip()
                    if in_block_comment:
                        if "*/" in trimmed:
                            in_block_comment = False
                            rest = trimmed.split("*/", 1)[1]
                            if ".unwrap()" in rest or ".expect(" in rest:
                                violations.append(f"{self.relative(path)}:{number} -> {trimmed}")
                        continue
                    if trimmed.startswith("/*"):
                        in_block_comment = "*/" not in trimmed
                        continue
                    if trimmed.startswith("//"):
                        continue
                    code = trimmed.split("//", 1)[0]
                    if ".unwrap()" in code or ".expect(" in code:
                        violations.append(f"{self.relative(path)}:{number} -> {trimmed}")
        self.no_violations(violations, "Architecture Rule Violation: .unwrap() or .expect() in core emulation")

    def test_no_external_hardcoded_paths(self):
        violations = []
        patterns = ("C:\\Users\\", "C:/Users/", "/home/", "Google Drive")
        for path in source_files(self.root / "crates"):
            for number, line in enumerate(read_lines(path), 1):
                for pattern in patterns:
                    if pattern in line:
                        violations.append(f"{self.relative(path)}:{number} contains forbidden external path pattern '{pattern}'")
        self.no_violations(violations, "Path Privacy Violation: Hardcoded external paths")

    def test_zero_user_defined_macros(self):
        violations = []
        for path in source_files(self.root / "crates"):
            for number, line in enumerate(read_lines(path), 1):
                trimmed = line.strip()
                if not trimmed.startswith("//") and "macro_rules!" in trimmed:
                    violations.append(f"{self.relative(path)}:{number} -> {trimmed}")
        self.no_violations(violations, "Architecture Rule Violation: Custom macros are forbidden")

    def test_zero_const_generic_handlers(self):
        violations = []
        for path in source_files(self.root / "crates/cpu/src"):
            for number, line in enumerate(read_lines(path), 1):
                trimmed = line.strip()
                if not trimmed.startswith("//") and "<const " in trimmed:
                    violations.append(f"{self.relative(path)}:{number} -> {trimmed}")
        self.no_violations(violations, "Architecture Rule Violation: Const-generic handlers in CPU core")

    def test_inlining_guidelines_compliance(self):
        for path in source_files(self.root / "crates/cpu/src"):
            lines = read_lines(path)
            for index, line in enumerate(lines):
                trimmed = line.strip()
                if trimmed.startswith(("pub fn trigger_", "fn trigger_")):
                    self.assertTrue(any("#[inline(never)]" in previous for previous in lines[max(0, index - 2):index]),
                                    f"Inlining Guideline Violation: {self.relative(path)}:{index + 1} -> {trimmed} requires #[inline(never)]")
        setters = ("pub fn set_ccr_xnzvc", "pub fn set_ccr_nzvc", "pub fn set_ccr_nz_clear_vc",
                   "pub fn set_ccr_z_only", "pub fn set_ccr_v_clear_c")
        lines = read_lines(self.root / "crates/cpu/src/state.rs")
        for setter in setters:
            matching = [index for index, line in enumerate(lines) if setter in line]
            self.assertTrue(matching, f"Could not find CCR setter '{setter}' in state.rs")
            for index in matching:
                self.assertTrue(any("#[inline(always)]" in previous for previous in lines[max(0, index - 2):index]),
                                f"Inlining Guideline Violation: CCR setter '{setter}' requires #[inline(always)]")
        prefixes = tuple("pub fn " + name + "_" for name in (
            "add", "sub", "and", "or", "eor", "cmp", "asr", "asl", "lsr", "lsl",
            "ror", "rol", "roxr", "roxl", "neg", "negx", "not", "tst", "abcd", "sbcd",
            "nbcd", "bchg", "bclr", "bset", "btst"))
        for path in source_files(self.root / "crates/cpu/src/instructions"):
            lines = read_lines(path)
            for index, line in enumerate(lines):
                trimmed = line.strip()
                if trimmed.startswith(prefixes):
                    self.assertTrue(any("#[inline(always)]" in previous for previous in lines[max(0, index - 2):index]),
                                    f"Inlining Guideline Violation: {self.relative(path)}:{index + 1} -> {trimmed} requires #[inline(always)]")

    def test_flat_instruction_hierarchy_and_zero_subdirectories(self):
        directory = self.root / "crates/cpu/src/instructions"
        self.assertTrue(directory.exists(), "CPU instructions directory does not exist")
        subdirectories = [self.relative(path) for path in sorted(directory.rglob("*")) if path.is_dir()]
        self.no_violations(subdirectories, "Architecture Rule Violation: Instruction subdirectories are forbidden")
        legacy = ("mul.rs", "div.rs", "link_unlk.rs", "bcd.rs", "privileged.rs", "bits.rs", "shifts.rs", "system.rs")
        self.no_violations([name for name in legacy if (directory / name).exists()],
                           "Architecture Rule Violation: Legacy bundled instruction files")
        self.no_violations([self.relative(path) for path in sorted(directory.iterdir())
                            if path.is_file() and path.suffix != ".rs"],
                           "Architecture Rule Violation: Non-Rust instruction files")

    def test_idle_microstep_naming_and_prohibition_of_anonymous_idle_structs(self):
        aliases = ("READ_WORD_FINISH", "READ_BYTE_FINISH", "PREFETCH_NEXT_RETIRE", "REFILL_FIRST_FINISH",
                   "REFILL_SECOND_FINISH", "READ_TARGET_OPCODE_FINISH", "READ_VECTOR_HIGH_FINISH", "ALU_INTERNAL_2CLK")
        violations = []
        for path in source_files(self.root / "crates/cpu/src/instructions"):
            lines = read_lines(path)
            for number, line in enumerate(lines, 1):
                if not line.strip().startswith("//"):
                    for alias in aliases:
                        if alias in line.strip():
                            violations.append(f"{self.relative(path)}:{number} -> Uses legacy alias '{alias}'. Use common::BUS_READ_IDLE or common::ALU_IDLE.")
            for index, line in enumerate(lines):
                if "MicroStep {" in line:
                    block = " ".join(lines[index:index + 6])
                    if "bus_fn: None" in block and "alu_fn: None" in block:
                        hint = "standardized idle constant from common"
                        for clocks, name in ((2, "ALU_IDLE"), (4, "ALU_IDLE_4CLK"), (8, "ALU_IDLE_8CLK"), (128, "ALU_IDLE_128CLK")):
                            if f"base_clocks: {clocks}" in block:
                                hint = "common::" + name
                                break
                        violations.append(f"{self.relative(path)}:{index + 1} -> Anonymous idle MicroStep struct. Replace with '{hint}'.")
        self.no_violations(violations, "Architecture Rule Violation: Non-standard or anonymous idle micro-step usage")

    def test_rule_files_size_limit_and_truncation_safety(self):
        violations = []
        agents = self.root / "AGENTS.md"
        paths = [(agents, MAX_AGENTS_MD_BYTES)] if agents.exists() else []
        directory = self.root / ".agents/rules"
        if directory.is_dir():
            paths.extend((path, MAX_RULE_FILE_BYTES) for path in sorted(directory.glob("*.md"))
                         if path.is_file() and path.suffix == ".md")
        for path, limit in paths:
            size = path.stat().st_size
            if size > limit:
                violations.append(f"{self.relative(path)} is {size} bytes (exceeds {limit} byte ceiling)")
        self.no_violations(violations, "Architecture Rule Violation: Rule file size safety thresholds")

    def test_obsidian_design_docs_links_integrity(self):
        directory = self.root / "Obsidian/Amiga/Design"
        self.assertTrue(directory.is_dir(), "Obsidian design directory does not exist")
        files = sorted(path for path in directory.glob("*.md") if path.suffix == ".md")
        self.assertGreaterEqual(len(files), 25, f"Expected at least 25 design documents, found {len(files)}")
        violations = []
        count = 0
        for path in files:
            remaining = path.read_text(encoding="utf-8")
            while "](" in remaining:
                after = remaining.split("](", 1)[1]
                if ")" not in after:
                    break
                link, remaining = after.split(")", 1)
                link = link.strip()
                if link.startswith(("http://", "https://", "mailto:")):
                    continue
                target = link.split("#", 1)[0].strip()
                if not target:
                    continue
                count += 1
                if not (directory / unquote(target)).exists():
                    violations.append(f"[{path.name}] Broken link '{link}'")
        self.assertGreaterEqual(count, 300, f"Expected at least 300 links across design docs, found {count}")
        self.no_violations(violations, "Architecture Rule Violation: Broken design document links")

    def test_zero_inline_tests_in_crates_src(self):
        violations = []
        for path in self.production_files():
            for number, line in enumerate(read_lines(path), 1):
                trimmed = line.strip()
                if trimmed in ("#[cfg(test)]", "mod tests {", "mod test {") or trimmed.startswith("#[test]"):
                    violations.append(f"{self.relative(path)}:{number} -> Found inline test attribute or module: '{trimmed}'")
        self.no_violations(violations, "Architecture Rule Violation: Inline tests in production src/ files")

    def test_zero_backward_compatibility_shims_and_stale_aliases(self):
        phrases = ("backward compatibility alias", "legacy compatibility", "compatibility alias", "run_dual_test")
        identifiers = ("reset_cold", "dispatch_custom_write", "dispatch_agnus_action", "dispatch_paula_action",
                       "dispatch_denise_action", "dispatch_cia_action", "propagate_agnus_write", "propagate_paula_write",
                       "propagate_denise_write", "propagate_cia_write", "inject_kickstart_rom", "save_state_self_contained")
        violations = []
        for path in source_files(self.root / "crates"):
            lines = read_lines(path)
            for number, line in enumerate(lines, 1):
                for phrase in phrases:
                    if phrase in line.lower():
                        violations.append(f"{self.relative(path)}:{number} -> Contains forbidden compatibility phrase/alias '{phrase}'")
                for name in identifiers:
                    if name in line:
                        violations.append(f"{self.relative(path)}:{number} -> Contains forbidden stale/legacy identifier '{name}'")
            if path.stem == path.parent.parent.name:
                for index, line in enumerate(lines):
                    trimmed = line.strip()
                    if trimmed.startswith("pub mod ") and trimmed.endswith("{"):
                        name = trimmed[len("pub mod "):].removesuffix("{").strip()
                        if name in ("micro", "function_code"):
                            continue
                        block = " ".join(lines[index:index + 5])
                        if "pub use " in block and "*" in block:
                            violations.append(f"{self.relative(path)}:{index + 1} -> Dummy compatibility wrapper module '{trimmed}'")
        self.no_violations(violations, "Architecture Rule Violation: Compatibility shims or stale aliases")

    def test_every_crate_has_dedicated_external_tests_suite(self):
        violations = []
        for crate in self.crates():
            directory = crate / "tests"
            if not directory.is_dir():
                violations.append(f"{self.relative(crate)} -> Missing dedicated tests/ directory")
                continue
            files = sorted(path for path in directory.glob("*.rs") if path.suffix == ".rs")
            count = sum(line.strip().startswith("#[test]") for path in files for line in read_lines(path))
            if not files:
                violations.append(f"{self.relative(crate)} -> tests/ contains zero .rs test files")
            elif not count:
                violations.append(f"{self.relative(crate)} -> At least one executable #[test] function is required")
        self.no_violations(violations, "Architecture Rule Violation: Missing dedicated external test suites")

    def test_named_crate_roots_and_zero_generic_lib_rs(self):
        files = source_files(self.root / "crates")
        tools = self.root / "tools"
        if tools.is_dir():
            for path in sorted(tools.iterdir()):
                if path.is_dir() and (path / "Cargo.toml").is_file():
                    files.extend(source_files(path))
        violations = [f"{self.relative(path)}: Generic lib.rs is forbidden" for path in files if path.name == "lib.rs"]
        for crate in self.crates():
            if not (crate / "src" / (crate.name + ".rs")).is_file():
                violations.append(f"{self.relative(crate)}: Missing expected crate root src/{crate.name}.rs")
            expected = f'path = "src/{crate.name}.rs"'
            if expected not in (crate / "Cargo.toml").read_text(encoding="utf-8"):
                violations.append(f"{self.relative(crate)}/Cargo.toml: Missing [lib] configuration with '{expected}'")
        self.no_violations(violations, "Architecture Rule Violation: Crate root naming")

    def test_canonical_test_file_naming_convention(self):
        violations = []
        for crate in self.crates():
            for path in sorted((crate / "tests").glob("*.rs")):
                if path.is_file() and path.suffix == ".rs" and not path.name.startswith("test_"):
                    violations.append(f"{self.relative(path)}: Test file does not start with test_ prefix")
        self.no_violations(violations, "Architecture Rule Violation: Non-canonical test filenames")

    def test_multi_module_crate_test_parity(self):
        violations = []
        for crate, names in REQUIRED_MULTI_MODULE_TESTS.items():
            for name in names:
                path = self.root / "crates" / crate / "tests" / name
                if not path.is_file():
                    violations.append(f"{self.relative(path)}: Missing 1:1 modular unit test file for submodule")
        self.no_violations(violations, "Architecture Rule Violation: Missing modular unit test files")


def main(argv=None):
    args = list(sys.argv[1:] if argv is None else argv)
    if len(args) > 1:
        print("Use check_architecture.py [FILTER]", file=sys.stderr)
        return 2
    names = unittest.defaultTestLoader.getTestCaseNames(ArchitectureChecks)
    if args:
        names = [name for name in names if args[0] in name]
    suite = unittest.TestSuite(ArchitectureChecks(name) for name in names)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
