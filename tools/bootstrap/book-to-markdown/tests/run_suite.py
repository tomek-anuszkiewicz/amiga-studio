"""Run Book Review contracts with one browser process for the selected suite."""

from pathlib import Path
import sys
import unittest

REPO_ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(REPO_ROOT / "tools/verification"))
from browser_session import browser_suite, close_browser
from case_report import ReportArgumentParser, run_reported
from run_unittest import ReportingResult, ReportingRunner


class BookReviewResult(ReportingResult):
    def startTest(self, test):
        # Renderer starts its own synchronous Playwright session on this thread.
        if test.__class__.__module__ == "test_book_text_recipe":
            close_browser()
        super().startTest(test)


def execute(argv, report):
    report.phase("discovery")
    parser = ReportArgumentParser(report, description=__doc__)
    parser.add_argument("-q", "--quiet", action="store_true")
    parser.add_argument("-p", "--pattern", default="test*.py")
    parser.add_argument("-k", "--case", action="append")
    args = parser.parse_args(argv)
    loader = unittest.TestLoader()
    if args.case:
        loader.testNamePatterns = [case if "*" in case else f"*{case}*" for case in args.case]
    suite = loader.discover(str(Path(__file__).resolve().parent), pattern=args.pattern)
    with browser_suite():
        result = ReportingRunner(report, verbosity=0 if args.quiet else 1,
                                 resultclass=BookReviewResult).run(suite)
    return 0 if result.wasSuccessful() else 1


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    return run_reported("Book Review suite", [sys.executable, str(Path(__file__).resolve()), *argv],
                        lambda report: execute(argv, report), cwd=REPO_ROOT)


if __name__ == "__main__":
    sys.exit(main())
