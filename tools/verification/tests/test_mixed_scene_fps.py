"""The score uses the median and preserves the approved references."""

import contextlib
from copy import deepcopy
import hashlib
import io
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from textwrap import dedent
import time
import unittest
from unittest.mock import MagicMock, Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import mixed_scene_fps as fps
import verify
from worker_processes import WindowsJob
from mixed_scene_fps import (
    CASES,
    GAME_CASES,
    SCHEMA,
    compare_all,
    game_media_identity,
    game_sample,
    install_browser_media,
    measure,
    record_game_sample,
    validate_samples,
    verify_matching_work,
)

ROM_PATH = Path("tests/kickstarts/kickstart-1.3.rom")
ROM_BYTES = 262144


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
        "display": {"pixels_per_point": 1.0, "viewport_points": [800, 600]},
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


    def test_game_media_identity_records_boot_mode_and_adf_digest(self):
        disk = self.path.with_name("scene.adf")
        disk_bytes = b"original disk fixture"
        disk.write_bytes(disk_bytes)

        with patch.object(fps, "ADF", disk):
            media = game_media_identity()
            self.assertEqual(media, {
                "boot_mode": "manual-setup-df0",
                "adf_sha256": hashlib.sha256(disk_bytes).hexdigest(),
            })
            disk.write_bytes(b"changed disk fixture")
            self.assertNotEqual(game_media_identity()["adf_sha256"], media["adf_sha256"])

    def test_historical_game_refresh_and_rom_digest_are_ignored_for_comparison(self):
        approved_bytes = fps.REFERENCE.read_bytes()
        approved = json.loads(approved_bytes)
        self.assertEqual(approved["schema"], 2)
        self.references = approved["cases"]
        for key in GAME_CASES:
            compatibility = self.references[key]["compatibility"]
            compatibility["workload"]["rom_sha256"] = hashlib.sha256(b"historical ROM fixture").hexdigest()
            compatibility["display"]["display_refresh_hz"] = 120
        for key, reference in self.references.items():
            rate = reference["reference_fps"]
            row = running_case(key, (rate, rate, rate))
            row["compatibility"] = deepcopy(reference["compatibility"])
            if key in GAME_CASES:
                del row["compatibility"]["workload"]["rom_sha256"]
                del row["compatibility"]["display"]["display_refresh_hz"]
            self.cases[key] = row

        historical_bytes = self.save_reference()
        original_reference = deepcopy(self.references)
        rows, passed = self.compare()
        self.assertTrue(passed)
        self.assertNotIn("rom_sha256", json.dumps(rows))
        self.assertNotIn("display_refresh_hz", json.dumps(rows))
        with patch.object(fps, "load_reference", return_value=(self.references, None)):
            self.assertEqual(self.compare(), (rows, True))
            self.assertEqual(self.compare(), (rows, True))
        self.assertEqual(self.references, original_reference)
        self.assertEqual(self.path.read_bytes(), historical_bytes)
        self.assertEqual(fps.REFERENCE.read_bytes(), approved_bytes)

        for key in GAME_CASES:
            del self.references[key]["compatibility"]["workload"]["rom_sha256"]
            del self.references[key]["compatibility"]["display"]["display_refresh_hz"]
        self.save_reference()
        self.assertEqual(self.compare(), (rows, True))

        # An earlier report can be compared again even if its refresh rate
        # differs from the reference or the reference no longer has one.
        for key in GAME_CASES:
            self.cases[key]["compatibility"]["display"]["display_refresh_hz"] = 60
        for references in (self.references, original_reference):
            self.references = references
            self.save_reference()
            compared, passed = self.compare()
            self.assertTrue(passed)
            for key in GAME_CASES:
                self.assertEqual(compared[key]["change_percent"], rows[key]["change_percent"])
        for key in GAME_CASES:
            del self.cases[key]["compatibility"]["display"]["display_refresh_hz"]

        for key, path, update in (
            ("FPS-03", ("workload", "adf_sha256"), "different ADF"),
            ("FPS-04", ("workload", "boot_mode"), "raw-program"),
            ("FPS-03", ("workload", "sha256"), "different program"),
            ("FPS-04", ("sampling", "speed"), 2),
            ("FPS-03", ("display", "renderer"), "different renderer"),
            ("FPS-04", ("display", "viewport_points"), [160, 120]),
            ("FPS-03", ("display", "pixels_per_point"), 1.75),
            ("FPS-04", ("display_refresh_hz",), 60),
            ("FPS-02", ("display",), {"display_refresh_hz": 60}),
            ("FPS-03", ("rom_sha256",), "digest at the wrong level"),
            ("FPS-02", ("workload", "rom_sha256"), "digest in an uncapped case"),
        ):
            with self.subTest(key=key, path=path):
                self.references = deepcopy(original_reference)
                compatibility = self.references[key]["compatibility"]
                target = compatibility[path[0]] if len(path) == 2 else compatibility
                target[path[-1]] = update
                content = self.save_reference()
                compared, passed = self.compare()
                self.assertFalse(passed)
                self.assertEqual(compared[key]["verdict"], "unverified")
                self.assertEqual(compared[key]["reason"], "Reference settings are incompatible")
                self.assertIsNone(compared[key]["change_percent"])
                self.assertTrue(all(row["verdict"] == "passed" for case, row in compared.items() if case != key))
                self.assertEqual(self.path.read_bytes(), content)

    def test_game_sessions_still_require_matching_renderer_viewport_and_scale(self):
        for field, updated in (("renderer", "another renderer"), ("viewport_points", [1024, 768]),
                               ("pixels_per_point", 2.0)):
            with self.subTest(field=field):
                row = {"compatibility": {}}
                record_game_sample(row, game_run())
                changed = game_run()
                dest = changed if field == "renderer" else changed["display"]
                dest[field] = updated
                with self.assertRaisesRegex(ValueError, "Game display settings differed between sessions"):
                    record_game_sample(row, changed)

    def test_missing_adf_keeps_game_rows_unverified_without_building(self):
        disk = self.path.with_name("missing-scene.adf")
        with patch.object(fps, "ADF", disk), patch.object(fps, "build_settings") as build:
            for key in GAME_CASES:
                row = self.cases[key] = {}
                measure(row, key, {}, self.path.parent, None, {})
                self.assertEqual(row["missing_input"], "The checked-in Mixed Scene ADF is unavailable.")
        build.assert_not_called()
        self.save_reference()
        rows, passed = self.compare()
        self.assertFalse(passed)
        self.assertEqual([row["verdict"] for row in rows.values()],
                         ["passed", "passed", "unverified", "unverified"])
        self.assertIsNone(rows["FPS-03"]["median_fps"])
        self.assertIsNone(rows["FPS-04"]["median_fps"])

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
        with patch.object(fps, "ADF", disk):
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

    def test_browser_game_records_renderer_and_display_without_sampling_refresh(self):
        sample = game_run()
        browser, page = Mock(), Mock()
        browser.new_page.return_value = page
        page.evaluate.side_effect = [False, json.dumps(sample), "GPU"]

        def load(url, **_):
            handlers = dict(call.args for call in page.on.call_args_list)
            handlers["worker"](Mock(url=url + "machine_worker.js"))

        page.goto.side_effect = load
        with patch.object(fps, "install_browser_media"):
            report = fps.browser_game("http://localhost/", browser, self.path)
        self.assertEqual(report, {**sample, "renderer": f"{sample['renderer']}: GPU"})
        page.bring_to_front.assert_called_once_with()
        page.wait_for_function.assert_called_once()
        self.assertIn("window.amigaGameFpsLostFocus", page.wait_for_function.call_args.args[0])
        self.assertIn("document.hidden", page.add_init_script.call_args.args[0])
        renderer_script = page.evaluate.call_args.args[0]
        self.assertIn("WEBGL_debug_renderer_info", renderer_script)
        self.assertNotIn("requestAnimationFrame", renderer_script)
        page.close.assert_called_once_with()

    def supply_rom(self, root=None, contents=bytes(ROM_BYTES)):
        path = (root if root is not None else self.path.parent) / ROM_PATH
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(contents)
        return path.resolve()

    def run_public_fps(self, settings, *options, root=None):
        root = root if root is not None else self.path.parent

        def launch(command, *, env, **_):
            # Run through the public verifier and its real output filter. Use
            # the same worker route in-process to supply repeatable measurements.
            self.assertEqual(command[1:3], ["tools/verification/verify.py", "--_mixed-scene-fps"])
            self.assertTrue(all(secret not in command for secret in settings.values()))
            with patch.dict(os.environ, env, clear=True), contextlib.redirect_stdout(io.StringIO()) as child:
                code = verify.main(command[2:])
            process = MagicMock()
            process.__enter__.return_value = process
            process.stdout = child.getvalue().splitlines(keepends=True)
            process.wait.return_value = code
            return process

        with (
            patch.dict(os.environ, settings, clear=True),
            patch.object(verify, "REPO_ROOT", root),
            patch("execution.REPO_ROOT", root),
            patch.object(fps, "REPO_ROOT", root),
            patch.object(fps, "TEMP_ROOT", root / ".agent/tmp/mixed-scene-fps"),
            patch.object(fps, "REFERENCE", self.path),
            patch.object(fps, "source_identity", return_value={"revision": "fixture"}),
            patch.object(fps, "host_identity", return_value={}),
            patch("timing_report.git", return_value=b"fixture"),
            patch.object(verify, "run_reported", side_effect=lambda _name, _argv, run, **_: run(None)),
            patch("execution.subprocess.Popen", side_effect=launch) as process,
            contextlib.redirect_stdout(io.StringIO()) as output,
        ):
            code = verify.main(["test", "fps", *options])
        process.assert_called_once()
        return code, output.getvalue()

    def test_public_verdict_uses_current_evidence_and_shows_each_case(self):
        self.save_reference()
        results = self.path.with_name("reports")
        report = results / "mixed-scene-fps/result.json"
        rom = self.supply_rom()
        browser = self.path.with_name("private browser.exe").resolve()
        settings = {"AMIGA_FPS_BROWSER_EXECUTABLE": str(browser),
                    "AMIGA_FPS_NATIVE_REFRESH_HZ": "invalid"}

        def fixture(row, case, host, directory, runtime, native):
            self.assertEqual(runtime.kickstart_rom, rom)
            self.assertEqual(runtime.browser_executable, browser)
            row.update(deepcopy(self.cases[case]))

        normal = self.cases["FPS-03"]
        for evidence, expected, reason in (
            (normal, "passed", None),
            (running_case("FPS-03", (190, 120, 121)), "failed", "FPS-03 sample 1 reached 190.000 FPS"),
            ({**normal, "compatibility": {"sampling": {"metric": "old"}}},
             "unverified", "Reference settings are incompatible"),
        ):
            with self.subTest(expected=expected, reason=reason):
                self.cases["FPS-03"] = evidence
                with patch.object(fps, "measure", side_effect=fixture) as measured:
                    code, console = self.run_public_fps(settings, "--results-dir", str(results))
                self.assertEqual([call.args[1] for call in measured.call_args_list], list(CASES))
                content = report.read_text(encoding="utf-8")
                result = json.loads(content)
                self.assertEqual(result["schema"], 2)
                self.assertEqual(list(result["cases"]), list(CASES))
                row = result["cases"]["FPS-03"]
                self.assertEqual(row["verdict"], expected)
                if reason is not None:
                    self.assertIn(reason, row["reason"])
                self.assertEqual(result["passed"], expected == "passed")
                self.assertEqual(code, int(not result["passed"]))
                self.assertIn(f"[INFO] Mixed Scene FPS report: {report}", console)
                self.assertNotIn(str(rom), console + content)
                self.assertNotIn(str(browser), console + content)
                for name in ("display_refresh_hz", "raf_median_ms", "AMIGA_FPS_NATIVE_REFRESH_HZ"):
                    self.assertNotIn(name, content + console)
                self.assertTrue(any((results / "verify").glob("*.log")))
                for key, row in result["cases"].items():
                    self.assertRegex(console, rf"(?m)^\[INFO\] {key} .* {row['verdict']}$")
                if expected == "failed":
                    self.assertIn("[WARN] Warning: FPS-03 sample 1 reached 190.000 FPS", console)

    def test_public_fps_rejects_missing_unreadable_and_wrong_size_rom_before_benchmarking(self):
        decoy = self.path.with_name("valid decoy.rom")
        decoy.write_bytes(bytes(ROM_BYTES))
        for input_kind, size, reason in (
            ("missing", None, "The file is missing or unreadable"),
            ("directory", None, "The file is missing or unreadable"),
            ("too small", 262143, "Found 262143 bytes"),
            ("too large", 262145, "Found 262145 bytes"),
        ):
            with (
                self.subTest(input_kind=input_kind),
                tempfile.TemporaryDirectory(dir=self.path.parent) as checkout,
                patch.object(fps, "run_measurement") as run,
                patch.object(fps, "command") as build,
                patch.object(fps, "chromium") as browser,
                patch.object(fps, "measure") as measured,
            ):
                root = Path(checkout).resolve()
                self.assertNotEqual(Path.cwd(), root)
                rom = root / ROM_PATH
                if input_kind == "directory":
                    rom.mkdir(parents=True)
                elif size is not None:
                    self.supply_rom(root, bytes(size))
                code, console = self.run_public_fps({"AMIGA_FPS_KICKSTART_ROM": str(decoy)}, root=root)
                self.assertEqual(code, 1)
                self.assertIn("tests/kickstarts/kickstart-1.3.rom", console)
                self.assertIn("262144-byte Kickstart 1.3 ROM", console)
                self.assertIn(reason, console)
                self.assertNotIn(str(rom), console)
                self.assertNotIn(str(decoy), console)
                self.assertFalse((root / ".agent/tmp/mixed-scene-fps/result.json").exists())
                run.assert_not_called()
                build.assert_not_called()
                browser.assert_not_called()
                measured.assert_not_called()

    def test_public_fps_accepts_distinct_rom_contents_of_exact_size(self):
        self.save_reference()
        for contents in (b"a" * ROM_BYTES, b"b" * ROM_BYTES):
            with (
                tempfile.TemporaryDirectory(dir=self.path.parent) as checkout,
                patch.object(fps, "measure") as measured,
                patch.object(fps.hashlib, "file_digest", side_effect=AssertionError("Preflight hashed input")),
            ):
                root = Path(checkout).resolve()
                rom = self.supply_rom(root, contents)

                def fixture(row, case, _host, _directory, runtime, _native):
                    self.assertEqual(runtime.kickstart_rom, rom)
                    row.update(deepcopy(self.cases[case]))

                measured.side_effect = fixture
                code, console = self.run_public_fps({}, root=root)
                self.assertEqual(code, 0, console)
                self.assertEqual([call.args[1] for call in measured.call_args_list], list(CASES))

    def test_public_runtime_settings_default_output_and_private_diagnostics(self):
        self.save_reference()
        root = self.path.parent
        report = root / ".agent/tmp/mixed-scene-fps/result.json"
        rom = self.supply_rom()
        browser = root / "private browser.exe"
        supplied = {"AMIGA_FPS_BROWSER_EXECUTABLE": str(browser)}

        def fixture(row, case, _host, _directory, runtime, _native):
            row.update(deepcopy(self.cases[case]))
            if case == "FPS-03":
                row["raw"][0]["error"] = f"Cannot open {runtime.kickstart_rom}"
            if case == "FPS-04":
                raise RuntimeError(f"Cannot start {runtime.browser_executable.as_posix()}")

        with patch.object(fps, "measure", side_effect=fixture):
            code, console = self.run_public_fps(supplied)
        self.assertEqual(code, 1)
        self.assertIn(f"[INFO] Mixed Scene FPS report: {report}", console)
        content = report.read_text(encoding="utf-8")
        result = json.loads(content)
        self.assertEqual(result["cases"]["FPS-03"]["raw"][0]["error"], "Cannot open [private path]")
        self.assertEqual(result["cases"]["FPS-04"]["reason"], "RuntimeError: Cannot start [private path]")
        for path in (rom, browser):
            self.assertNotIn(str(path), content + console)
            self.assertNotIn(path.as_posix(), content + console)

    def test_browser_defaults_to_playwright_chromium_when_unset(self):
        browser = self.path.with_name("private browser.exe").resolve()
        self.supply_rom()
        for supplied, expected in (({}, None), ({"AMIGA_FPS_BROWSER_EXECUTABLE": str(browser)}, str(browser))):
            with (
                self.subTest(supplied=supplied),
                patch.dict(os.environ, supplied, clear=True),
                patch.object(fps, "REPO_ROOT", self.path.parent),
            ):
                playwright = Mock()
                fps.chromium(playwright, fps.load_settings().browser_executable)
                playwright.chromium.launch.assert_called_once()
                self.assertEqual(playwright.chromium.launch.call_args.kwargs["executable_path"], expected)

    def test_native_game_requests_foreground_and_returns_the_application_report(self):
        root = self.path.parent
        rom = self.supply_rom()
        sample = game_run()

        def launch(_command, *, env, **_):
            Path(env["AMIGA_GAME_FPS_RESULT"]).write_text(json.dumps(sample), encoding="utf-8")
            process = MagicMock()
            process.__enter__.return_value = process
            process.poll.return_value = None
            process.wait.return_value = 0
            return process

        with (
            patch.object(fps, "REPO_ROOT", root),
            patch.object(fps.sys, "platform", "win32"),
            patch.object(fps.subprocess, "Popen", side_effect=launch),
            patch.object(fps, "windows_game_window", return_value=1),
            patch.object(fps, "request_windows_game_focus", return_value=False) as focus,
            contextlib.redirect_stdout(io.StringIO()) as console,
        ):
            report = fps.native_game("fixture", root / "Game result", rom)
        self.assertEqual(report, sample)
        focus.assert_called_once_with(1)
        self.assertIn("Bring the Amiga Studio Game window to the foreground", console.getvalue())

    def test_source_identity_excludes_only_the_current_untracked_command_log(self):
        root = self.path.parent.resolve()
        active = root / "active.log"
        other = root / "other.log"

        def observe():
            with (
                patch.dict(os.environ, {fps.VERIFIER_LOG_PATH: str(active)}),
                patch.object(fps, "REPO_ROOT", root),
                patch.object(fps, "git", side_effect=[b"fixture", b"", b"active.log\0other.log\0"]),
            ):
                return fps.source_identity()

        active.write_bytes(b"first line")
        other.write_bytes(b"first change")
        original = observe()
        active.write_bytes(b"more lines")
        self.assertEqual(observe(), original)
        other.write_bytes(b"another change")
        self.assertNotEqual(observe(), original)

    def test_report_cannot_overwrite_private_rom(self):
        contents = b"r" * ROM_BYTES
        rom = self.supply_rom(contents=contents)
        with (
            patch.dict(os.environ, {}, clear=True),
            patch.object(fps, "REPO_ROOT", self.path.parent),
            patch.object(fps, "measure") as measured,
        ):
            settings = fps.load_settings()
            with self.assertRaisesRegex(ValueError, "The report path must differ from the workload and boot media paths"):
                fps.run_measurement(rom, settings, fps.private_path_pattern(settings))
        measured.assert_not_called()
        self.assertEqual(rom.read_bytes(), contents)


@unittest.skipUnless(os.name == "nt", "Windows job ownership requires Windows")
class WindowsWebBuildTests(unittest.TestCase):
    def test_command_preserves_stdout_and_nonzero_diagnostics(self):
        script = ("import os, sys; os.write(1, b'out\\r\\n'); "
                  "os.write(2, b'err\\xff\\r\\n'); sys.exit(int(sys.argv[1]))")
        args = [sys.executable, "-c", script]
        self.assertEqual(fps.command([*args, "0"], windows_process_tree=True, timeout=10), "out\n")
        with self.assertRaises(RuntimeError) as failure:
            fps.command([*args, "23"], windows_process_tree=True, timeout=10)
        self.assertEqual(str(failure.exception), f"{sys.executable} exited 23\nerr\ufffd\n\nout")

    def test_job_assignment_failure_keeps_the_command_gated(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / "command.started"
            args = [sys.executable, "-c", "import pathlib, sys; pathlib.Path(sys.argv[1]).touch()",
                    str(marker)]
            with (
                patch.object(WindowsJob, "attach", side_effect=OSError("job assignment failed")),
                self.assertRaisesRegex(OSError, "job assignment failed"),
            ):
                fps.command(args, windows_process_tree=True, timeout=10)
            self.assertFalse(marker.exists())

    def test_web_build_timeout_ends_powershell_and_descendants_with_shared_output(self):
        import ctypes
        from ctypes import wintypes
        import psutil

        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel.WaitForSingleObject.restype = wintypes.DWORD
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]

        def process_still_running(pid):
            handle = kernel.OpenProcess(0x00100000, False, pid)  # SYNCHRONIZE
            if not handle:
                if ctypes.get_last_error() == 87:  # ERROR_INVALID_PARAMETER, the PID is gone.
                    return False
                raise ctypes.WinError()
            try:
                status = kernel.WaitForSingleObject(handle, 0)
                if status == 0xFFFFFFFF:  # WAIT_FAILED
                    raise ctypes.WinError()
                return status == 258  # WAIT_TIMEOUT
            finally:
                kernel.CloseHandle(handle)

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / "tools/web.ps1"
            descendant = root / "descendant.py"
            script.parent.mkdir()
            script.write_text(dedent("""
                $PID | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'powershell.pid')
                Write-Output 'PowerShell started'
                & $env:AMIGA_FPS_TEST_PYTHON (Join-Path $PSScriptRoot '../descendant.py') child
                exit $LASTEXITCODE
            """), encoding="utf-8")
            descendant.write_text(dedent("""
                import os
                from pathlib import Path
                import subprocess
                import sys
                import time

                role = sys.argv[1]
                Path(__file__).with_name(role + ".pid").write_text(str(os.getpid()), encoding="ascii")
                if role == "child":
                    grandchild = subprocess.Popen([sys.executable, __file__, "grandchild"])
                for _ in range(300):
                    print(f"{role} stdout", flush=True)
                    print(f"{role} stderr", file=sys.stderr, flush=True)
                    time.sleep(0.1)
                if role == "child":
                    grandchild.wait()
            """), encoding="utf-8")
            markers = (script.with_name("powershell.pid"), root / "child.pid", root / "grandchild.pid")

            def surviving_processes():
                expected_scripts = {os.path.normcase(path) for path in (script, descendant)}
                for marker in markers:
                    if marker.is_file():
                        pid = int(marker.read_text(encoding="ascii").strip())
                        if not process_still_running(pid):
                            continue
                        try:
                            process = psutil.Process(pid)
                            arguments = (os.path.normcase(os.path.normpath(arg)) for arg in process.cmdline())
                            if expected_scripts.intersection(arguments):
                                yield process
                        except psutil.NoSuchProcess:
                            pass

            unrelated = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
            try:
                timeout = 5
                started = time.monotonic()
                with (
                    patch.object(fps, "REPO_ROOT", root),
                    patch.object(fps, "BUILD_TIMEOUT", timeout),
                    patch.dict(os.environ, {"AMIGA_FPS_TEST_PYTHON": sys.executable}),
                    self.assertRaises(subprocess.TimeoutExpired) as expired,
                ):
                    fps.web_build(root / "dist")
                elapsed = time.monotonic() - started
                self.assertLess(elapsed, timeout + 5, f"Build returned after {elapsed:.3f} seconds")
                self.assertEqual(expired.exception.timeout, timeout)
                self.assertEqual(expired.exception.cmd[0], "pwsh")
                self.assertIn(b"PowerShell started", expired.exception.stdout)
                for role in ("child", "grandchild"):
                    self.assertIn(f"{role} stdout".encode(), expired.exception.stdout)
                    self.assertIn(f"{role} stderr".encode(), expired.exception.stderr)
                self.assertTrue(all(marker.is_file() for marker in markers))
                deadline = time.monotonic() + 2
                while survivors := list(surviving_processes()):
                    self.assertLess(time.monotonic(), deadline, f"Build processes still running: {survivors}")
                    time.sleep(0.02)
                self.assertIsNone(unrelated.poll())
            finally:
                unrelated.kill()
                unrelated.wait(timeout=5)
                for process in surviving_processes():
                    process.kill()
                    process.wait(timeout=5)
