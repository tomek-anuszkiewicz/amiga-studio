"""Run Book Review contracts with one browser process for the selected suite."""

import argparse
from pathlib import Path
import sys
import unittest

from browser_session import browser_suite, close_browser


class BookReviewResult(unittest.TextTestResult):
    def startTest(self, test):
        # Renderer starts its own synchronous Playwright session on this thread.
        if test.__class__.__module__ == "test_book_text_recipe":
            close_browser()
        super().startTest(test)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-q", "--quiet", action="store_true")
    parser.add_argument("-p", "--pattern", default="test*.py")
    parser.add_argument("-k", "--case")
    args = parser.parse_args()
    loader = unittest.TestLoader()
    if args.case:
        pattern = args.case if "*" in args.case else f"*{args.case}*"
        loader.testNamePatterns = [pattern]
    suite = loader.discover(str(Path(__file__).resolve().parent), pattern=args.pattern)
    with browser_suite():
        result = unittest.TextTestRunner(
            verbosity=0 if args.quiet else 1, resultclass=BookReviewResult).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
