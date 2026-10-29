"""Publication prepares portable Markdown and images without changing review data."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.publish import publish_book


class BookPublicationTests(unittest.TestCase):
    def test_published_contents_link_to_headings_without_review_anchors(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book_root = root / "books" / "sample-book" / "variant"
            accepted = book_root / "accepted"
            accepted.mkdir(parents=True)
            recipe = {"toc": {"markdown_file": "contents.md"}, "chapters": [
                {"markdown_file": "chapter one.md", "elements": [
                    {"id": "heading_1", "type": "heading", "visible": True},
                    {"id": "paragraph_1", "type": "prose", "visible": True},
                ]},
            ]}
            source = json.dumps(recipe)
            (book_root / "book.json").write_text(source, encoding="utf-8")
            (accepted / "book.json").write_text(source, encoding="utf-8")
            (accepted / "contents.md").write_text(
                "# Contents\n\n- [[chapter one.md#^element-heading-u1|Horizontal Beam Position]]\n",
                encoding="utf-8",
            )
            (accepted / "chapter one.md").write_text(
                '<a id="element-heading_1"></a>\n\n# Horizontal Beam Position '
                '^element-heading-u1\n\n<a id="element-paragraph_1"></a>\n\nBody.\n',
                encoding="utf-8",
            )

            destination, _, _ = publish_book(book_root, root / "Reference")

            self.assertIn(
                "- [Horizontal Beam Position](chapter%20one.md#Horizontal%20Beam%20Position)",
                (destination / "contents.md").read_text(encoding="utf-8"),
            )
            self.assertEqual(
                (destination / "chapter one.md").read_text(encoding="utf-8"),
                "# Horizontal Beam Position\n\nBody.\n",
            )
            self.assertIn("#^element-heading-u1", (accepted / "contents.md").read_text(encoding="utf-8"))

    def test_repeated_heading_gets_a_unique_published_link(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book_root = root / "books" / "sample-book" / "variant"
            accepted = book_root / "accepted"
            accepted.mkdir(parents=True)
            recipe = {"toc": {"markdown_file": "contents.md"}, "chapters": [
                {"markdown_file": "chapter.md", "elements": []},
            ]}
            source = json.dumps(recipe)
            (book_root / "book.json").write_text(source, encoding="utf-8")
            (accepted / "book.json").write_text(source, encoding="utf-8")
            (accepted / "contents.md").write_text(
                "# Contents\n\n- [[chapter.md#^element-second|Repeat]]\n", encoding="utf-8"
            )
            (accepted / "chapter.md").write_text(
                '<a id="element-first"></a>\n\n## Repeat ^element-first\n\n'
                '<a id="element-second"></a>\n\n## Repeat ^element-second\n',
                encoding="utf-8",
            )

            destination, _, _ = publish_book(book_root, root / "Reference")
            self.assertIn("- [Repeat (2)](chapter.md#Repeat%20%282%29)",
                          (destination / "contents.md").read_text(encoding="utf-8"))
            self.assertEqual((destination / "chapter.md").read_text(encoding="utf-8"),
                             "## Repeat\n\n## Repeat (2)\n")
            self.assertIn("## Repeat ^element-second", (accepted / "chapter.md").read_text(encoding="utf-8"))

    def test_copies_accepted_files_and_description_sidecar(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book_root = root / "books" / "sample-book" / "variant"
            accepted = book_root / "accepted"
            (accepted / "assets").mkdir(parents=True)
            reference = root / "Reference"
            recipe = {"toc": {"markdown_file": "contents.md"}, "chapters": [
                {"markdown_file": "chapter.md", "elements": [
                    {"type": "graphic", "asset": "assets/picture.png",
                     "description": "First line\n\nSecond line"},
                    {"type": "graphic", "asset": "assets/other.png"},
                    {"type": "graphic", "asset": "assets/hidden.png", "visible": False,
                     "description": "Hidden image"},
                ]},
            ]}
            source = json.dumps(recipe)
            (book_root / "book.json").write_text(source, encoding="utf-8")
            (accepted / "book.json").write_text(source, encoding="utf-8")
            (accepted / "contents.md").write_text("# Contents\n", encoding="utf-8")
            (accepted / "chapter.md").write_text("![Image](assets/picture.png)\n", encoding="utf-8")
            (accepted / "assets" / "picture.png").write_bytes(b"image")
            (accepted / "assets" / "other.png").write_bytes(b"other")
            (accepted / "assets" / "hidden.png").write_bytes(b"hidden")
            (accepted / "rendered.json").write_text("{}", encoding="utf-8")

            destination, asset_count, description_count = publish_book(book_root, reference)

            self.assertEqual(destination, reference / "sample-book")
            self.assertEqual((asset_count, description_count), (2, 1))
            self.assertEqual((destination / "chapter.md").read_bytes(),
                             (accepted / "chapter.md").read_bytes())
            self.assertEqual((destination / "assets" / "picture.png.txt").read_text(encoding="utf-8"),
                             "First line\n\nSecond line\n")
            self.assertEqual({file.relative_to(destination).as_posix() for file in destination.rglob("*")
                              if file.is_file()},
                             {"contents.md", "chapter.md", "assets/picture.png",
                              "assets/other.png", "assets/picture.png.txt"})
            with self.assertRaises(FileExistsError):
                publish_book(book_root, reference)
            self.assertTrue((book_root / "book.json").exists())

    def test_requires_root_recipe_to_match_accepted(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book_root = root / "books" / "sample-book" / "variant"
            accepted = book_root / "accepted"
            accepted.mkdir(parents=True)
            (book_root / "book.json").write_text('{"toc": {"markdown_file": "contents.md"}, "chapters": []}',
                                                   encoding="utf-8")
            (accepted / "book.json").write_text('{"toc": {"markdown_file": "other.md"}, "chapters": []}',
                                                  encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "differs from Accepted"):
                publish_book(book_root, root / "Reference")
            self.assertFalse((root / "Reference").exists())
