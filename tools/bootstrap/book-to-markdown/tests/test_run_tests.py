"""Dependency setup must make the review tests runnable on a clean checkout."""

import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch


RUNNER = Path(__file__).resolve().parents[1] / "run_tests.py"
SPEC = importlib.util.spec_from_file_location("book_review_test_runner", RUNNER)
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class DependencySetupTests(unittest.TestCase):
    def test_missing_python_package_installs_pinned_requirements(self):
        with patch.object(runner.importlib.metadata, "version", side_effect=runner.importlib.metadata.PackageNotFoundError), \
             patch.object(runner.subprocess, "run") as execute:
            with redirect_stdout(io.StringIO()):
                runner.ensure_python_packages()
        execute.assert_called_once_with(
            [sys.executable, "-m", "pip", "install", "-r", str(runner.HERE / "requirements.txt")],
            cwd=runner.REPO_ROOT, check=True)

    def test_missing_katex_uses_locked_npm_install_without_scripts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "package.json").write_text('{"dependencies":{"katex":"0.16.22"}}', encoding="utf-8")

            def install(command, **kwargs):
                asset = root / "node_modules/katex/dist/katex.min.js"
                asset.parent.mkdir(parents=True)
                asset.write_text("installed", encoding="utf-8")
                (root / "node_modules/katex/package.json").write_text('{"version":"0.16.22"}',
                                                                         encoding="utf-8")
                self.assertEqual(command[1:3], ["ci", "--prefix"])
                self.assertIn("--ignore-scripts", command)
                self.assertEqual(kwargs["cwd"], runner.REPO_ROOT)

            with patch.object(runner, "HERE", root), \
                 patch.object(runner.shutil, "which", return_value="npm.cmd"), \
                 patch.object(runner.subprocess, "run", side_effect=install) as execute:
                with redirect_stdout(io.StringIO()):
                    runner.ensure_katex()
                    runner.ensure_katex()
            execute.assert_called_once()

    def test_install_failure_stops_before_test_discovery(self):
        archive = unittest.mock.Mock()
        with patch.object(runner, "run_reported", side_effect=lambda _producer, _argv, run, **_: run(archive)), \
             patch.object(runner, "ensure_python_packages", side_effect=subprocess.CalledProcessError(1, "pip")), \
             patch.object(runner, "ensure_katex") as katex, \
             patch.object(runner.subprocess, "run") as execute, \
             patch.object(sys, "argv", [str(RUNNER)]):
            with redirect_stderr(io.StringIO()):
                self.assertEqual(runner.main(), 1)
        katex.assert_not_called()
        execute.assert_not_called()
        archive.phase.assert_called_with("dependency setup")
        archive.event.assert_called_once_with("launcher_error", detail=str(subprocess.CalledProcessError(1, "pip")),
                                             native_exit_code=1)


class LifecycleTests(unittest.TestCase):
    def test_reports_two_contexts_and_closes_the_browser_before_renderer(self):
        suite_path = RUNNER.with_name("tests") / "run_suite.py"
        sys.path.insert(0, str(suite_path.parent))
        self.addCleanup(sys.path.remove, str(suite_path.parent))
        spec = importlib.util.spec_from_file_location("book_review_report_suite", suite_path)
        suite_module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(suite_module)
        import browser_session

        observed = []
        playwright = unittest.mock.Mock()
        browser = playwright.chromium.launch.return_value
        playwright.chromium.launch.side_effect = lambda **_: observed.append("browser start") or browser
        playwright.stop.side_effect = lambda: observed.append("playwright stop")
        browser.close.side_effect = lambda: observed.append("browser close")
        contexts = [unittest.mock.Mock(), unittest.mock.Mock()]
        for index, context in enumerate(contexts, 1):
            context.close.side_effect = lambda index=index: observed.append(f"context {index} close")
        pending_contexts = iter(contexts)

        def new_context(**_):
            context = next(pending_contexts)
            observed.append(f"context {contexts.index(context) + 1} start")
            return context

        browser.new_context.side_effect = new_context

        class Ui(unittest.TestCase):
            def test_first(self):
                with browser_session.browser_page(viewport={"width": 800, "height": 600}) as page:
                    self.assertIs(page, contexts[0].new_page.return_value)

            def test_second(self):
                with browser_session.browser_page(viewport={"width": 800, "height": 600}) as page:
                    self.assertIs(page, contexts[1].new_page.return_value)

        class Renderer(unittest.TestCase):
            def test_render(self):
                observed.append("renderer")
                self.fail("fixture renderer failure")

        Renderer.__module__ = "test_book_text_recipe"
        selected = unittest.TestSuite([Ui("test_first"), Ui("test_second"), Renderer("test_render")])
        archive = unittest.mock.Mock()
        archive.attachment.return_value = None
        with patch.object(suite_module.unittest.TestLoader, "discover", return_value=selected), \
             patch.object(browser_session, "sync_playwright") as start, \
             redirect_stderr(io.StringIO()), redirect_stdout(io.StringIO()):
            start.return_value.start.return_value = playwright
            self.assertEqual(suite_module.execute(["-q"], archive), 1)
        self.assertEqual(observed, ["browser start", "context 1 start", "context 1 close",
                                    "context 2 start", "context 2 close", "browser close",
                                    "playwright stop", "renderer"])
        results = [call.kwargs for call in archive.event.call_args_list if call.args == ("case_finished",)]
        self.assertEqual([row["outcome"] for row in results], ["passed", "passed", "failure"])
        self.assertIn("fixture renderer failure", results[-1]["detail"])
        self.assertTrue(all(row["seconds"] is not None for row in results))


if __name__ == "__main__":
    unittest.main()
