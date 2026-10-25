"""Regression for render-error recovery on Windows consoles."""

from io import BytesIO, TextIOWrapper
from copy import deepcopy
from pathlib import Path
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

BOOK_TO_MARKDOWN = Path(__file__).resolve().parents[4] / "tools/bootstrap/book-to-markdown"
sys.path.insert(0, str(BOOK_TO_MARKDOWN))
from review.model import load_input, write_json
from review.service import ReviewState, replace_accepted


class FailingMathRenderer:
    settings = {"presentation": "html"}

    def __init__(self, *args):
        pass

    def render(self, *args):
        raise ValueError("KaTeX parse error: undefined command \\bad\u0332")

    def close(self):
        pass


class SnapshotRenderer:
    settings = {"presentation": "html"}

    def __init__(self, *args):
        pass

    def render(self, directory, book, assets, fingerprint, previous):
        revision = fingerprint[:12]
        output = directory / "renders" / revision
        output.mkdir(parents=True, exist_ok=True)
        (output / "chapter.html").write_text("<p>Complete revision</p>", encoding="utf-8")
        return {**deepcopy(book), "settings": self.settings, "revision": revision,
                "input_fingerprint": fingerprint, "metrics": {}}

    def close(self):
        pass


class RenderRecoveryTest(unittest.TestCase):
    def test_failed_main_book_update_restores_accepted(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book = {"toc": {"depth": 2}, "chapters": [
                {"id": "c", "title": "Chapter", "markdown_file": "chapter.md", "elements": [
                    {"id": "p", "type": "prose", "md_text": "Initial", "pdf_regions": [], "role": None}]}]}
            renderer = SnapshotRenderer()
            write_json(root / "book.json", book)
            for version in ("working", "accepted"):
                directory = root / version
                write_json(directory / "book.json", book)
                data, assets, fingerprint = load_input(directory)
                write_json(directory / "rendered.json", renderer.render(directory, data, assets, fingerprint, None))
            original_main = (root / "book.json").read_bytes()
            original_accepted = (root / "accepted/book.json").read_bytes()
            book["chapters"][0]["elements"][0]["md_text"] = "Changed"
            write_json(root / "working/book.json", book)
            data, assets, fingerprint = load_input(root / "working")
            snapshot = renderer.render(root / "working", data, assets, fingerprint, None)
            write_json(root / "working/rendered.json", snapshot)

            with patch.object(Path, "replace", side_effect=OSError("book.json locked")):
                with self.assertRaisesRegex(OSError, "book.json locked"):
                    replace_accepted(root, snapshot, publish_book=True)

            self.assertEqual((root / "book.json").read_bytes(), original_main)
            self.assertEqual((root / "accepted/book.json").read_bytes(), original_accepted)
            self.assertFalse((root / ".accepted-backup").exists())

    def test_later_render_preserves_newly_accepted_snapshot_and_its_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book = {"toc": {"depth": 2}, "chapters": [
                {"id": "c", "title": "Chapter", "markdown_file": "chapter.md", "elements": [
                    {"id": "p", "type": "prose", "md_text": "Initial", "pdf_regions": [], 'role': None}]}]}
            renderer = SnapshotRenderer()
            write_json(root / "book.json", book)
            for version in ("working", "accepted"):
                directory = root / version
                write_json(directory / "book.json", book)
                data, assets, fingerprint = load_input(directory)
                write_json(directory / "rendered.json", renderer.render(directory, data, assets, fingerprint, None))
            state = ReviewState(root)
            with patch("review.service.Renderer", SnapshotRenderer):
                worker = threading.Thread(target=state.render_loop)
                worker.start()
                try:
                    self.wait_for(lambda: state.status()["can_accept"])
                    book["chapters"][0]["elements"][0]["md_text"] = "Accepted update"
                    write_json(root / "working/book.json", book)
                    self.wait_for(lambda: state.status()["can_accept"])
                    self.assertEqual(load_input(root)[0]["chapters"][0]["elements"][0]["md_text"], "Initial")
                    accepted_revision = state.working["revision"]
                    state.accept({"revision": accepted_revision})
                    self.assertEqual((root / "book.json").read_bytes(), (root / "accepted/book.json").read_bytes())
                    book["chapters"][0]["elements"][0]["md_text"] = "Later working update"
                    write_json(root / "working/book.json", book)
                    self.wait_for(lambda: state.status()["can_accept"])
                    self.assertNotEqual(state.working["revision"], accepted_revision)
                    self.assertEqual(state.accepted["revision"], accepted_revision)
                    self.assertTrue((root / "accepted/renders" / state.accepted["revision"] / "chapter.html").exists())
                    self.assertEqual(state.accepted["chapters"][0]["elements"][0]["md_text"], "Accepted update")
                    self.assertEqual(load_input(root)[0]["chapters"][0]["elements"][0]["md_text"], "Accepted update")
                finally:
                    state.stopping.set()
                    worker.join(5)

    def test_unicode_math_error_does_not_stop_queue_on_ascii_console(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book = {"toc": {"depth": 2}, "chapters": [
                {"id": "c", "title": "Math", "markdown_file": "math.md", "elements": [
                    {"id": "p", "type": "prose", "md_text": "Valid", "pdf_regions": [], 'role': None}]}]}
            for version in ("working", "accepted"):
                write_json(root / version / "book.json", book)
                fingerprint = load_input(root / version)[2]
                write_json(root / version / "rendered.json", {
                    **book, "settings": FailingMathRenderer.settings,
                    "input_fingerprint": fingerprint, "revision": "valid"})
            state = ReviewState(root)
            valid = (root / "working/book.json").read_bytes()
            book["chapters"][0]["elements"][0]["md_text"] = "Invalid math"
            write_json(root / "working/book.json", book)
            output = TextIOWrapper(BytesIO(), encoding="ascii", errors="strict")
            with patch("review.service.Renderer", FailingMathRenderer), patch("sys.stdout", output):
                worker = threading.Thread(target=state.render_loop)
                worker.start()
                try:
                    self.wait_for(lambda: state.status()["error"])
                    (root / "working/book.json").write_bytes(valid)
                    self.wait_for(lambda: state.status()["can_accept"])
                    self.assertTrue(worker.is_alive())
                    self.assertEqual(state.working["revision"], "valid")
                finally:
                    state.stopping.set()
                    worker.join(5)

    def wait_for(self, condition):
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if condition():
                return
            time.sleep(.02)
        self.fail("Render queue did not recover")


if __name__ == "__main__":
    unittest.main()
