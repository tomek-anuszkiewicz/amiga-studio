"""Protect existing books when publishing completed conversion artifacts."""

from pathlib import Path
import contextlib
import importlib.util
import io
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap"))
from conversion.publication import book_directory, check_destination, publish_output


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.output = self.root / "Book-tmp/workspace/14_link_toc"
        self.output.mkdir(parents=True)
        (self.output / "Chapter.md").write_text("# Chapter\n", encoding="utf-8")
        (self.output / "assets").mkdir()
        (self.output / "assets/figure.svg").write_text("<svg/>", encoding="utf-8")
        (self.output / ".metrics.json").write_text("{}", encoding="utf-8")
        self.book = self.root / "Book"

    def test_infers_book_from_staged_pdf_workspace_and_html_crawl(self):
        self.assertEqual(book_directory(self.output.parent), self.book)
        self.assertEqual(book_directory(self.root / "Book-tmp/live"), self.book)
        with self.assertRaises(ValueError):
            book_directory(self.root / "ordinary-workspace")

    def test_nonempty_destination_is_rejected_without_changing_files(self):
        self.book.mkdir()
        sentinel = self.book / ".keep"
        sentinel.write_bytes(b"original")
        with self.assertRaises(FileExistsError):
            check_destination(self.book)
        with self.assertRaises(FileExistsError):
            publish_output(self.output, self.book)
        self.assertEqual(sentinel.read_bytes(), b"original")
        self.assertEqual(list(self.book.iterdir()), [sentinel])

    def test_copies_markdown_and_assets_to_missing_or_empty_book(self):
        for exists in (False, True):
            destination = self.root / f"Published-{exists}"
            if exists:
                destination.mkdir()
            publish_output(self.output, destination)
            self.assertEqual((destination / "Chapter.md").read_bytes(), (self.output / "Chapter.md").read_bytes())
            self.assertTrue((destination / "assets/figure.svg").is_file())
            self.assertFalse((destination / ".metrics.json").exists())
        self.assertTrue((self.output / "Chapter.md").is_file())

    def test_destination_filled_during_preparation_is_preserved(self):
        import shutil
        original_copy = shutil.copy2

        def concurrent_write(source, destination):
            self.book.mkdir(exist_ok=True)
            (self.book / "original.md").write_text("original", encoding="utf-8")
            return original_copy(source, destination)

        with patch("conversion.publication.shutil.copy2", side_effect=concurrent_write):
            with self.assertRaises(FileExistsError):
                publish_output(self.output, self.book)
        self.assertEqual((self.book / "original.md").read_text(encoding="utf-8"), "original")
        self.assertFalse((self.book / "Chapter.md").exists())

    def test_copy_failure_leaves_empty_destination_and_source_intact(self):
        self.book.mkdir()
        with patch("conversion.publication.shutil.copy2", side_effect=OSError("copy failed")):
            with self.assertRaises(OSError):
                publish_output(self.output, self.book)
        self.assertEqual(list(self.book.iterdir()), [])
        self.assertTrue((self.output / "Chapter.md").is_file())

    def load_pipeline(self, kind):
        root = Path(__file__).resolve().parents[1]
        spec = importlib.util.spec_from_file_location(f"publish_{kind}", root / f"tools/bootstrap/{kind}-to-markdown/pipeline.py")
        pipeline = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pipeline)
        return root, pipeline

    def test_both_pipelines_reject_occupied_book_before_inference_or_cleanup(self):
        self.book.mkdir()
        sentinel = self.book / "original.md"
        sentinel.write_text("original", encoding="utf-8")
        for kind in ("pdf", "html"):
            root, pipeline = self.load_pipeline(kind)
            argv = ["pipeline.py", "--publish", "--config", str(root / f"tools/bootstrap/{kind}-to-markdown/config.yaml")]
            if kind == "pdf":
                argv += ["--workspace", str(self.output.parent)]
                client_name = "CodexTransport"
            else:
                source = self.root / "Book-tmp/source.html"
                source.write_text("<p>Source</p>", encoding="utf-8")
                argv += ["--input", str(source)]
                client_name = "CodexClient"
            with patch.object(sys, "argv", argv), patch.object(pipeline, client_name) as client:
                with self.assertRaises(FileExistsError):
                    pipeline.main()
                client.assert_not_called()
        self.assertEqual(sentinel.read_text(encoding="utf-8"), "original")
        self.assertTrue((self.output / "Chapter.md").is_file())

    def test_both_public_clis_reject_output_dir(self):
        for kind in ("pdf", "html"):
            root, pipeline = self.load_pipeline(kind)
            argv = ["pipeline.py", "--config", str(root / f"tools/bootstrap/{kind}-to-markdown/config.yaml"),
                    "--output-dir", str(self.book)]
            if kind == "html":
                argv += ["--input", str(self.root)]
            with patch.object(sys, "argv", argv), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as error:
                    pipeline.main()
                self.assertEqual(error.exception.code, 2)

    def test_validated_pdf_resume_publishes_workspace_output(self):
        root, pipeline = self.load_pipeline("pdf")
        state = {"source": {"name": "source.pdf", "pages": None},
                 "stages": {stage["id"]: {} for stage in pipeline.STAGE_REGISTRY}}
        argv = ["pipeline.py", "--workspace", str(self.output.parent), "--resume", "--publish",
                "--config", str(root / "tools/bootstrap/pdf-to-markdown/config.yaml")]
        with patch.object(sys, "argv", argv), patch.object(pipeline, "read_state", return_value=state), \
             patch.object(pipeline, "first_incomplete_stage", return_value=len(pipeline.STAGE_REGISTRY)), \
             patch.object(pipeline, "validate_completed_stages") as validate, patch.object(pipeline, "restore_shared"):
            pipeline.main()
            self.assertEqual(len(validate.call_args.args[1]), len(pipeline.STAGE_REGISTRY))
        self.assertTrue((self.book / "Chapter.md").is_file())

    def test_html_keeps_local_output_until_publish_without_parent_mirror(self):
        root, pipeline = self.load_pipeline("html")
        crawl = self.root / "Book-tmp/live"
        crawl.mkdir()
        (crawl / "index.html").write_text("<p>Source</p>", encoding="utf-8")
        argv = ["pipeline.py", "--input", str(crawl),
                "--config", str(root / "tools/bootstrap/html-to-markdown/config.yaml")]
        with patch.object(pipeline, "CodexClient") as client_type, patch.object(pipeline, "extract_assets"):
            client = client_type.return_value.__enter__.return_value
            client.generate_text.return_value = "---\ntitle: Book\n---\n# Book\n"
            client.call_count, client.cached_call_count, client.metrics = 1, 0, []
            with patch.object(sys, "argv", argv):
                pipeline.main()
            self.assertTrue((self.root / "Book-tmp/workspace/html_to_markdown/Book.md").is_file())
            self.assertFalse(self.book.exists())
            self.assertFalse((self.root / "Book.md").exists())
            with patch.object(sys, "argv", argv + ["--publish"]):
                pipeline.main()
        self.assertTrue((self.book / "Book.md").is_file())
        self.assertFalse((self.book / ".conversion-metrics.json").exists())

    def test_bootstrap_rejects_occupied_book_before_download(self):
        root = Path(__file__).resolve().parents[1]
        target = self.root / "Instruction Prefetch on the Motorola 68000 Processor"
        target.mkdir()
        sentinel = target / ".keep"
        sentinel.write_bytes(b"original")
        result = subprocess.run(["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
                                 str(root / "tools/bootstrap/bootstrap_documentation.ps1"),
                                 "-Destination", str(self.root), "-Prefetch", "-Markdown", "-Publish"],
                                capture_output=True, text=True, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Publication destination is not an empty directory", result.stderr)
        self.assertFalse(target.with_name(target.name + "-tmp").exists())
        self.assertEqual(sentinel.read_bytes(), b"original")


if __name__ == "__main__":
    unittest.main()
