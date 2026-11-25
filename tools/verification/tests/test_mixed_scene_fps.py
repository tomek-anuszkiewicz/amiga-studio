"""The score uses the median and preserves the approved references."""

import hashlib
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from run_mixed_scene_fps import (
    CASES,
    SCHEMA,
    compare_all,
    game_media_identity,
    game_sample,
    install_browser_media,
    main,
    measure,
    validate_samples,
    verify_matching_work,
)


def running_case(median):
    return {"compatibility": {"renderer": "fixture renderer"},
            "samples": validate_samples([{"count": count, "seconds": 100}
                                         for count in (12000, median, 1000)])}


class MixedSceneFpsTests(unittest.TestCase):
    def setUp(self):
        directory = self.enterContext(tempfile.TemporaryDirectory())
        self.path = Path(directory) / "reference.json"
        self.cases = {key: running_case(8750) for key in CASES}
        reference = {"reference_fps": 100, "tolerance": 0.125,
                     "compatibility": {"renderer": "fixture renderer"}}
        self.references = {key: reference.copy() for key in CASES}

    def save_reference(self):
        content = json.dumps({"schema": SCHEMA, "cases": self.references})
        self.path.write_text(content, encoding="utf-8")
        return content.encode()

    def test_lower_limit_is_inclusive_and_all_four_cases_must_pass(self):
        content = self.save_reference()
        rows, passed = compare_all(self.cases, self.path)
        self.assertTrue(passed)
        self.assertEqual(len(rows), 4)
        self.assertEqual(rows["FPS-01"]["median_fps"], 87.5)
        self.assertEqual(rows["FPS-01"]["minimum_fps"], 87.5)
        self.assertEqual(rows["FPS-01"]["change_percent"], -12.5)

        self.cases["FPS-02"] = running_case(8749)
        self.cases["FPS-03"] = running_case(11000)
        rows, passed = compare_all(self.cases, self.path)
        self.assertFalse(passed)
        self.assertEqual([row["verdict"] for row in rows.values()],
                         ["passed", "failed", "passed", "passed"])
        self.assertEqual(rows["FPS-02"]["median_fps"], 87.49)
        self.assertAlmostEqual(rows["FPS-03"]["change_percent"], 10)
        self.assertEqual(self.path.read_bytes(), content)

    def test_missing_and_incompatible_references_keep_measured_rates_unverified(self):
        rows, passed = compare_all(self.cases, self.path)
        self.assertFalse(passed)
        self.assertTrue(all(row["verdict"] == "unverified" for row in rows.values()))
        self.assertFalse(self.path.exists())
        del self.references["FPS-02"]
        self.references["FPS-04"]["compatibility"] = {"renderer": "other renderer"}
        content = self.save_reference()
        rows, passed = compare_all(self.cases, self.path)
        self.assertFalse(passed)
        self.assertEqual([row["verdict"] for row in rows.values()],
                         ["passed", "unverified", "passed", "unverified"])
        self.assertEqual(rows["FPS-02"]["median_fps"], 87.5)
        self.assertEqual(rows["FPS-04"]["reference_fps"], 100)
        self.assertIsNone(rows["FPS-04"]["change_percent"])
        self.assertEqual(self.path.read_bytes(), content)

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
            for key in ("FPS-03", "FPS-04"):
                self.cases[key]["compatibility"] = {"workload": media}
                self.references[key]["compatibility"] = {"workload": media.copy()}
            content = self.save_reference()
            rows, passed = compare_all(self.cases, self.path)
            self.assertTrue(passed)
            self.assertEqual(rows["FPS-03"]["verdict"], "passed")

            rom.write_bytes(b"s" * len(rom_bytes))
            self.cases["FPS-03"]["compatibility"] = {"workload": game_media_identity(rom)}
            rom.write_bytes(rom_bytes)
            disk.write_bytes(b"changed disk fixture")
            self.cases["FPS-04"]["compatibility"] = {"workload": game_media_identity(rom)}
            rows, passed = compare_all(self.cases, self.path)
            self.assertFalse(passed)
            self.assertEqual([row["verdict"] for row in rows.values()],
                             ["passed", "passed", "unverified", "unverified"])
            self.assertIsNone(rows["FPS-03"]["change_percent"])
            self.assertIsNone(rows["FPS-04"]["change_percent"])
            self.assertEqual(self.path.read_bytes(), content)

        self.cases["FPS-03"]["compatibility"] = {"workload": {**media, "boot_mode": "raw-program"}}
        rows, passed = compare_all(self.cases, self.path)
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
                for key in ("FPS-03", "FPS-04"):
                    row = self.cases[key] = {}
                    measure(row, key, {}, self.path.parent, SimpleNamespace(kickstart_rom=unavailable), {})
                    self.assertIn(reason, row["missing_input"])
                    self.assertNotIn(rom.name, row["missing_input"])
                build.assert_not_called()
                self.save_reference()
                rows, passed = compare_all(self.cases, self.path)
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
        first = {"cck": 100, "field_number": 3, "fields": 1, "work_loops": 10, "fault": 0}
        confirmed = {"cck": 150, "field_number": 4, "fields": 2, "work_loops": 20, "fault": 0}
        final = {"cck": 500, "field_number": 6, "fields": 4, "work_loops": 40, "fault": 0}
        report = {"generation": 2, "sample": {"submissions": 498, "seconds": 10,
                                              "deadline_delay_seconds": 0.0003},
                  "guest_before": confirmed, "guest_after": final}
        with self.assertRaisesRegex(ValueError, "Incomplete Game boot"):
            game_sample(report)
        report["boot_first"] = first
        self.assertEqual(game_sample(report), {"count": 498, "seconds": 10,
                                               "deadline_delay_seconds": 0.0003})
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
