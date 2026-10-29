"""Frontmatter gate includes local reference books regardless of Git status."""

from pathlib import Path
import sys
import tempfile
import unittest


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
from check_obsidian_frontmatter import check_vault


class FrontmatterTests(unittest.TestCase):
    def test_scans_ignored_reference_book_and_reports_missing_properties(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            vault = root / "Obsidian" / "Amiga"
            book = vault / "Reference" / "Local Book"
            book.mkdir(parents=True)
            (root / ".gitignore").write_text("/Obsidian/Amiga/Reference/Local Book/\n", encoding="utf-8")
            chapter = book / "01 Chapter.md"
            chapter.write_text("# Chapter\n", encoding="utf-8")
            issues, count = check_vault(root)
            self.assertEqual((count, issues), (1, [(chapter, "missing opening YAML frontmatter marker on line 1")]))

            chapter.write_text("---\ntitle: Chapter\nbook: Local Book\nchapter: Chapter 1\n"
                               "tags: [amiga, reference, book, chapter, topic]\n---\n# Chapter\n", encoding="utf-8")
            self.assertEqual(check_vault(root), ([], 1))

            chapter.write_text("---\ntags: [amiga, reference]\n---\n# Chapter\n", encoding="utf-8")
            self.assertEqual(check_vault(root)[0], [(chapter, "missing title property")])

            chapter.write_text("---\ntitle: Chapter\n---\n# Chapter\n", encoding="utf-8")
            self.assertEqual(check_vault(root)[0], [(chapter, "missing tags property")])

            chapter.write_text("---\ntitle: Chapter\ntags: [amiga, reference]\n# Chapter\n", encoding="utf-8")
            self.assertEqual(check_vault(root)[0], [(chapter, "missing closing YAML frontmatter marker")])

            chapter.write_text("---\ntitle: Chapter\nmanual: Local Book\nchapter: Chapter 1\n"
                               "tags: [amiga, reference, book, chapter]\n---\n# Chapter\n", encoding="utf-8")
            self.assertEqual(check_vault(root)[0], [(chapter, "obsolete manual property; use book")])

            chapter.write_text("---\ntitle: Chapter\ntags: [amiga, reference, book, chapter]\n"
                               "---\n# Chapter\n", encoding="utf-8")
            self.assertEqual(check_vault(root)[0], [(chapter, "missing book property")])

            chapter.write_text("---\ntitle: Chapter\nbook: Local Book\nchapter: Chapter 1\n"
                               "tags:\n  - reference\n  - book\n---\n# Chapter\n", encoding="utf-8")
            self.assertEqual(check_vault(root)[0], [(chapter, "missing chapter tag")])


if __name__ == "__main__":
    unittest.main()
