"""Direct PDF configuration parsing without HTML validation."""

from pathlib import Path
import yaml

def load_config(path):
    return yaml.safe_load(Path(path).read_text(encoding="utf-8"))


def preparation_settings(config):
    """Resolve preparation defaults without rewriting older workspace configs."""
    defaults = load_config(Path(__file__).resolve().parents[1] / "config.yaml")
    rendering = {**defaults["rendering"], **config.get("rendering", {})}
    ocr = {**defaults["stages"]["01_text_layer"],
           **config.get("stages", {}).get("01_text_layer", {})}
    return {"render_dpi": rendering["dpi"], **ocr}
