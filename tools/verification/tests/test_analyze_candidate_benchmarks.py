"""Reject malformed counts and undersized accepted benchmark series."""

from pathlib import Path
import statistics
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from analyze_candidate_benchmarks import validate_measurement_series


def measurement(elapsed_ns=400_000_000, target_count=700_000):
    ns = elapsed_ns / target_count
    samples = [
        {"attempt": 0, "ordinal": ordinal, "blocks": 1000,
         "target_count": target_count, "elapsed_ns": elapsed_ns, "ns_per_target": ns}
        for ordinal in range(3)
    ]
    return {
        "case_id": "synthetic-contract", "series": samples,
        "blocks_per_series": 1000, "target_duration_capped": False,
        "status": "measured", "dispersion_pct": 0.0,
        "selected_median_ns_per_target": statistics.median(sample["ns_per_target"] for sample in samples),
    }


class CandidateReportContracts(unittest.TestCase):
    def test_valid_fixed_count_series_is_accepted(self):
        row = measurement()
        self.assertEqual(len(validate_measurement_series(row, 700)), 3)

    def test_declared_target_count_must_match_actual_blocks(self):
        row = measurement(target_count=699_000)
        with self.assertRaisesRegex(ValueError, "invalid target divisor"):
            validate_measurement_series(row, 700)

    def test_short_series_cannot_be_reported_as_valid(self):
        row = measurement(elapsed_ns=70_000_000)
        with self.assertRaisesRegex(ValueError, "accepted a short or long series"):
            validate_measurement_series(row, 700)

    def test_unstable_series_cannot_publish_a_median(self):
        row = measurement()
        row["status"] = "unstable"
        with self.assertRaisesRegex(ValueError, "invalid sample accepted"):
            validate_measurement_series(row, 700)


if __name__ == "__main__":
    unittest.main()
