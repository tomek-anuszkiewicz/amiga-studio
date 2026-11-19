"""Book Review's native writer ownership and Compose mount boundary."""

from importlib.util import module_from_spec, spec_from_file_location
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image

PROGRAM = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROGRAM))
from review.library import BookLibrary
from review.model import write_json
from review.ownership import BookInUseError, BookWriter


def mount_builder():
    spec = spec_from_file_location("compose_mounts", PROGRAM / "compose-mounts.py")
    module = module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class OwnershipTests(unittest.TestCase):
    def test_host_writer_refuses_an_owned_book_before_writing_and_releases_on_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            working = root / "working/book.json"
            write_json(working, {"toc": {}, "chapters": []})
            baseline = working.read_bytes()
            with BookWriter(root):
                command = subprocess.run([sys.executable, str(PROGRAM / "book_review.py"),
                                          "render", "--book", str(root)], capture_output=True, text=True)
                self.assertNotEqual(command.returncode, 0)
                self.assertIn("Book already has a writer", command.stderr)
                self.assertEqual(working.read_bytes(), baseline)
                self.assertTrue((root / ".book-review-writer").is_dir())
            failed = subprocess.run([sys.executable, str(PROGRAM / "book_review.py"),
                                     "export", "--book", str(root)], capture_output=True, text=True)
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn("Render the newest authored data", failed.stderr)
            self.assertFalse((root / ".book-review-writer").exists())
            self.assertEqual(working.read_bytes(), baseline)

    def test_library_refuses_a_host_owned_variant_and_releases_after_initialization_error(self):
        with tempfile.TemporaryDirectory() as temporary:
            library_root = Path(temporary)
            root = library_root / "manual/variant"
            write_json(root / "working/book.json", {"toc": {}, "chapters": []})
            library = BookLibrary(library_root)
            with BookWriter(root), self.assertRaisesRegex(BookInUseError, "Book already has a writer"):
                library.state("manual/variant")
            self.assertEqual(library.sessions, {})
            with patch("review.library.ReviewState", side_effect=ValueError("failed to initialize")):
                with self.assertRaisesRegex(ValueError, "failed to initialize"):
                    library.state("manual/variant")
            with BookWriter(root):
                self.assertEqual(library.sessions, {})

    def test_closing_a_library_releases_only_after_render_worker_exits(self):
        with tempfile.TemporaryDirectory() as temporary:
            library_root = Path(temporary)
            root = library_root / "manual/variant"
            write_json(root / "working/book.json", {"toc": {}, "chapters": []})
            stopped = []

            def render_loop(state):
                state.stopping.wait()
                with self.assertRaises(BookInUseError):
                    with BookWriter(root):
                        pass
                stopped.append(True)

            library = BookLibrary(library_root)
            with patch("review.service.ReviewState.render_loop", render_loop):
                library.state("manual/variant")
                library.close()
            self.assertEqual(stopped, [True])
            with BookWriter(root):
                self.assertEqual(library.sessions, {})


class MountTests(unittest.TestCase):
    def test_protected_sources_have_read_only_overlays_and_working_is_not_a_mount(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            library = base / "books"
            root = library / "manual/variant"
            write_json(root / "working/book.json", {"toc": {}, "chapters": []})
            write_json(root / "source.json", {"page_count": 1})
            write_json(root / "pdf-to-markdown/config.yaml", {})
            (root / "source.pdf").write_bytes(b"source fixture")
            image = root / "pdf/pages/page_0001.png"
            image.parent.mkdir(parents=True)
            Image.new("RGB", (4, 4), "white").save(image)
            write_json(root / "book.json", {"toc": {}, "chapters": []})
            unrelated = library / "not-imported/variant"
            write_json(unrelated / "book.json", {"toc": {}, "chapters": []})
            diagnostics = base / "logs"
            diagnostics.mkdir()
            config, auth = base / "config.yaml", base / "auth.json"
            config.write_text("llm: {}", encoding="utf-8")
            auth.write_text("private fixture payload", encoding="utf-8")

            service = mount_builder().overlay(library, diagnostics, config, auth)["services"]["book-review"]
            self.assertEqual(json.loads(service["environment"]["BOOK_REVIEW_PROTECTED_VARIANTS"]),
                             ["manual/variant"])
            self.assertEqual(set(entry["target"] for entry in service["volumes"]),
                             {"/library/manual/variant/" + name for name in
                              ("source.pdf", "source.json", "pdf-to-markdown", "pdf")} |
                             {"/run/secrets/book-review-auth", "/library/manual/variant"})
            self.assertEqual([item["target"] for item in service["volumes"] if not item["read_only"]],
                             ["/library/manual/variant"])
            self.assertTrue(all(not item["bind"]["create_host_path"] for item in service["volumes"]))
            self.assertNotIn("private fixture payload", json.dumps(service))
            self.assertEqual([item["id"] for item in BookLibrary(library, allowed=["manual/variant"]).catalog()],
                             ["manual/variant"])

    def test_immutable_inputs_cannot_also_be_inside_writable_mounts(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            library, diagnostics = base / "books", base / "logs"
            library.mkdir()
            diagnostics.mkdir()
            config = base / "config.yaml"
            config.touch()
            auth = diagnostics / "auth.json"
            auth.touch()
            with self.assertRaisesRegex(ValueError, "must be outside writable mount roots"):
                mount_builder().overlay(library, diagnostics, config, auth)
