"""Check timestamp interpretation and bounded artifact traversal."""

from datetime import date, timedelta
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest


SOURCE = Path(__file__).resolve().parents[1] / "report.py"
SPEC = importlib.util.spec_from_file_location("artifact_report", SOURCE)
report = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = report
SPEC.loader.exec_module(report)


class ArtifactReportTests(unittest.TestCase):
    def test_rewritten_and_copied_files_use_the_later_date(self):
        old, new = date(2026, 10, 1), date(2026, 11, 13)
        usage = report.Usage()
        usage.add(Path("target/debug/deps/rewritten.bin"), 30, old, new)
        usage.add(Path("target/debug/deps/copied.bin"), 70, new, old)
        self.assertEqual(usage.created, {old: 30, new: 70})
        self.assertEqual(usage.modified, {old: 70, new: 30})
        self.assertEqual(usage.latest, {new: 100})
        self.assertEqual(usage.size, 100)

    def test_missing_creation_time_is_explicit(self):
        day = date(2026, 11, 13)
        usage = report.Usage()
        usage.add(Path(".agent/tmp/task/report.json"), 17, None, day)
        self.assertEqual(usage.created, {})
        self.assertEqual(usage.latest, {day: 17})
        self.assertIn("Creation time unavailable for 1 files", report.render(".agent/tmp", usage))

    def test_continuous_work_does_not_hide_the_daily_distribution(self):
        usage = report.Usage()
        for offset in range(14):
            day = date(2026, 11, 1) + timedelta(days=offset)
            usage.add(Path(f"target/debug/{offset}.bin"), report.GIB, day, day)
        lines = report.render("target", usage).splitlines()
        rows = [line for line in lines if line.startswith("2026-11-")]
        self.assertEqual(len(rows), 14)
        self.assertTrue(rows[0].startswith("2026-11-01 "))
        self.assertTrue(rows[-1].startswith("2026-11-14 "))

    def test_scan_counts_only_files_and_leaves_contents_unchanged(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            target = repo / "target"
            (target / "debug/deps").mkdir(parents=True)
            artifact = target / "debug/deps/test.bin"
            artifact.write_bytes(b"test data")
            before = artifact.stat().st_mtime_ns
            usage = report.scan(target, repo)
            self.assertEqual((usage.size, usage.files, usage.errors), (9, 1, []))
            self.assertEqual(usage.directories["target/debug/deps"].size, 9)
            self.assertEqual(artifact.read_bytes(), b"test data")
            self.assertEqual(artifact.stat().st_mtime_ns, before)
            self.assertEqual(report.scan(repo / ".agent/tmp", repo).size, 0)

    def test_links_are_not_included_in_the_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            target = repo / "target"
            target.mkdir()
            outside = repo / "outside"
            outside.mkdir()
            (outside / "data.bin").write_bytes(b"not an artifact")
            link = target / "external"
            try:
                link.symlink_to(outside, target_is_directory=True)
            except OSError as error:
                self.skipTest(f"Symbolic links unavailable: {error}")
            usage = report.scan(target, repo)
            self.assertEqual((usage.size, usage.files, usage.skipped), (0, 0, 1))


if __name__ == "__main__":
    unittest.main()
