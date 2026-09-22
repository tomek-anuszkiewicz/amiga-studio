"""Contract tests for the standalone Reference image-description bootstrap tool."""

import importlib.util
import tempfile
import unittest
from pathlib import Path


TOOL_PATH = Path(__file__).resolve().parents[1] / "describe_reference_images.py"
SPEC = importlib.util.spec_from_file_location("describe_reference_images", TOOL_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class DescribeReferenceImagesTests(unittest.TestCase):
    def test_creates_only_missing_sidecars_for_reference_images(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            reference_root = Path(temporary_directory) / "Reference"
            assets = reference_root / "Manual" / "assets"
            assets.mkdir(parents=True)
            image = assets / "dma.png"
            image.write_bytes(b"image")
            (reference_root / "Manual" / "chapter.md").write_text("![DMA diagram](assets/dma.png)", encoding="utf-8")

            self.assertEqual(MODULE.create_missing_descriptions(reference_root), 1)
            sidecar = assets / "dma.png.txt"
            self.assertIn("DMA diagram", sidecar.read_text(encoding="utf-8"))

            sidecar.write_text("curated", encoding="utf-8")
            self.assertEqual(MODULE.create_missing_descriptions(reference_root), 0)
            self.assertEqual(sidecar.read_text(encoding="utf-8"), "curated")

    def test_skips_obsidian_assets_and_honors_dry_run(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            reference_root = Path(temporary_directory) / "Reference"
            ignored = reference_root / ".obsidian" / "ignored.png"
            target = reference_root / "Manual" / "asset.svg"
            ignored.parent.mkdir(parents=True)
            target.parent.mkdir(parents=True)
            ignored.write_bytes(b"image")
            target.write_bytes(b"image")

            self.assertEqual(MODULE.create_missing_descriptions(reference_root, dry_run=True), 1)
            self.assertFalse((target.parent / "asset.svg.txt").exists())
