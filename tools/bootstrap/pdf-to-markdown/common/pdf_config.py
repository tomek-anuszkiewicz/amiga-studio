"""Direct PDF configuration parsing without HTML validation."""

from pathlib import Path
import yaml

OCR_SETTINGS = {
    "language": "eng",
    "colorspace": "RGB",
    "alpha": False,
    "ocr_compress": True,
    "save_deflate": True,
    "save_garbage": 4,
}

def load_config(path):
    return yaml.safe_load(Path(path).read_text(encoding="utf-8"))


def preparation_settings(config):
    """Resolve the current preparation settings and configuration defaults."""
    defaults = load_config(Path(__file__).resolve().parents[1] / "config.yaml")
    rendering = {**defaults["rendering"], **config.get("rendering", {})}
    ocr = {**defaults["stages"]["01_text_layer"],
           **config.get("stages", {}).get("01_text_layer", {})}
    return {"render_dpi": rendering["dpi"],
            "traineddata_sha256": ocr["traineddata_sha256"],
            "traineddata_url": ocr["traineddata_url"], **OCR_SETTINGS}
