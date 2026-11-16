"""Protect cached path identity, instance lifetime, failure and launch behavior."""

from collections import Counter
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import json
import os
import subprocess
import sys
import tempfile

from tools.graphify import refresh


class SourcePaths:
    def __init__(self, root):
        self.root = root
        self.calls = Counter()

    def normalize(self, value):
        self.calls["normalize"] += 1
        return value.replace("\\", "/") if value is not None else None

    def absolute_identity(self, value, root):
        self.calls["absolute_identity"] += 1
        value = self.normalize(value)
        return str(root / value) if value else value

    def identity(self, value):
        self.calls["identity"] += 1
        return self.absolute_identity(value, self.root)

    def in_watch_root(self, value):
        self.calls["in_watch_root"] += 1
        return self.identity(value) is not None


class RefreshTests(unittest.TestCase):
    def test_paths_keep_results_and_each_instance_has_its_own_cache(self):
        def rebuild(root, *, force, block_on_lock):
            self.assertEqual(root, Path("."))
            self.assertTrue(block_on_lock)
            for selected_root in (Path("project"), Path("reference")):
                paths = watch._StoredSourcePaths(selected_root)
                expected = SourcePaths(selected_root)
                for value in ("src/file.rs", "src\\file.rs", "", None):
                    for name in ("normalize", "identity", "in_watch_root"):
                        self.assertEqual(getattr(paths, name)(value), getattr(expected, name)(value))
                    for anchor in (selected_root, Path("other")):
                        self.assertEqual(paths.absolute_identity(value, anchor), expected.absolute_identity(value, anchor))
                first_calls = dict(paths.calls)
                for value in ("src/file.rs", "src\\file.rs", "", None):
                    for name in ("normalize", "identity", "in_watch_root"):
                        getattr(paths, name)(value)
                    for anchor in (selected_root, Path("other")):
                        paths.absolute_identity(value, anchor)
                self.assertEqual(dict(paths.calls), first_calls)
                self.assertGreater(paths.calls["normalize"], 0)
            return True

        watch = SimpleNamespace(_StoredSourcePaths=SourcePaths, _rebuild_code=rebuild)
        self.assertTrue(refresh.refresh(watch))
        self.assertIs(watch._StoredSourcePaths, SourcePaths)

    def test_force_environment_retains_the_original_cli_behavior(self):
        for value, expected in (("", False), ("true", True), ("YES", True), ("0", False)):
            with self.subTest(value=value), patch.dict(refresh.os.environ, {"GRAPHIFY_FORCE": value}):
                def rebuild(root, *, force, block_on_lock):
                    self.assertEqual(force, expected)
                    return True
                watch = SimpleNamespace(_StoredSourcePaths=SourcePaths, _rebuild_code=rebuild)
                self.assertTrue(refresh.refresh(watch))

    def test_failure_and_exception_restore_the_original_path_class(self):
        for failure in (False, RuntimeError("failed rebuild")):
            def rebuild(*args, **kwargs):
                if isinstance(failure, Exception):
                    raise failure
                return failure

            watch = SimpleNamespace(_StoredSourcePaths=SourcePaths, _rebuild_code=rebuild)
            if isinstance(failure, Exception):
                with self.assertRaisesRegex(RuntimeError, "failed rebuild"):
                    refresh.refresh(watch)
            else:
                self.assertFalse(refresh.refresh(watch))
            self.assertIs(watch._StoredSourcePaths, SourcePaths)

    def test_unsupported_version_does_not_start_a_rebuild(self):
        with patch.object(refresh.sys, "argv", ["refresh.py"]), \
             patch.object(Path, "is_file", return_value=True), \
             patch.object(refresh, "import_module") as load, \
             patch.object(refresh.metadata, "version", return_value="other"), \
             patch.object(refresh, "build") as rebuild:
            self.assertEqual(refresh.main(), 1)
            rebuild.assert_not_called()
            load.assert_called_once_with("graphify.watch")

    def test_missing_tool_does_not_install_or_relaunch_the_current_interpreter(self):
        with patch.object(refresh.shutil, "which", return_value=None):
            with self.assertRaisesRegex(RuntimeError, "interpreter"):
                refresh.installed_python()
        result = SimpleNamespace(stdout=str(Path("tools")))
        with patch.object(refresh.shutil, "which", return_value="uv"), \
             patch.object(refresh.subprocess, "run", return_value=result) as run, \
             patch.object(Path, "is_file", return_value=False):
            with self.assertRaisesRegex(RuntimeError, "existing"):
                refresh.installed_python()
            run.assert_called_once_with(["uv", "tool", "dir"], capture_output=True, text=True, check=True)

    def test_relaunch_preserves_arguments_and_failure_status(self):
        missing = ModuleNotFoundError("graphify is absent", name="graphify")
        interpreter = Path("environment with spaces/python.exe")
        with patch.object(refresh.sys, "argv", ["refresh.py", "--graph", "reference", "--root", "target"]), \
             patch.object(Path, "is_file", return_value=True), \
             patch.object(refresh, "import_module", side_effect=missing), \
             patch.object(refresh, "installed_python", return_value=interpreter), \
             patch.object(refresh.subprocess, "run", return_value=SimpleNamespace(returncode=7)) as run:
            self.assertEqual(refresh.main(), 7)
            self.assertEqual(run.call_args.args, ([str(interpreter), str(Path(refresh.__file__).resolve()),
                                                   "--graph", "reference", "--root", "target"],))
            self.assertEqual(run.call_args.kwargs["env"]["PYTHONHASHSEED"], refresh.os.environ.get("PYTHONHASHSEED", "0"))

    def test_default_seed_requires_a_waited_child_but_explicit_seeds_rebuild_in_place(self):
        with patch.object(refresh.sys, "argv", ["refresh.py"]), \
             patch.object(Path, "is_file", return_value=True), \
             patch.object(refresh, "import_module"), \
             patch.object(refresh.metadata, "version", return_value=refresh.GRAPHIFY_VERSION), \
             patch.object(refresh, "build", return_value=True) as rebuild, \
             patch.object(refresh.subprocess, "run", return_value=SimpleNamespace(returncode=7)) as run, \
             patch.dict(refresh.os.environ):
            refresh.os.environ.pop("PYTHONHASHSEED", None)
            self.assertEqual(refresh.main(), 7)
            self.assertEqual(run.call_args.args, ([refresh.sys.executable, str(Path(refresh.__file__).resolve())],))
            self.assertEqual(run.call_args.kwargs["env"]["PYTHONHASHSEED"], "0")
            self.assertNotIn("PYTHONHASHSEED", refresh.os.environ)
            rebuild.assert_not_called()
            run.reset_mock()
            refresh.os.environ["PYTHONHASHSEED"] = "123"
            self.assertEqual(refresh.main(), 0)
            rebuild.assert_called_once()
            run.assert_not_called()

    def test_virtual_environment_can_share_its_underlying_binary_with_system_python(self):
        result = SimpleNamespace(stdout=str(Path("tool environments")))
        with patch.object(refresh.shutil, "which", return_value="uv"), \
             patch.object(refresh.subprocess, "run", return_value=result), \
             patch.object(Path, "is_file", return_value=True), \
             patch.object(Path, "resolve", return_value=Path(refresh.sys.executable)):
            expected = Path("tool environments/graphifyy") / (
                "Scripts/python.exe" if refresh.sys.platform == "win32" else "bin/python")
            self.assertEqual(refresh.installed_python(), expected)

    def test_missing_target_configuration_fails_before_loading_graphify(self):
        with patch.object(refresh.sys, "argv", ["refresh.py"]), \
             patch.object(Path, "is_file", return_value=False), \
             patch.object(refresh, "import_module") as load:
            with self.assertRaises(SystemExit) as error:
                refresh.main()
            self.assertEqual(error.exception.code, 2)
            load.assert_not_called()

    def test_real_child_starts_with_a_deterministic_seed_and_utf8_streams(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".graphifyignore").write_text("", encoding="utf-8")
            package = root / "graphify"
            package.mkdir()
            (package / "__init__.py").write_text("", encoding="utf-8")
            (package / "watch.py").write_text('''import json, os, sys
from contextlib import contextmanager
@contextmanager
def _rebuild_lock(*args, **kwargs):
    yield True
class _StoredSourcePaths:
    pass
def _rebuild_code(*args, **kwargs):
    print(json.dumps({"seed": os.environ.get("PYTHONHASHSEED"),
                      "hash": hash("graphify"), "encoding": sys.stdout.encoding}))
    print(chr(0x3c0))
    return True
''', encoding="utf-8")
            (package / "detect.py").write_text("", encoding="utf-8")
            metadata = root / "graphifyy-0.9.74.dist-info"
            metadata.mkdir()
            (metadata / "METADATA").write_text("Metadata-Version: 2.1\nName: graphifyy\nVersion: 0.9.74\n", encoding="utf-8")
            environment = dict(os.environ, PYTHONPATH=str(root), PYTHONIOENCODING="ascii")
            environment.pop("PYTHONHASHSEED", None)
            observed = []
            for _ in range(2):
                result = subprocess.run([sys.executable, str(Path(refresh.__file__).resolve()), "--root", str(root)],
                                        cwd=root, env=environment, capture_output=True, encoding="utf-8")
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn(chr(0x3c0), result.stdout)
                observed.append(json.loads(result.stdout.splitlines()[0]))
            self.assertEqual(observed[0], observed[1])
            self.assertEqual(observed[0]["seed"], "0")
            self.assertEqual(observed[0]["encoding"], "utf-8")


if __name__ == "__main__":
    unittest.main()
