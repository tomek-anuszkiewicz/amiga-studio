"""Tooltip-free element hover/focus and mandatory TOC behavior."""

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

    def test_hover_and_focus_show_no_tooltip(self):
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
            self.assertEqual(page.locator('#element-hint, [role="tooltip"]').count(), 0)
            page.locator("header strong").hover()
            page.locator('#working [data-element="heading"]').focus()
            self.assertEqual(page.locator('#element-hint, [role="tooltip"]').count(), 0)
            page.locator('#accepted [data-element="heading"]').hover()
            self.assertEqual(page.locator('#element-hint, [role="tooltip"]').count(), 0)
            page.keyboard.press("Escape")
            self.assertEqual(errors, [])
            browser.close()
