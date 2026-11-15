"""Keep navigation, files and edits isolated when one server hosts two books."""

from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import urlopen

from PIL import Image
from browser_session import browser_page

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.library import BookLibrary, LibraryHandler
from review.model import read_json, write_json


class BookLibraryTests(unittest.TestCase):
    def test_switching_books_preserves_page_and_width_without_redirecting_edits(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ("first", "second"):
                variant = root / name / "variant"
                write_json(variant / "working/book.json", {
                    "toc": {"enabled": False, "depth": 2}, "chapters": [{
                        "id": "chapter", "title": name, "markdown_file": "chapter.md",
                        "elements": [{"id": f"page-{page}", "type": "prose", "role": None,
                                      "md_text": f"{name} page {page}",
                                      "pdf_regions": [{"page": page, "bbox": [0, 0, 200, 100]}]}
                                     for page in range(1, 5)]}]})
                (variant / "pdf/pages").mkdir(parents=True)
                for page in range(1, 5):
                    Image.new("RGB", (200, 800), "white").save(variant / f"pdf/pages/page_{page:04d}.png")
            # An accepted recipe alone is not an imported editor workspace.
            write_json(root / "not-imported/variant/book.json", {"toc": {}, "chapters": []})
            library = BookLibrary(root)
            self.assertEqual(library.catalog(), [
                {"id": "first/variant", "name": "first"}, {"id": "second/variant", "name": "second"}])
            server = ThreadingHTTPServer(("127.0.0.1", 0), LibraryHandler)
            server.daemon_threads = True
            server.library = library
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            base = f"http://127.0.0.1:{server.server_port}"
            try:
                with urlopen(base + "/api/status") as response:
                    self.assertEqual(response.status, 200)
                self.assertEqual(library.sessions, {})
                for path in ("/?book=missing", "/books/missing/variant/files/../../book.json"):
                    with self.assertRaises(HTTPError) as error:
                        urlopen(base + path)
                    self.assertEqual(error.exception.code, 404)
                with browser_page(viewport={"width": 1600, "height": 950}) as page:
                    errors = []
                    page.on("pageerror", lambda error: errors.append(str(error)))
                    page.goto(base)
                    self.assertEqual(page.locator("#status").inner_text(), "Choose a book to review")
                    self.assertFalse(page.locator("#show-hidden").is_enabled())
                    self.assertEqual(library.sessions, {})
                    page.select_option("#book", "first/variant")
                    page.wait_for_function("state?.can_accept && !loading && nodes.pdf.length === 4")
                    self.assertEqual(page.evaluate("bookId"), "first/variant")
                    page.evaluate("setPdfWidth(710);savePdfWidth();panes.pdf.scrollTop=panes.pdf.querySelectorAll('.page')[2].offsetTop")
                    page.wait_for_function("localStorage.getItem(pdfPageKey) === '3'")
                    page.check("#show-hidden")
                    page.select_option("#book", "second/variant")
                    page.wait_for_function("state?.can_accept && !loading && nodes.pdf.length === 4")
                    self.assertEqual(page.evaluate("bookId"), "second/variant")
                    self.assertEqual(page.evaluate("pdfWidth"), 600)
                    self.assertFalse(page.is_checked("#show-hidden"))
                    self.assertEqual(page.locator("#pdf-location").inner_text(), "Page 1")
                    other = page.context.new_page()
                    other.goto(base + "/?book=first%2Fvariant")
                    other.wait_for_function("state?.can_accept && !loading")
                    self.assertEqual(other.evaluate("pdfWidth"), 710)
                    other.wait_for_function("document.getElementById('pdf-location').textContent === 'Page 3'")
                    # An old tab's command must still edit its own book.
                    page.evaluate("post('/api/toc',{depth:3,expected_revision:state.authored_revision})")
                    other.evaluate("post('/api/toc',{depth:1,expected_revision:state.authored_revision})")
                    self.assertEqual(read_json(root / "first/variant/working/book.json")["toc"]["depth"], 1)
                    self.assertEqual(read_json(root / "second/variant/working/book.json")["toc"]["depth"], 3)
                    page.goto(base)
                    page.wait_for_function("bookId === 'first/variant' && state?.can_accept && !loading")
                    self.assertEqual(page.evaluate("pdfWidth"), 710)
                    page.wait_for_function("document.getElementById('pdf-location').textContent === 'Page 3'")
                    self.assertEqual(errors, [])
            finally:
                server.shutdown()
                server.server_close()
                library.close()
                thread.join(timeout=5)

    def test_catalog_distinguishes_variants_and_rejects_unknown_books(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for variant in ("aaa", "bbb"):
                write_json(root / "manual" / variant / "working/book.json", {"toc": {}, "chapters": []})
            library = BookLibrary(root)
            self.assertEqual([book["name"] for book in library.catalog()], ["manual (aaa)", "manual (bbb)"])
            for book_id in ("../outside", "manual/missing"):
                with self.assertRaises(FileNotFoundError):
                    library.state(book_id)
            self.assertEqual(library.sessions, {})


if __name__ == "__main__":
    unittest.main()
