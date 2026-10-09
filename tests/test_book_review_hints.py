"""Review configuration hints and mandatory TOC behavior."""

from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from urllib.parse import urlsplit

from playwright.sync_api import sync_playwright

REVIEW = Path(__file__).resolve().parents[1] / "tools/bootstrap/book-to-markdown/review"
sys.path.insert(0, str(REVIEW.parent))
from review.model import changes, load_input, toc_entries, toc_markdown, write_json


class ReviewHintsTest(unittest.TestCase):
    def book(self):
        return {"toc": {"depth": 2}, "chapters": [{"id": "chapter", "title": "Example",
            "markdown_file": "example.md", "elements": [{"id": "heading", "type": "heading",
            "heading_level": 2, "md_text": "## Example", "pdf_regions": []}]}]}

    def test_retired_visibility_cannot_disable_toc(self):
        book = self.book()
        book["toc"]["enabled"] = False
        self.assertEqual(len(toc_entries(book)), 1)
        self.assertIn("Table of Contents", toc_markdown(book))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(root / "working/book.json", book)
            loaded, _, _ = load_input(root / "working")
        self.assertEqual(loaded["toc"], {"depth": 2})
        updated = deepcopy(loaded)
        updated["toc"]["depth"] = 3
        self.assertEqual(changes(loaded, updated)[0]["accepted"], {"depth": 2})
        self.assertEqual(changes(loaded, updated)[0]["working"], {"depth": 3})

    def test_hover_compares_depth_and_heading_properties(self):
        accepted = self.book()
        working = deepcopy(accepted)
        working["toc"]["depth"] = 3
        for revision, snapshot in [("before", accepted), ("after", working)]:
            snapshot.update({"revision": revision, "render": {"chapters": [
                {"id": "toc", "document": "toc.html"},
                {"id": "chapter", "document": "chapter.html"}]}})
        status = {"accepted": accepted, "working": working, "can_accept": True,
            "authored_revision": "after", "pending": False, "error": None,
            "changes": changes(accepted, working), "heading_types": ["heading"], "pdf_pages": []}
        errors = []

        def respond(route):
            path = urlsplit(route.request.url).path
            if path == "/api/status":
                route.fulfill(content_type="application/json", body=json.dumps(status))
            elif path.endswith("toc.html"):
                route.fulfill(content_type="text/html", body=(
                    '<div data-element="generated-toc"><h1>Table of Contents</h1></div>'))
            elif path.endswith("chapter.html"):
                route.fulfill(content_type="text/html", body=(
                    '<div data-element="heading"><h2>Example</h2></div>'))
            elif path == "/katex/katex.min.css":
                route.fulfill(content_type="text/css", body="")
            else:
                route.fulfill(path=str(REVIEW / ("viewer.html" if path == "/" else Path(path).name)))

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(channel="msedge")
            page = browser.new_page(viewport={"width": 1600, "height": 900})
            page.add_init_script("window.EventSource=class {constructor(){}}")
            page.route("**/*", respond)
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.goto("http://book-review.test/")
            page.wait_for_function("state && !loading")
            self.assertEqual(page.locator("#status").inner_text(), "1 change")
            self.assertEqual(page.locator("#toc-enabled, #selection").count(), 0)
            page.locator('#working [data-element="generated-toc"]').hover()
            hint = page.locator("#element-hint")
            self.assertTrue(hint.is_visible())
            row = hint.locator("tr.changed")
            self.assertEqual(row.inner_text().split(), ["Depth", "2", "3"])
            self.assertIn("Changed: Depth", hint.inner_text())
            box = hint.bounding_box()
            self.assertGreaterEqual(box["x"], 0)
            self.assertLessEqual(box["x"] + box["width"], 1600)
            self.assertLessEqual(box["y"] + box["height"], 900)
            page.locator("header strong").hover()
            self.assertFalse(hint.is_visible())
            working["chapters"][0]["elements"][0]["heading_level"] = 3
            page.evaluate("value => {state.working.chapters[0].elements[0].heading_level=value;}", 3)
            page.locator('#working [data-element="heading"]').focus()
            self.assertIn("Heading level", hint.locator("tr.changed").inner_text())
            page.keyboard.press("Escape")
            self.assertFalse(hint.is_visible())
            self.assertEqual(errors, [])
            browser.close()
