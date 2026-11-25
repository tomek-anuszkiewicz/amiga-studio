"""The score uses the median and preserves the approved references."""

import hashlib
import json
import math
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from run_mixed_scene_fps import (
    CASES,
    GAME_CASES,
    SCHEMA,
    compare_all,
    game_media_identity,
    game_sample,
    install_browser_media,
    main,
    measure,
    record_game_sample,
    validate_samples,
    verify_matching_work,
)


def game_run(count=120, seconds=10.5):
    return {
        "generation": 2,
        "sample": {"warmup_start_field": 968, "start_field": 1000,
                   "end_field": 1000 + count, "seconds": seconds, "speed": 4.0},
        "boot_first": {"cck": 100, "field_number": 960, "fields": 1, "work_loops": 10, "fault": 0},
        "guest_before": {"cck": 150, "field_number": 968, "fields": 2, "work_loops": 20, "fault": 0},
        "guest_after": {"cck": 500, "field_number": 1010 + count,
                        "fields": 4, "work_loops": 40, "fault": 0},
        "renderer": "fixture renderer",
        "display": {"pixels_per_point": 1.0},
        "display_refresh_hz": 60,
    }


def running_case(case, rates=(120, 100, 10)):
    row = {"compatibility": {"sampling": {"fields": 120}}}
    if case in GAME_CASES:
        row["compatibility"]["sampling"] = {
            "seconds": 10, "speed": 4, "metric": "completed-emulated-fields",
        }
        for rate in rates:
            count = math.ceil(rate * 10)
            record_game_sample(row, game_run(count, count / rate))
    else:
        row["samples"] = validate_samples([{"count": round(rate * 100), "seconds": 100}
                                           for rate in rates])
    return row


class MixedSceneFpsTests(unittest.TestCase):
    def setUp(self):
        directory = self.enterContext(tempfile.TemporaryDirectory())
        self.path = Path(directory) / "reference.json"
        self.cases = {key: running_case(key) for key in CASES}
        self.references = {key: {"reference_fps": 100, "tolerance": 0.05,
                                 "compatibility": row["compatibility"].copy()}
                           for key, row in self.cases.items()}

    def save_reference(self, schema=SCHEMA):
        content = json.dumps({"schema": schema, "cases": self.references})
        self.path.write_text(content, encoding="utf-8")
        return content.encode()

    def compare(self):
        return compare_all({"schema": SCHEMA, "cases": self.cases}, self.path)

    def test_both_tolerance_edges_pass_and_excess_in_either_direction_fails(self):
        content = self.save_reference()
        self.cases["FPS-01"] = running_case("FPS-01", (120, 95, 10))
        self.cases["FPS-02"] = running_case("FPS-02", (200, 105, 10))
        rows, passed = self.compare()
        self.assertTrue(passed)
        self.assertEqual(len(rows), 4)
        self.assertEqual(rows["FPS-01"]["median_fps"], 95)
        self.assertEqual(rows["FPS-01"]["minimum_fps"], 95)
        self.assertEqual(rows["FPS-01"]["change_percent"], -5)
        self.assertEqual(rows["FPS-02"]["median_fps"], 105)
        self.assertEqual(rows["FPS-02"]["maximum_fps"], 105)
        self.assertEqual(rows["FPS-02"]["change_percent"], 5)

        self.cases["FPS-01"] = running_case("FPS-01", (120, 94.99, 10))
        self.cases["FPS-02"] = running_case("FPS-02", (120, 105.01, 10))
        rows, passed = self.compare()
        self.assertFalse(passed)
        self.assertEqual([row["verdict"] for row in rows.values()],
                         ["failed", "failed", "passed", "passed"])
        self.assertAlmostEqual(rows["FPS-01"]["change_percent"], -5.01)
        self.assertAlmostEqual(rows["FPS-02"]["change_percent"], 5.01)
        self.assertIn("5.010% below reference", rows["FPS-01"]["reason"])
        self.assertIn("5.010% above reference", rows["FPS-02"]["reason"])
        self.assertEqual(self.path.read_bytes(), content)

    def test_missing_and_incompatible_references_keep_measured_rates_unverified(self):
        rows, passed = self.compare()
        self.assertFalse(passed)
        self.assertTrue(all(row["verdict"] == "unverified" for row in rows.values()))
        self.assertFalse(self.path.exists())
        del self.references["FPS-02"]
        self.references["FPS-04"]["compatibility"]["sampling"] = {
            "seconds": 10, "speed": 1, "metric": "drawing-submissions",
        }
        content = self.save_reference()
        rows, passed = self.compare()
        self.assertFalse(passed)
        self.assertEqual([row["verdict"] for row in rows.values()],
                         ["passed", "unverified", "passed", "unverified"])
        self.assertEqual(rows["FPS-02"]["median_fps"], 100)
        self.assertEqual(rows["FPS-04"]["reference_fps"], 100)
        self.assertIsNone(rows["FPS-04"]["change_percent"])
        self.assertEqual(rows["FPS-04"]["reason"], "Reference settings are incompatible")
        self.assertEqual(self.path.read_bytes(), content)

    def test_old_schemas_are_incompatible_and_saved_measurements_can_be_compared_again(self):
        self.save_reference(schema=1)
        measured, passed = self.compare()
        self.assertFalse(passed)
        self.assertTrue(all(row["reason"] == "Reference schema is incompatible"
                            for row in measured.values()))
        with self.assertRaisesRegex(ValueError, "Result schema is incompatible"):
            compare_all({"schema": 1, "cases": measured}, self.path)

        content = self.save_reference()
        saved = {"schema": SCHEMA, "cases": measured, "passed": False}
        compared, passed = compare_all(saved, self.path)
        self.assertTrue(passed)
        for row in compared.values():
            self.assertEqual(row["change_percent"], 0)
            self.assertIsNone(row["reason"])
        self.assertEqual(compare_all({**saved, "cases": compared}, self.path), (compared, True))
        self.assertEqual(self.path.read_bytes(), content)

    def test_any_game_sample_at_the_cap_guard_fails_before_reference_comparison(self):
        self.cases["FPS-01"] = running_case("FPS-01", (190, 100, 10))
        self.cases["FPS-03"] = running_case("FPS-03", (120, 121, 190))
        self.cases["FPS-04"] = running_case("FPS-04", (120, 121, 189.99))
        for reference_available in (False, True):
            with self.subTest(reference_available=reference_available):
                if reference_available:
                    for key in GAME_CASES:
                        self.references[key]["reference_fps"] = 121
                    self.save_reference()
                rows, passed = self.compare()
                self.assertFalse(passed)
                guarded = rows["FPS-03"]
                self.assertEqual(guarded["verdict"], "failed")
                self.assertEqual(guarded["median_fps"], 121)
                self.assertEqual(guarded["samples"][2]["fps"], 190)
                self.assertEqual(guarded["raw"][2]["sample"]["end_field"], 2900)
                self.assertIsNone(guarded["reference_fps"])
                self.assertIn("Warning: FPS-03 sample 3 reached 190.000 FPS", guarded["reason"])
                self.assertIn("raising the application's playback speed limit", guarded["reason"])
                for key in ("FPS-01", "FPS-04"):
                    self.assertEqual(rows[key]["verdict"], "passed" if reference_available else "unverified")

        self.cases["FPS-04"] = running_case("FPS-04", (120, 121, 190.1))
        rows, passed = self.compare()
        self.assertFalse(passed)
        self.assertEqual(rows["FPS-04"]["verdict"], "failed")
        self.assertIn("FPS-04 sample 3 reached 190.100 FPS", rows["FPS-04"]["reason"])

    def test_incomplete_game_case_retains_earlier_cap_warning_and_invalid_report(self):
        row = self.cases["FPS-03"] = running_case("FPS-03", (190,))
        invalid = {"error": "Game lost focus"}
        with self.assertRaisesRegex(RuntimeError, "Game lost focus") as failure:
            record_game_sample(row, invalid)
        row["error"] = str(failure.exception)
        self.cases["FPS-04"] = running_case("FPS-04", (100,))
        self.save_reference()
        rows, passed = self.compare()
        self.assertFalse(passed)
        for key in GAME_CASES:
            self.assertEqual(rows[key]["verdict"], "failed")
            self.assertIsNone(rows[key]["median_fps"])
        self.assertIn("FPS-03 sample 1 reached 190.000 FPS", rows["FPS-03"]["reason"])
        self.assertEqual(rows["FPS-03"]["error"], "Game lost focus")
        self.assertEqual(rows["FPS-03"]["raw"][1], invalid)
        self.assertEqual(rows["FPS-04"]["reason"], "No complete measurement")

    def test_three_valid_samples_and_matching_clock_count_are_required(self):
        with self.assertRaisesRegex(ValueError, "complete samples"):
            validate_samples([{"count": 120, "seconds": 1.0}])
        with self.assertRaisesRegex(ValueError, "Invalid sample"):
            validate_samples([{"count": 120, "seconds": 0.0}] * 3)
        work = {"emulated_ccks_per_series": 500, "guest_before": {"fields": 32},
                "guest_after": {"fields": 152}, "width": 320, "height": 256, "pc": 4096}
        verify_matching_work(work, work.copy())
        with self.assertRaisesRegex(ValueError, "emulated_ccks_per_series"):
            verify_matching_work(work, {**work, "emulated_ccks_per_series": 501})


    def test_game_references_require_same_rom_disk_and_boot_mode(self):
        rom = self.path.with_name("kickstart.rom")
        disk = self.path.with_name("scene.adf")
        rom_bytes = b"r" * (256 * 1024)
        disk_bytes = b"original disk fixture"
        rom.write_bytes(rom_bytes)
        disk.write_bytes(disk_bytes)

        with patch("run_mixed_scene_fps.ADF", disk):
            media = game_media_identity(rom)
            self.assertEqual(media, {
                "boot_mode": "manual-setup-df0",
                "rom_sha256": hashlib.sha256(rom_bytes).hexdigest(),
                "adf_sha256": hashlib.sha256(disk_bytes).hexdigest(),
            })
            for key in GAME_CASES:
                self.cases[key]["compatibility"]["workload"] = media
                self.references[key]["compatibility"]["workload"] = media.copy()
            content = self.save_reference()
            rows, passed = self.compare()
            self.assertTrue(passed)
            self.assertEqual(rows["FPS-03"]["verdict"], "passed")

            rom.write_bytes(b"s" * len(rom_bytes))
            self.cases["FPS-03"]["compatibility"]["workload"] = game_media_identity(rom)
            rom.write_bytes(rom_bytes)
            disk.write_bytes(b"changed disk fixture")
            self.cases["FPS-04"]["compatibility"]["workload"] = game_media_identity(rom)
            rows, passed = self.compare()
            self.assertFalse(passed)
            self.assertEqual([row["verdict"] for row in rows.values()],
                             ["passed", "passed", "unverified", "unverified"])
            self.assertIsNone(rows["FPS-03"]["change_percent"])
            self.assertIsNone(rows["FPS-04"]["change_percent"])
            self.assertEqual(self.path.read_bytes(), content)

        self.cases["FPS-03"]["compatibility"]["workload"] = {**media, "boot_mode": "raw-program"}
        rows, passed = self.compare()
        self.assertFalse(passed)
        self.assertEqual(rows["FPS-03"]["reason"], "Reference settings are incompatible")

    def test_missing_boot_media_keeps_game_rows_unverified_without_building(self):
        rom = self.path.with_name("private-kickstart.rom")
        disk = self.path.with_name("missing-scene.adf")
        for unavailable, reason in (
            (None, "--kickstart-rom"),
            (rom, "The private Kickstart ROM is unreadable"),
        ):
            with self.subTest(reason=reason), patch("run_mixed_scene_fps.build_settings") as build:
                for key in GAME_CASES:
                    row = self.cases[key] = {}
                    measure(row, key, {}, self.path.parent, SimpleNamespace(kickstart_rom=unavailable), {})
                    self.assertIn(reason, row["missing_input"])
                    self.assertNotIn(rom.name, row["missing_input"])
                build.assert_not_called()
                self.save_reference()
                rows, passed = self.compare()
                self.assertFalse(passed)
                self.assertEqual([row["verdict"] for row in rows.values()],
                                 ["passed", "passed", "unverified", "unverified"])
                self.assertIsNone(rows["FPS-03"]["median_fps"])
                self.assertIsNone(rows["FPS-04"]["median_fps"])

        rom.write_bytes(bytes(256 * 1024))
        with patch("run_mixed_scene_fps.ADF", disk):
            with self.assertRaisesRegex(FileNotFoundError, "ADF is unavailable"):
                game_media_identity(rom)

    def test_game_requires_advancing_boot_and_final_observations(self):
        report = game_run()
        first, confirmed, final = (report[key] for key in ("boot_first", "guest_before", "guest_after"))
        with self.assertRaisesRegex(ValueError, "Incomplete Game boot"):
            game_sample({**report, "boot_first": None})
        for key in ("cck", "field_number", "fields", "work_loops"):
            with self.subTest(stage="boot", key=key):
                with self.assertRaisesRegex(ValueError, "counters failed to advance"):
                    game_sample({**report, "guest_before": {**confirmed, key: first[key]}})
            with self.subTest(stage="final", key=key):
                with self.assertRaisesRegex(ValueError, "counters failed to advance"):
                    game_sample({**report, "guest_after": {**final, key: confirmed[key]}})
        with self.assertRaisesRegex(ValueError, "guest fault"):
            game_sample({**report, "boot_first": {**first, "fault": 1}})
        with self.assertRaisesRegex(RuntimeError, "Guest boot timed out"):
            game_sample({**report, "error": "Guest boot timed out"})

    def test_game_counts_all_fields_over_the_actual_elapsed_time(self):
        report = game_run()
        sample = game_sample(report)
        self.assertEqual(sample, {
            "warmup_start_field": 968, "start_field": 1000, "end_field": 1120,
            "count": 120, "seconds": 10.5, "speed": 4.0, "fps": 120 / 10.5,
        })
        stalled = game_sample(game_run(count=0, seconds=10))
        self.assertEqual(stalled["fps"], 0)

    def test_game_rejects_legacy_counts_and_invalid_timing_boundaries(self):
        report = game_run()
        for update, reason in (
            ({"start_field": None}, "boundary counts"),
            ({"end_field": 1 << 64}, "boundary counts"),
            ({"warmup_start_field": 967}, "outside guest observations"),
            ({"warmup_start_field": 969}, "32 completed Game fields"),
            ({"end_field": 999}, "boundaries regressed"),
            ({"end_field": 1131}, "outside guest observations"),
            ({"seconds": 9.999}, "at least 10 seconds"),
            ({"seconds": float("inf")}, "at least 10 seconds"),
            ({"speed": 2}, "at 4x speed"),
        ):
            with self.subTest(update=update), self.assertRaisesRegex(ValueError, reason):
                game_sample({**report, "sample": {**report["sample"], **update}})
        with self.assertRaisesRegex(ValueError, "boundary counts"):
            game_sample({**report, "sample": {
                "submissions": 498, "seconds": 10, "deadline_delay_seconds": 0.0003,
            }})

    def test_browser_receives_boot_inputs_via_private_page_routes(self):
        rom = self.path.with_name("private-kickstart.rom")
        disk = self.path.with_name("scene.adf")
        rom.write_bytes(b"test ROM")
        disk.write_bytes(b"test disk")
        page = Mock()
        url = "http://127.0.0.1:48100/"
        with patch("run_mixed_scene_fps.ADF", disk):
            install_browser_media(page, url, rom)
        handlers = dict(call.args for call in page.route.call_args_list)
        self.assertEqual(set(handlers), {url + "game-fps-input/rom", url + "game-fps-input/disk"})
        for label, expected in (("rom", b"test ROM"), ("disk", b"test disk")):
            route = Mock()
            handlers[url + "game-fps-input/" + label](route)
            route.fulfill.assert_called_once_with(body=expected, content_type="application/octet-stream",
                                                  headers={"cache-control": "no-store"})
        script = page.add_init_script.call_args.args[0]
        self.assertIn("window.amigaGameFpsMedia", script)
        self.assertIn("game-fps-input/rom", script)
        self.assertIn("game-fps-input/disk", script)
        self.assertNotIn(rom.name, script)
        self.assertNotIn("test ROM", script)

    def test_report_cannot_overwrite_private_rom(self):
        rom = self.path.with_name("private-kickstart.rom")
        rom.write_bytes(b"keep the original")
        with patch("sys.stderr"):
            with self.assertRaises(SystemExit) as error:
                main(["--kickstart-rom", str(rom), "--output", str(rom)])
        self.assertEqual(error.exception.code, 2)
        self.assertEqual(rom.read_bytes(), b"keep the original")
