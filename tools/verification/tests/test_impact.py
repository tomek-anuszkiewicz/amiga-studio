"""Selection must retain consumers and fall back when ownership is uncertain."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import impact
from repository import collect_changes, git


class ImpactTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()

    def metadata(self):
        def package(name, dependencies=(), directory=None):
            return {"id": name, "name": name,
                    "manifest_path": str(self.root / (directory or "crates/" + name) / "Cargo.toml"),
                    "dependencies": [{"name": dependency, "path": str(self.root / "crates" / dependency),
                                      "kind": kind, "optional": True, "target": "cfg(windows)", "rename": "alias"}
                                     for dependency, kind in dependencies]}
        packages = [package("base"), package("core", [("base", None)]),
                    package("machine", [("core", "dev")]), package("gui", [("machine", "build")]),
                    package("test_runner", [("core", "dev")]), package("independent"),
                    package("generator", directory="tools/generator")]
        return {"packages": packages, "workspace_members": [item["id"] for item in packages]}

    def select(self, paths, metadata=None):
        output = json.dumps(metadata if metadata is not None else self.metadata())
        with patch.object(impact.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, output, "")) as run:
            scope = impact.select_rust_scope(self.root, paths, ("tools/artifacts/", "tools/bootstrap/"))
        if run.called:
            self.assertIn("--locked", run.call_args.args[0])
            self.assertIn("--offline", run.call_args.args[0])
        return scope

    def test_transitive_consumers_include_all_dependency_kinds_and_no_unrelated_package(self):
        scope = self.select(["crates/core/src/lib.rs"])
        self.assertEqual(scope.packages, ("core", "gui", "machine", "test_runner"))
        # Dependencies build as needed; their own tests are not consumers.
        self.assertNotIn("base", scope.packages)

    def test_package_fixtures_deleted_files_and_non_crates_members_have_owners(self):
        for path, expected in (("crates/gui/tests/old.bin", ("gui",)),
                               ("crates/gui/README.md", ("gui",)),
                               ("tools/generator/src/lib.rs", ("generator",))):
            with self.subTest(path=path):
                self.assertEqual(self.select([path]).packages, expected)
        self.assertEqual(self.select(["crates/gui/src/old.rs", "crates/independent/src/new.rs"]).packages,
                         ("gui", "independent"))

    def test_cycles_terminate_and_multiple_changes_form_a_union(self):
        metadata = self.metadata()
        metadata["packages"][0]["dependencies"].append({"path": str(self.root / "crates/gui")})
        scope = self.select(["crates/gui/src/lib.rs", "crates/independent/tests/test.rs"], metadata)
        self.assertEqual(scope.packages, ("base", "core", "gui", "independent", "machine", "test_runner"))

    def test_independent_docs_and_python_skip_rust(self):
        self.assertEqual(self.select(["DIARY.md", "docs/guide.md", ".agents/rules/git-commits.md",
                                      "tools/verification/README.md", "tools/artifacts/report.py"]).packages, ())

    def test_unknown_inputs_build_changes_and_selection_changes_require_full_scope(self):
        for path in ("Cargo.lock", "crates/core/Cargo.toml", "crates/core/build.rs", ".cargo/config.toml",
                     "rust-toolchain.toml", "tools/verification/impact.py", "tools/verification/tests/test_impact.py",
                     "Obsidian/Amiga/Design/CPU.md", "tools/new/check.py", "examples/fixture.bin",
                     "crates/deleted/src/lib.rs", "tools/artifacts/native.rs", "tools/artifacts-old/a.py"):
            with self.subTest(path=path):
                self.assertIsNone(self.select([path]).packages)
        self.assertIsNone(self.select([]).packages)

    def test_metadata_failure_incomplete_graph_and_outside_local_dependency_cannot_skip(self):
        with patch.object(impact.subprocess, "run", side_effect=OSError("not available")):
            self.assertIsNone(impact.select_rust_scope(self.root, ["docs/guide.md"]).packages)
        for change in ("incomplete", "outside", "malformed"):
            metadata = self.metadata()
            if change == "incomplete":
                metadata["workspace_members"].append("missing")
            elif change == "outside":
                metadata["packages"][0]["dependencies"].append({"path": str(self.root.parent / "external")})
            else:
                metadata = {"packages": []}
            with self.subTest(change=change):
                self.assertIsNone(self.select(["docs/guide.md"], metadata).packages)

    def test_git_change_boundary_includes_index_worktree_new_files_and_both_rename_ends(self):
        git(self.root, "init", "-q")
        git(self.root, "config", "user.name", "Test")
        git(self.root, "config", "user.email", "test@example.invalid")
        for path in ("old.txt", "working.txt", "staged.txt"):
            (self.root / path).write_text("original", encoding="utf-8")
        git(self.root, "add", ".")
        git(self.root, "commit", "--quiet", "--message", "fixture")
        git(self.root, "mv", "old.txt", "new.txt")
        (self.root / "staged.txt").write_text("staged", encoding="utf-8")
        git(self.root, "add", "staged.txt")
        (self.root / "working.txt").write_text("working", encoding="utf-8")
        (self.root / "untracked.txt").write_text("new", encoding="utf-8")
        self.assertEqual(collect_changes(self.root), {"old.txt", "new.txt", "staged.txt", "working.txt", "untracked.txt"})


if __name__ == "__main__":
    unittest.main()
