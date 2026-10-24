"""Browser regression for shared horizontal scrolling of wide book tables."""

import json
from pathlib import Path
import unittest
from urllib.parse import urlsplit

from playwright.sync_api import sync_playwright

REVIEW = Path(__file__).resolve().parents[4] / "tools/bootstrap/book-to-markdown/review"


class BookOverflowTest(unittest.TestCase):
    def test_pdf_page_is_restored_after_reload(self):
        snapshot = {"revision": "test", "toc": {"enabled": False, "depth": 2},
                    "chapters": [{"id": "chapter", "title": "Pages", "elements": [
                        {"id": f"page-{number}", "type": "prose", "role": None,
                         "pdf_regions": [{"page": number, "bbox": [10, 10, 190, 60]}]}
                        for number in range(1, 5)]}],
                    "render": {"chapters": [{"id": "chapter", "document": "renders/test/chapter.html"}]}}
        status = {"working": snapshot, "accepted": snapshot, "can_accept": True,
                  "authored_revision": "test", "pending": False, "error": None, "changes": [],
                  "heading_types": [], "pdf_pages": [
                      {"page": number, "image": f"pdf/pages/page_{number:04d}.svg", "width": 200, "height": 800}
                      for number in range(1, 5)]}

        def respond(route):
            path = urlsplit(route.request.url).path
            if path == "/api/status":
                route.fulfill(content_type="application/json", body=json.dumps(status))
            elif path.endswith("chapter.html"):
                route.fulfill(content_type="text/html", body="".join(
                    f'<div data-element="page-{number}">Page {number}</div>' for number in range(1, 5)))
            elif path.endswith(".svg"):
                route.fulfill(content_type="image/svg+xml", body=(
                    '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="800"></svg>'))
            elif path == "/katex/katex.min.css":
                route.fulfill(content_type="text/css", body="")
            else:
                route.fulfill(path=str(REVIEW / ("viewer.html" if path == "/" else Path(path).name)))

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(channel="msedge")
            page = browser.new_page(viewport={"width": 1200, "height": 800})
            page.add_init_script("window.EventSource=class {constructor(){}}")
            page.route("**/*", respond)
            page.goto("http://book-review.test/")
            page.wait_for_function("state && !loading && nodes.pdf.length === 4")
            page.evaluate("panes.pdf.scrollTop = [...panes.pdf.querySelectorAll('.page')][2].offsetTop")
            page.wait_for_function("localStorage.getItem('book-review.pdf-page') === '3'")
            page.reload()
            page.wait_for_function("state && !loading && document.getElementById('pdf-location').textContent === 'Page 3'")
            self.assertEqual(page.evaluate("localStorage.getItem('book-review.pdf-page')"), "3")
            browser.close()

    def test_wide_tables_scroll_in_panel_while_text_and_images_follow_width(self):
        prose = "Ordinary paragraph text wraps to the panel width. " * 60
        cells = "".join(f"<td>REGISTER_{i:02d}_LONG_VALUE</td>" for i in range(12))
        table = f"<table><tbody><tr>{cells}</tr></tbody></table>"
        markup = (f'<div data-element="prose"><p>{prose}</p></div>'
                  f'<div data-element="table">{table}</div>'
                  '<div data-element="image"><img src="image.svg"></div>'
                  f'<div data-element="second-table">{table}</div>')
        elements = [{"id": identifier, "type": kind, "pdf_regions": [], 'role': None}
                    for identifier, kind in [("prose", "prose"), ("table", "table"),
                                             ("image", "graphic"), ("second-table", "table")]]
        snapshot = {"revision": "test", "toc": {"enabled": False, "depth": 2},
                    "chapters": [{"id": "chapter", "title": "Overflow", "elements": elements}],
                    "render": {"chapters": [{"id": "chapter", "document": "renders/test/chapter.html"}]}}
        status = {"working": snapshot, "accepted": snapshot, "can_accept": True,
                  "authored_revision": "test", "pending": False, "error": None,
                  "changes": [], "heading_types": [], "pdf_pages": []}
        requests, errors = [], []

        def respond(route):
            path = urlsplit(route.request.url).path
            requests.append(path)
            if path == "/api/status":
                route.fulfill(content_type="application/json", body=json.dumps(status))
            elif path.endswith("chapter.html"):
                route.fulfill(content_type="text/html", body=markup)
            elif path.endswith("image.svg"):
                route.fulfill(content_type="image/svg+xml", body=(
                    '<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="300">'
                    '<rect width="1200" height="300" fill="steelblue"/></svg>'))
            elif path == "/katex/katex.min.css":
                route.fulfill(content_type="text/css", body="")
            else:
                name = "viewer.html" if path == "/" else Path(path).name
                route.fulfill(path=str(REVIEW / name))

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(channel="msedge")
            page = browser.new_page(viewport={"width": 1200, "height": 800})
            page.add_init_script("window.EventSource=class {constructor(){}}")
            page.route("**/*", respond)
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.goto("http://book-review.test/")
            page.wait_for_function("state && !loading && nodes.working.length === 4")

            def geometry():
                return page.evaluate("""() => Object.fromEntries(['accepted','working'].map(version => {
                  const pane=panes[version], root=pane.querySelector('.chapter-content').shadowRoot;
                  const table=root.querySelector('table'), paragraph=root.querySelector('p'), image=root.querySelector('img');
                  return [version,{panelWidth:pane.clientWidth,scrollWidth:pane.scrollWidth,
                    tableWidth:table.getBoundingClientRect().width,zoom:getComputedStyle(table).zoom,
                    paragraphWidth:paragraph.getBoundingClientRect().width,paragraphHeight:paragraph.getBoundingClientRect().height,
                    imageWidth:image.getBoundingClientRect().width,
                    wrappers:[...root.querySelectorAll('[data-element]')].map(n=>getComputedStyle(n).overflowX)}];
                }))""")

            narrow = geometry()
            for version, result in narrow.items():
                self.assertEqual(result["zoom"], "1", version)
                self.assertGreater(result["tableWidth"], result["panelWidth"], version)
                self.assertGreater(result["scrollWidth"], result["panelWidth"], version)
                self.assertLessEqual(result["paragraphWidth"], result["panelWidth"], version)
                self.assertLessEqual(result["imageWidth"], result["panelWidth"], version)
                self.assertTrue(all(value == "visible" for value in result["wrappers"]), version)
            page.evaluate("panes.working.scrollLeft=150")
            page.wait_for_timeout(100)
            self.assertGreater(page.evaluate("panes.working.scrollLeft"), 0)
            requests.clear()
            page.set_viewport_size({"width": 1800, "height": 800})
            page.wait_for_timeout(200)
            wide = geometry()
            for version, result in wide.items():
                self.assertEqual(result["zoom"], "1", version)
                self.assertAlmostEqual(result["tableWidth"], narrow[version]["tableWidth"], delta=1)
                self.assertLess(result["paragraphHeight"], narrow[version]["paragraphHeight"], version)
                self.assertGreater(result["imageWidth"], narrow[version]["imageWidth"], version)
                self.assertLessEqual(result["paragraphWidth"], result["panelWidth"], version)
            self.assertEqual(requests, [], "Resize must not request new document content")
            # The PDF width is changed by the divider, then remains fixed across
            # window resizing and reload; wide tables still scroll in their panels.
            divider = page.locator("#pdf-splitter")
            box = divider.bounding_box()
            initial_width = page.locator("#pdf").evaluate("n=>n.parentElement.getBoundingClientRect().width")
            page.mouse.move(box["x"] + box["width"] / 2, box["y"] + 40)
            page.mouse.down()
            page.mouse.move(box["x"] + box["width"] / 2 + 120, box["y"] + 40, steps=4)
            page.mouse.up()
            self.assertEqual(page.evaluate("Number(localStorage.getItem('book-review.pdf-width'))"), initial_width + 120)
            page.set_viewport_size({"width": 900, "height": 800})
            page.wait_for_timeout(100)
            self.assertEqual(page.locator("#pdf").evaluate("n=>n.parentElement.getBoundingClientRect().width"), initial_width + 120)
            self.assertTrue(page.locator("main").evaluate("n=>n.scrollWidth>n.clientWidth"))
            page.locator("main").evaluate("n=>n.scrollLeft=n.scrollWidth")
            self.assertTrue(page.locator("#working").is_visible())
            self.assertEqual(requests, [], "Dragging the divider must not request rendering")
            page.reload()
            page.wait_for_function("state && !loading && nodes.working.length === 4")
            self.assertEqual(page.locator("#pdf-splitter").get_attribute("aria-valuenow"), str(int(initial_width + 120)))
            self.assertEqual(page.locator("#pdf").evaluate("n=>n.parentElement.getBoundingClientRect().width"), initial_width + 120)
            page.locator("main").evaluate("n=>n.scrollLeft=0")
            divider.focus()
            divider.press("ArrowRight")
            self.assertEqual(page.evaluate("Number(localStorage.getItem('book-review.pdf-width'))"), initial_width + 130)
            self.assertFalse(page.locator("body").evaluate("n=>n.classList.contains('resizing-pdf')"))
            self.assertEqual(errors, [])
            browser.close()


if __name__ == "__main__":
    unittest.main()
