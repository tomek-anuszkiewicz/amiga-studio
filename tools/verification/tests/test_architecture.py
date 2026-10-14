"""Failure and boundary contracts for repository source inspection."""
import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
import check_architecture as architecture

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / ".agents/skills/obsidian-vault-linking/scripts"))
import audit_vault as vault


class ArchitectureContracts(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.write("ROADMAP.md", "")
        (self.root / "crates").mkdir()

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="")
        return path

    def check(self, name, passes, detail=""):
        result = unittest.TestResult()
        with patch.object(architecture, "REPO_ROOT", self.root):
            architecture.ArchitectureChecks(name).run(result)
        self.assertFalse(result.errors, result.errors)
        self.assertEqual(result.wasSuccessful(), passes, result.failures)
        if detail:
            self.assertIn(detail, result.failures[0][1])

    def test_source_line_limit_and_registered_exceptions(self):
        name = "test_file_size_limits"
        self.write("crates/example/src/example.rs", "line\r\n" * 800)
        path = self.write("crates/example/src/lines.rs", "first\r\nlast\r")
        self.assertEqual(architecture.read_lines(path), ["first", "last\r"])
        self.write("crates/example/tests/test_large.rs", "line\n" * 801)
        self.write("crates/cpu/src/dispatch_table.rs", "line\n" * 801)
        self.check(name, True)
        self.write("crates/example/src/example.rs", "line\n" * 801)
        self.check(name, False, "801 lines")

    def test_missing_and_stale_exceptions_fail_without_pruning(self):
        name = "test_no_stale_line_count_exceptions"
        for exception in architecture.LINE_COUNT_EXCEPTIONS:
            self.write("crates/cpu/src/" + exception, "line\n" * 801)
        self.check(name, True)
        path = self.write("crates/cpu/src/add.rs", "line\n" * 800)
        self.check(name, False, "exception is stale")
        path.unlink()
        self.check(name, False, "matches no production file")

    def test_runtime_calls_preserve_comment_handling_and_crate_scope(self):
        name = "test_zero_runtime_panics_or_unwraps"
        self.write("crates/cpu/src/cpu.rs", "// a.unwrap()\n/*\na.expect(\n*/\nvalue(); // b.unwrap()\n")
        self.write("crates/gui/src/gui.rs", "a.unwrap()\n")
        self.check(name, True)
        self.write("crates/cpu/src/cpu.rs", "/*\n*/ a.expect(\n")
        self.check(name, False, "cpu.rs:2")
        self.write("crates/cpu/src/cpu.rs", "a.unwrap()\n")
        self.check(name, False, "cpu.rs:1")

    def test_path_patterns_include_rust_tests(self):
        name = "test_no_external_hardcoded_paths"
        self.write("crates/example/tests/test_paths.rs", "relative/path\n")
        self.check(name, True)
        self.write("crates/example/tests/test_paths.rs", "/home/example\n")
        self.check(name, False, "test_paths.rs:1")

    def test_macros_skip_line_comments_but_cover_all_crates(self):
        name = "test_zero_user_defined_macros"
        self.write("crates/gui/tests/test_example.rs", "// macro_rules!\n")
        self.check(name, True)
        self.write("crates/gui/tests/test_example.rs", "macro_rules! example {}\n")
        self.check(name, False, "test_example.rs:1")

    def test_const_handlers_scan_only_cpu_sources(self):
        name = "test_zero_const_generic_handlers"
        self.write("crates/gui/src/gui.rs", "fn example<const N: usize>() {}\n")
        self.write("crates/cpu/src/cpu.rs", "// fn example<const N: usize>() {}\n")
        self.check(name, True)
        self.write("crates/cpu/src/cpu.rs", "fn example<const N: usize>() {}\n")
        self.check(name, False, "cpu.rs:1")

    def test_inlining_preserves_two_line_attribute_window(self):
        name = "test_inlining_guidelines_compliance"
        setters = ("xnzvc", "nzvc", "nz_clear_vc", "z_only", "v_clear_c")
        state = "".join(f"#[inline(always)]\npub fn set_ccr_{setter}() {{}}\n" for setter in setters)
        self.write("crates/cpu/src/state.rs", state)
        self.write("crates/cpu/src/exception.rs", "#[inline(never)]\n// cold\npub fn trigger_example() {}\n")
        self.write("crates/cpu/src/instructions/add.rs", "#[inline(always)]\npub fn add_example() {}\n")
        self.check(name, True)
        self.write("crates/cpu/src/exception.rs", "#[inline(never)]\n// cold\n// too far\npub fn trigger_example() {}\n")
        self.check(name, False, "inline(never)")
        self.write("crates/cpu/src/exception.rs", "")
        self.write("crates/cpu/src/instructions/add.rs", "pub fn add_example() {}\n")
        self.check(name, False, "inline(always)")
        self.write("crates/cpu/src/instructions/add.rs", "")
        self.write("crates/cpu/src/state.rs", state.replace("set_ccr_z_only", "other"))
        self.check(name, False, "Could not find CCR setter")

    def test_instruction_directory_rejects_bundles_other_files_and_subdirectories(self):
        name = "test_flat_instruction_hierarchy_and_zero_subdirectories"
        self.write("crates/cpu/src/instructions/add.rs", "")
        self.check(name, True)
        for relative, detail in (("mul.rs", "Legacy bundled"), ("extra.txt", "Non-Rust"),
                                 ("nested/add.rs", "subdirectories")):
            path = self.write("crates/cpu/src/instructions/" + relative, "")
            self.check(name, False, detail)
            path.unlink()
            if path.parent.name == "nested":
                path.parent.rmdir()

    def test_idle_aliases_and_anonymous_six_line_windows(self):
        name = "test_idle_microstep_naming_and_prohibition_of_anonymous_idle_structs"
        path = "crates/cpu/src/instructions/add.rs"
        self.write(path, "// READ_WORD_FINISH\ncommon::ALU_IDLE\n")
        self.check(name, True)
        self.write(path, "READ_WORD_FINISH\n")
        self.check(name, False, "Uses legacy alias")
        self.write(path, "MicroStep {\nbus_fn: None,\nalu_fn: None,\nbase_clocks: 4,\n}\n")
        self.check(name, False, "common::ALU_IDLE_4CLK")

    def test_instruction_budgets_measure_bytes_and_ignore_nested_rules(self):
        name = "test_rule_files_size_limit_and_truncation_safety"
        self.write("AGENTS.md", "a" * architecture.MAX_AGENTS_MD_BYTES)
        self.write(".agents/rules/example.md", "a" * architecture.MAX_RULE_FILE_BYTES)
        self.write(".agents/rules/nested/example.md", "a" * (architecture.MAX_RULE_FILE_BYTES + 1))
        self.check(name, True)
        self.write("AGENTS.md", "a" * (architecture.MAX_AGENTS_MD_BYTES - 1) + "£")
        self.check(name, False, "14001 bytes")
        self.write("AGENTS.md", "")
        self.write(".agents/rules/example.md", "a" * (architecture.MAX_RULE_FILE_BYTES + 1))
        self.check(name, False, "23001 bytes")

    def test_design_links_decode_paths_ignore_external_links_and_keep_minimums(self):
        name = "test_obsidian_design_docs_links_integrity"
        directory = "Obsidian/Amiga/Design/"
        for number in range(25):
            folder = "Application/" if number >= 5 else ""
            link = "../Target%20Note.md#heading" if folder else "Target%20Note.md#heading"
            self.write(directory + folder + f"document{number}.md", f"[link]({link})\n" * 12)
        target = self.write(directory + "Target Note.md", "[external](https://example.com)\n[anchor](#heading)\n")
        self.check(name, True)
        target.unlink()
        self.check(name, False, "Broken link")
        self.write(directory + "Target Note.md", "")
        for path in (self.root / directory).rglob("document*.md"):
            path.write_text("", encoding="utf-8")
        self.check(name, False, "at least 300 links")

    def test_vault_audit_covers_nested_notes_and_resolves_links_from_their_parent(self):
        directory = "Obsidian/Amiga/Design/"
        properties = '---\nsubsystem: "gui"\ntags: [design, gui]\n---\n'
        target = self.write(directory + "Target Note.md", properties + "[GUI](Application/GUI.md)\n")
        self.write(directory + "Application/GUI.md", properties + "[target](../Target%20Note.md#heading)\n")
        with patch.object(vault, "REPO_ROOT", self.root):
            links = vault.check_vault_links()
            self.assertEqual(links["total_docs"], 2)
            self.assertEqual(links["total_links"], 2)
            self.assertEqual(links["broken_links"], [])
            self.assertEqual(vault.check_frontmatter_compliance()["total_docs"], 2)
            self.assertEqual(vault.check_frontmatter_compliance()["issues"], [])
            target.unlink()
            broken = vault.check_vault_links()["broken_links"]
            self.assertEqual(len(broken), 1)
            self.assertEqual(broken[0]["file"], "Application/GUI.md")
            self.write(directory + "Application/GUI.md", "# Missing properties\n")
            issues = vault.check_frontmatter_compliance()["issues"]
            self.assertEqual(len(issues), 1)
            self.assertEqual(issues[0]["file"], "Application/GUI.md")

    def test_inline_tests_include_nested_sources_but_ignore_external_tests(self):
        name = "test_zero_inline_tests_in_crates_src"
        self.write("crates/example/tests/test_example.rs", "#[test]\n")
        self.write("crates/example/src/nested/example.rs", "// #[test]\n")
        self.check(name, True)
        self.write("crates/example/src/nested/example.rs", "#[cfg(test)]\n")
        self.check(name, False, "example.rs:1")

    def test_compatibility_checks_comments_identifiers_and_root_wrappers(self):
        name = "test_zero_backward_compatibility_shims_and_stale_aliases"
        path = "crates/example/src/example.rs"
        self.write(path, "pub mod micro {\npub use module::*;\n}\n")
        self.check(name, True)
        for text, detail in (("// Legacy Compatibility\n", "compatibility phrase"),
                             ("reset_cold();\n", "stale/legacy identifier"),
                             ("pub mod example {\npub use module::*;\n}\n", "wrapper module")):
            self.write(path, text)
            self.check(name, False, detail)

    def test_existing_external_tests_need_executable_case(self):
        name = "test_existing_external_test_suites_are_executable"
        self.write("crates/example/Cargo.toml", "")
        self.check(name, True)
        self.write("crates/example/tests/test_example.rs", "")
        self.check(name, False, "At least one executable")
        self.write("crates/example/tests/test_example.rs", "#[test]\nfn test_example() {}\n")
        self.check(name, True)

    def test_named_roots_scan_owned_cargo_tools_and_manifest_paths(self):
        name = "test_named_crate_roots_and_zero_generic_lib_rs"
        self.write("crates/example/Cargo.toml", '[lib]\npath = "src/example.rs"\n')
        self.write("crates/example/src/example.rs", "")
        self.write("tools/vendor/src/lib.rs", "")
        self.check(name, True)
        self.write("tools/vendor/Cargo.toml", "")
        self.check(name, False, "Generic lib.rs")
        (self.root / "tools/vendor/Cargo.toml").unlink()
        self.write("crates/example/Cargo.toml", "[lib]\n")
        self.check(name, False, "Missing [lib] configuration")

    def test_named_roots_allow_harness_packages_but_require_core_libraries(self):
        name = "test_named_crate_roots_and_zero_generic_lib_rs"
        self.write("crates/harness/Cargo.toml", '[package]\nname = "harness"\n')
        self.check(name, False, "Missing expected crate root")
        self.write("crates/harness/tests/test_cpu.rs", "#[test]\nfn test_cpu() {}\n")
        self.check(name, True)
        self.write("crates/harness/Cargo.toml", '[lib]\npath = "missing.rs"\n')
        self.check(name, False, "Missing expected crate root")
        self.write("crates/harness/Cargo.toml", '[package]\nname = "harness"\n')
        self.write("crates/cpu/Cargo.toml", '[package]\nname = "cpu"\n')
        self.write("crates/cpu/tests/test_cpu.rs", "#[test]\nfn test_cpu() {}\n")
        self.check(name, False, "crates/cpu: Missing expected crate root")

    def test_test_file_names_apply_only_to_direct_rust_files(self):
        name = "test_canonical_test_file_naming_convention"
        self.write("crates/example/Cargo.toml", "")
        self.write("crates/example/tests/common/mod.rs", "")
        self.write("crates/example/tests/test_example.rs", "")
        self.check(name, True)
        self.write("crates/example/tests/example.rs", "")
        self.check(name, False, "test_ prefix")

    def test_modular_parity_keeps_all_required_files(self):
        name = "test_multi_module_crate_test_parity"
        for crate, files in architecture.REQUIRED_MULTI_MODULE_TESTS.items():
            for file in files:
                self.write(f"crates/{crate}/tests/{file}", "")
        self.check(name, True)
        (self.root / "crates/disassembler/tests/test_ea.rs").unlink()
        self.check(name, False, "crates/disassembler/tests/test_ea.rs")

    def test_filter_is_a_literal_substring_and_extra_arguments_fail(self):
        with patch.object(architecture, "REPO_ROOT", self.root), contextlib.redirect_stderr(io.StringIO()) as output:
            self.assertEqual(architecture.main(["zero_const"]), 0)
            self.assertIn("Ran 1 test", output.getvalue())
            output.seek(0)
            output.truncate()
            self.assertEqual(architecture.main(["*zero_const*"]), 0)
            self.assertIn("Ran 0 tests", output.getvalue())
            self.assertEqual(architecture.main(["one", "two"]), 2)


if __name__ == "__main__":
    unittest.main()
